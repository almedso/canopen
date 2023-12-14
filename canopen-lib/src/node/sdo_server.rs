//! SDO Server:
//!
//! Read a data object from a sdo servicer a.k.a. upload
//!
//! # Arguments
//!
//! * `index` - The index of the object to access on the server side
//! * `subindex` - The index of the object to access on the server side
//! * `data`- Buffer where the ok - result is transferred into
//!
//! # Returns
//!
//! * Number of bytes written into the result buffer
//!
//! # Errors
//!
//! - SDO timeout (in case any response takes longer than the maximum answer
//!   time or does not arrive at all.
//! - SDO Abort code - details reveal
//!
//! # Example
//!
//! ```
//! use col::node::{sdo_server::SdoServer, object_dictionary::ObjectDictionaryBuilder};
//! use col::CANOpenFrame;
//! use tokio_socketcan::CANSocket;
//! use core::str::from_utf8;
//!
//! let my_future = async {
//!
//!     let mut can_socket = CANSocket::open("can0").unwrap();
//!     let node_id_of_sdo_server = 0x20_u8;
//!     let device_type = 0x_ffff_0000_u32;  // LSB part is profile number e.g. 402; MSB is additional information
//!     let vendor_id = 0_u32; // need to be registered/purchased at CANOpen authority
//!     let od = ObjectDictionaryBuilder::new(device_type, vendor_id)
//!         .device_name("Device Name")
//!         .hardware_version("Rev 1.0")
//!         .software_version("1.0.0")
//!         .product_identifier(1_u32)  // up to the vendor to decide
//!         .product_revision(1_u32) // up to the vendor to decide
//!         .serial_number(123456_u32)
//!         .build(node_id_of_sdo_server);
//!     let mut sdo_server= SdoServer::new(node_id_of_sdo_server, can_socket, od.into());
//!
//!     // run CANOpen frame processing in a loop at infinitum
//!     while let frame = sdo_server.next_sdo_frame().await {
//!         process_complete_sdo_request(frame).await.unwrap_or_default();
//!     }
//! };
//!
//! ```
use crate::object_dictionary::ObjectDictionary;
use crate::CanOpenError;
use crate::CanOpenFrameBuilder;
use crate::CommandDataSize;
use crate::CommandSpecifier;
use crate::ValueVariant;
use crate::WithIndexPayload;
use crate::{CANOpenFrame, FrameType, Payload, SDOAbortCode};
use std::rc::Rc;

// std/no-std dependent dependent
use futures_util::stream::StreamExt;
use log::debug;
use tokio_socketcan::CANSocket; // for reading next  from can socket

#[derive(Default, PartialEq)]
pub struct SdoSession {
    object_index: u16,
    object_subindex: u8,
    data_index: u32,
    data_size: u32,
    segment_number: u32,
    toggle_flag: bool,
    acknowledge_frame: u32, // at blockdownload how often an acknowledge frame the server needs to send to the client
}

pub struct SdoServer<'a> {
    node_id: u8,
    can_socket: CANSocket,
    object_dictionary: Rc<ObjectDictionary<'a>>,
    session: SdoSession,
}

struct IndexedPayloadError;

