//! SDO Client
//!
use crate::extract_length;
use crate::CanOpenFrameBuilder;
use crate::CommandSpecifier;
use crate::{CANOpenFrame, FrameType, Payload, SDOAbortCode};

use super::CanOpenError;
use core::time::Duration;
use futures_timer::Delay;
use futures_util::StreamExt;
use futures_util::{pin_mut, select, FutureExt};
use log::debug;
use tokio_socketcan::CANSocket; // for reading next  from can socket

pub struct SdoClient {
    node_id: u8,
    can_socket: CANSocket,
}

impl SdoClient {
    /// Create a new SdoClient that allows to access a remote SdoServer
    ///
    /// A client can always be used to address one server identified by
    /// the node id of the server.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node ID of the CANOpen server to address the requests to.
    ///
    /// # Panics
    ///
    /// Panics if the `node_id` is not in range of `0..0x7F`.
    ///
    pub fn new(node_id: u8, can_socket: CANSocket) -> Self {
        if node_id > 0x7F {
            panic!("node_id is out of allowed range [0..0x7F] {:?}", node_id);
        }
        SdoClient {
            node_id,
            can_socket,
        }
    }

    /// Read a data object from a sdo servicer a.k.a. upload
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the object to access on the server side
    /// * `subindex` - The index of the object to access on the server side
    /// * `data`- Buffer where the ok - result is transferred into
    ///
    /// # Returns
    ///
    /// * Number of bytes written into the result buffer
    ///
    /// # Errors
    ///
    /// - SDO timeout (in case any response takes longer than the maximum answer
    ///   time or does not arrive at all.
    /// - SDO Abort code - details reveal
    ///
    /// # Example
    ///
    /// ```
    /// use col::sdo_client::SdoClient;
    /// use tokio_socketcan::CANSocket;
    ///
    /// let my_future = async {
    ///
    ///     let mut can_socket = CANSocket::open("can0").unwrap();
    ///     let node_id_of_sdo_server = 0x20_u8;
    ///     let mut sdo_client = SdoClient::new(node_id_of_sdo_server, can_socket);
    ///     let index = 0x2000_u16;
    ///     let subindex = 0x01_u8;
    ///     let mut data = [0_u8; 4];
    ///
    ///     match sdo_client.read_object(index, subindex, &mut data).await {
    ///         Ok(len) =>  {
    ///             println!("Object 0x2000,0x01@0x20 value {:?}", &data[0..len]);
    ///         }
    ///         Err(error) => {
    ///             println!("Error {}", error);
    ///         }
    ///     }
    /// };
    /// ```
    pub async fn read_object(
        &mut self,
        index: u16,
        subindex: u8,
        data: &mut [u8],
    ) -> Result<usize, CanOpenError> {
        let mut len = self.read_indexed(index, subindex, data).await?;
        if len > 4 {
            debug!("Read segmented {} bytes", len);
            let mut bytes_already_read = 0_usize;
            let data_buffer_len = data.len();
            let mut toggle = false;
            while bytes_already_read < len {
                debug!("Bytes already read: {}", bytes_already_read);
                bytes_already_read += self
                    .read_segment(
                        index,
                        subindex,
                        toggle,
                        &mut data[bytes_already_read..data_buffer_len],
                    )
                    .await?;
                toggle = !toggle;
            }
            debug!("Bytes totally read: {}", bytes_already_read);
        }
        Ok(len)
    }

    pub async fn read_indexed(
        &mut self,
        index: u16,
        subindex: u8,
        data: &mut [u8],
    ) -> Result<usize, CanOpenError> {
        debug!("Send upload request");
        let worker = async {
            let builder = CanOpenFrameBuilder::sdo_request(self.node_id)
                .unwrap()
                .with_index(index, subindex)
                .upload_request();
            let frame = builder.build().into();
            self.can_socket
                .write_frame(frame)
                .map_err(|_| -> CanOpenError { CanOpenError::SocketInstanciatingError })?
                .await
                .map_err(|_| -> CanOpenError { CanOpenError::SocketWriteError })?;

            debug!("Await upload response");
            let mut len: usize = 0;
            while let Some(Ok(frame)) = self.can_socket.next().await {
                let frame = CANOpenFrame::try_from(frame)?;
                if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoTx {
                    if let Payload::SdoWithIndex(payload) = frame.payload {
                        if payload.index == index && payload.subindex == subindex {
                            if payload.cs
                                == CommandSpecifier::Scs(crate::ServerCommandSpecifier::Abort)
                            {
                                return Err(CanOpenError::SdoAbortCode {
                                    abort_code: SDOAbortCode::from(payload.data),
                                });
                            }
                            if payload.expedited_flag {
                                // Expedited response
                                len = extract_length(payload.size);
                                for (i, item) in data.iter_mut().enumerate() {
                                    if i < len {
                                        *item = (payload.data >> (8 * i) & 0xff) as u8;
                                    }
                                }
                            } else {
                                // Data bigger than 4 byte -> segmented response is required
                                len = payload.data as usize;
                                if len > data.len() {
                                    return Err(CanOpenError::StringIsTooLong {
                                        max_length: data.len(),
                                        given_length: len,
                                    });
                                }
                            }

                            break;
                        }
                    }
                }
            }

            Ok(len)
        }
        .fuse();
        let timeout = client_server_communication_timeout().fuse();

        pin_mut!(worker, timeout);

        let result: Result<usize, CanOpenError>;

        select! {
            worker_result = worker => result = worker_result,
            () = timeout =>  {
                debug!("Timeout reached");
                result = Err(CanOpenError::SdoProtocolTimedOut);
            }
        }
        result
    }

    async fn read_segment(
        &mut self,
        index: u16, // only to maintain the session and potentially catch SDO Abort
        subindex: u8,
        toggle: bool,
        data: &mut [u8],
    ) -> Result<usize, CanOpenError> {
        debug!("Send segment upload request");
        let worker = async {
            let mut builder = CanOpenFrameBuilder::sdo_request(self.node_id)
                .unwrap()
                .without_index()
                .upload_request()
                .toggle(toggle);
            let frame = builder.build().into();
            self.can_socket
                .write_frame(frame)
                .map_err(|_| -> CanOpenError { CanOpenError::SocketInstanciatingError })?
                .await
                .map_err(|_| -> CanOpenError { CanOpenError::SocketWriteError })?;

            debug!("Await segment upload reponse");
            let mut len: usize = 0;
            while let Some(Ok(frame)) = self.can_socket.next().await {
                let frame = CANOpenFrame::try_from(frame)?;
                if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoTx {
                    if let Payload::SdoWithIndex(payload) = frame.payload {
                        if payload.index == index && payload.subindex == subindex {
                            if payload.cs
                                == CommandSpecifier::Scs(crate::ServerCommandSpecifier::Abort)
                            {
                                return Err(CanOpenError::SdoAbortCode {
                                    abort_code: SDOAbortCode::from(payload.data),
                                });
                            }
                            break;
                        }
                    } else if let Payload::SdoWithoutIndex(payload) = frame.payload {
                        // copy the data over
                        len = 7_usize
                            - match payload.length_of_empty_bytes {
                                Some(x) => x as usize,
                                None => 0_usize,
                            };
                        debug!("Segment extract {} bytes", len);
                        for i in 0..len {
                            data[i] = payload.data[i];
                        }
                        break;
                    }
                }
            }

            Ok(len)
        }
        .fuse();

        let timeout = client_server_communication_timeout().fuse();

        pin_mut!(worker, timeout);

        let result: Result<usize, CanOpenError>;

        select! {
            worker_result = worker => result = worker_result,
            () = timeout =>  {
                debug!("Timeout reached");
                result = Err(CanOpenError::SdoProtocolTimedOut);
            }
        }
        result
    }

    /// Write a data object at a sdo server a.k.a download
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the object to access on the server side
    /// * `subindex` - The index of the object to access on the server side
    /// * `data`- slice that contains the data to write
    ///
    /// # Errors
    ///
    /// - SDO timeout (in case any response takes longer than the maximum answer
    ///   time or does not arrive at all.
    /// - SDO Abort code
    ///
    /// # Example
    ///
    /// ```
    /// use col::sdo_client::SdoClient;
    /// use tokio_socketcan::CANSocket;
    ///
    /// let my_future = async {
    ///
    ///     let mut can_socket = CANSocket::open("can0").unwrap();
    ///     let node_id_of_sdo_server = 0x20_u8;
    ///     let mut sdo_client = SdoClient::new(node_id_of_sdo_server, can_socket);
    ///     let index = 0x2000_u16;
    ///     let subindex = 0x01_u8;
    ///     let data = &[0x01, 0x02, 0x03, 0x04];
    ///
    ///     match sdo_client.write_object(index, subindex, data).await {
    ///         Ok(()) =>  {
    ///             println!("Set object 0x2000,0x01@0x20 to value {:?}", data);
    ///         }
    ///         Err(error) => {
    ///             println!("Error {}", error);
    ///         }
    ///     }
    /// };
    /// ```
    pub async fn write_object(
        &mut self,
        index: u16,
        subindex: u8,
        data: &[u8],
    ) -> Result<(), CanOpenError> {
        let worker = async {
            let builder = CanOpenFrameBuilder::sdo_request(self.node_id)
                .unwrap()
                .with_index(index, subindex)
                .download(data);
            let frame = builder.build().into();
            self.can_socket
                .write_frame(frame)
                .map_err(|_| -> CanOpenError { CanOpenError::SocketInstanciatingError })?
                .await
                .map_err(|_| -> CanOpenError { CanOpenError::SocketWriteError })?;

            // wait for the matching response
            while let Some(Ok(frame)) = self.can_socket.next().await {
                let frame = CANOpenFrame::try_from(frame)?;
                if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoTx {
                    if let Payload::SdoWithIndex(payload) = frame.payload {
                        if payload.index == index && payload.subindex == subindex {
                            break;
                        }
                    }
                }
            }

            Ok(())
        }
        .fuse();
        let timeout = client_server_communication_timeout().fuse();

        pin_mut!(worker, timeout);

        let result: Result<(), CanOpenError>;
        select! {
            worker_result = worker => result = worker_result,
            () = timeout =>  result = Err(CanOpenError::SdoProtocolTimedOut),
        }
        result
    }
}

async fn client_server_communication_timeout() {
    const SDO_COMMUNICATION_TIMEOUT_IN_MS: u64 = 500;
    debug!(
        "Set response timeout to {} milliseconds",
        SDO_COMMUNICATION_TIMEOUT_IN_MS
    );
    let _timeout = Delay::new(Duration::from_millis(SDO_COMMUNICATION_TIMEOUT_IN_MS)).await;
}