fn cast_indexed_payload_to_value_variant<'a>(
    object_value: ValueVariant<'_>,
    payload: &'a WithIndexPayload,
) -> Result<ValueVariant<'a>, IndexedPayloadError> {
    match object_value {
        ValueVariant::F32(_) => {
            if payload.size == CommandDataSize::FourBytes {
                Ok(ValueVariant::F32(payload.data as f32))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::U32(_) => {
            if payload.size == CommandDataSize::FourBytes {
                Ok(ValueVariant::U32(payload.data))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::U16(_) => {
            if payload.size == CommandDataSize::TwoBytes {
                Ok(ValueVariant::U16(payload.data as u16))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::U8(_) => {
            if payload.size == CommandDataSize::OneByte {
                Ok(ValueVariant::U8(payload.data as u8))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::I32(_) => {
            if payload.size == CommandDataSize::FourBytes {
                Ok(ValueVariant::I32(payload.data as i32))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::I16(_) => {
            if payload.size == CommandDataSize::TwoBytes {
                Ok(ValueVariant::I16(payload.data as i16))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::I8(_) => {
            if payload.size == CommandDataSize::OneByte {
                Ok(ValueVariant::I8(payload.data as i8))
            } else {
                Err(IndexedPayloadError)
            }
        }
        ValueVariant::S(_) => Err(IndexedPayloadError),
    }
}

/// A CANOpen server
///
/// It is a single session server one session is handled at a time.
///
/// SDO frames that interfere the session are responded with an error frame
///
/// # Example
///
/// ```
/// ```
impl<'a> SdoServer<'a> {
    /// Create a new canopen server
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node ID of the CANOpen server to address the requests to.
    /// * `can_socket` - Socket that represents the can bus
    /// * `object_dictionary` - The object dictionary this server is serving
    ///
    /// # Panics
    ///
    /// Panics if the `node_id` is not in range of `0..0x7F`.
    ///
    pub fn new(
        node_id: u8,
        can_socket: CANSocket,
        object_dictionary: Rc<ObjectDictionary<'a>>,
    ) -> Self {
        if node_id > 0x7F {
            panic!("node_id is out of allowed range [0..0x7F] {:?}", node_id);
        }
        SdoServer {
            node_id,
            can_socket,
            object_dictionary,
            session: Default::default(),
        }
    }

    /// Pick sdo frames addressed to this node  from the stream of CAN frames
    ///
    /// Other frames are ignored
    pub async fn next_sdo_frame(&mut self) -> CANOpenFrame {
        loop {
            if let Some(Ok(frame)) = self.can_socket.next().await {
                if let Ok(canopen_frame) = CANOpenFrame::try_from(frame) {
                    if canopen_frame.node_id() == self.node_id
                        && (canopen_frame.frame_type() == FrameType::SdoTx
                            || canopen_frame.frame_type() == FrameType::SdoRx)
                    {
                        return canopen_frame;
                    }
                }
            }
        }
    }

    /// Process a complete SDO request
    ///
    /// if it is an expedited request it is
    /// - receive a request
    /// - send a single response.
    /// If it is a none expedited request it is a sequence of
    /// - request
    /// - 1st response
    /// - next request
    /// - next response
    /// - ... request
    /// - ... response
    /// - last request
    /// - last response
    ///
    /// # Return
    ///
    /// - `()` - if fhe frame is no SDO request
    /// - `()` - if the response to the last frame request was not an error
    /// - `CanOpenError::SdoTransferInterrupted` - if another SDO request destroyed the sdo transfer
    ///    session
    /// - `CanOpenError::SdoTransferTimeout` - if the Request was aborted due to a missing request
    /// - `CanOpenError::ObjectDoesNotExist`
    pub async fn process_frame(&self, frame: &'a CANOpenFrame) -> Result<(), CanOpenError> {
        if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoRx {
            let response = self.process_frame_with_index(frame);
            self.process_frame_without_index(frame);

            if let Some(response_frame_builder) = response {
                debug!("Send SDO response communication object");
                self.can_socket
                    .write_frame(response_frame_builder.into())
                    .map_err(|_| -> CanOpenError { CanOpenError::SocketInstanciatingError })?
                    .await
                    .map_err(|_| -> CanOpenError { CanOpenError::SocketWriteError })?;
            }
        }
        Ok(())
    }

    fn process_frame_without_index(&self, frame: &CANOpenFrame) {
        if let Payload::SdoWithoutIndex(payload) = &frame.payload {
            // no index and subindex; Data bigger than 4 byte -> segmented response is required
            if self.session == Default::default() {
                // A session must be setup at this point already
                // since there is no index,subindex known, an error response cannot be sent
            } else {
                match payload.cs {
                    CommandSpecifier::Ccs(crate::ClientCommandSpecifier::DownloadSegment) => {}
                    CommandSpecifier::Ccs(_) => {}
                    CommandSpecifier::Scs(_) => {}
                }
            }
        }
    }

    fn process_frame_with_index(&self, frame: &'a CANOpenFrame) -> Option<CANOpenFrame> {
        if let Payload::SdoWithIndex(payload) = &frame.payload {
            let index = payload.index;
            let subindex = payload.subindex;
            let response_frame = match payload.cs {
                CommandSpecifier::Ccs(crate::ClientCommandSpecifier::Download) => {
                    // download expedited request - aka write
                    if let Ok(object) = self.object_dictionary.get_object_value(index, subindex) {
                        if let Ok(value) = cast_indexed_payload_to_value_variant(object, &payload) {
                            match self.object_dictionary
                                .download_expedited(index, subindex, value)
                                .map_err(|error| match error {
                                    CanOpenError::CannotWriteToConstStorage
                                    | CanOpenError::WritingForbidden => {
                                        SDOAbortCode::WriteReadOnlyError
                                    }
                                    _ => SDOAbortCode::DictionaryError,
                                }) {
                                Ok(()) => CanOpenFrameBuilder::sdo_response(self.node_id)
                                    .unwrap()
                                    .with_index(index, subindex)
                                    .download_response()
                                    .build(),
                                Err(abort_code) => CanOpenFrameBuilder::sdo_response(self.node_id)
                                    .unwrap()
                                    .with_index(index, subindex)
                                    .abort(abort_code)
                                    .build(),
                            }
                        } else {
                            CanOpenFrameBuilder::sdo_response(self.node_id)
                                .unwrap()
                                .with_index(index, subindex)
                                .abort(SDOAbortCode::WrongLength)
                                .build()
                        }
                    } else {
                        CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .abort(SDOAbortCode::ObjectDoesNotExist)
                            .build()
                    }
                }
                CommandSpecifier::Ccs(crate::ClientCommandSpecifier::UploadSegment) => {
                    // todo
                    CanOpenFrameBuilder::sdo_response(self.node_id)
                        .unwrap()
                        .with_index(index, subindex)
                        .abort(SDOAbortCode::UnsupportedAccess)
                        .build()
                }
                CommandSpecifier::Ccs(crate::ClientCommandSpecifier::Upload) => {
                    match  self.object_dictionary.upload(index, subindex).map_err(|error| match error {
                        CanOpenError::ObjectDoesNotExist { index: _, subindex: _ } =>  SDOAbortCode::ObjectDoesNotExist,
                        CanOpenError::ReadAccessImpossible => SDOAbortCode::WriteReadOnlyError,
                        _ => SDOAbortCode::DictionaryError,
                    }) {
                        Ok(object_value) =>  match object_value {
                            ValueVariant::S(v) => self.process_segmented(index, subindex, v),
                            _ => self.send_expedited_object_value(index, subindex, object_value),
                            }
                        Err(abort_code) => CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .abort(abort_code)
                            .build(),
                    }
                }
                CommandSpecifier::Ccs(crate::ClientCommandSpecifier::BlockDownload) => {
                    // will be implemented later
                    CanOpenFrameBuilder::sdo_response(self.node_id)
                        .unwrap()
                        .with_index(index, subindex)
                        .abort(SDOAbortCode::UnsupportedAccess)
                        .build()
                }
                CommandSpecifier::Ccs(crate::ClientCommandSpecifier::Unspecified)
                    // should not happen at all
                | CommandSpecifier::Ccs(crate::ClientCommandSpecifier::BlockUpload)
                    // block upload aka read large chunks from the server --> send abort code
                    // design decission do not implement since use case is of little value
                | CommandSpecifier::Ccs(crate::ClientCommandSpecifier::DownloadSegment) => {
                    // download segmented or request - aka write --> send abort code
                    // design decission do not implement since use case is of little value
                    CanOpenFrameBuilder::sdo_response(self.node_id)
                        .unwrap()
                        .with_index(index, subindex)
                        .abort(SDOAbortCode::UnsupportedAccess)
                        .build()
                }
                CommandSpecifier::Scs(_) => {
                    // Invalid command specifier in request --> send abort code
                    CanOpenFrameBuilder::sdo_response(self.node_id)
                        .unwrap()
                        .with_index(index, subindex)
                        .abort(SDOAbortCode::GeneralError)
                        .build()
                }
            };
            Some(response_frame)
        } else {
            None
        }
    }

    fn process_segmented(&self, index: u16, subindex: u8, _value: &str) -> CANOpenFrame {
        // todo
        // setup the session
        CanOpenFrameBuilder::sdo_response(self.node_id)
            .unwrap()
            .with_index(index, subindex)
            .abort(SDOAbortCode::ObjectDoesNotExist)
            .build()
    }

    fn send_expedited_object_value(
        &self,
        index: u16,
        subindex: u8,
        value: ValueVariant<'_>,
    ) -> CANOpenFrame {
        let mut buffer = [0_u8; 4];
        let data = value.to_little_endian_buffer(&mut buffer);
        CanOpenFrameBuilder::sdo_response(self.node_id)
            .unwrap()
            .with_index(index, subindex)
            .download(data)
            .build()
    }
}
