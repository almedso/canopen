//! SDO Client
//!
use crate::extract_length;
use crate::object_dictionary::ObjectDictionary;
use crate::CanOpenError;
use crate::CanOpenFrameBuilder;
use crate::CommandSpecifier;
use crate::{CANOpenFrame, FrameType, Payload, SDOAbortCode};

// std/no-std dependent dependent
use log::debug;
use tokio_socketcan::CANSocket; // for reading next  from can socket

pub struct SdoServer {
    node_id: u8,
    can_socket: CANSocket,
    object_dictionary: &ObjectDictionary,
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
/// use tokio_socketcan::CANSocket;
/// use tokio;
///
///
/// fn main() {
///     let my_future = async {
///         let mut can_socket = match CANSocket::open("can0") {
///             Ok(socket) => socket,
///             Err(error) => {
///                 error!("Error opening {}: {}", cli.interface, error);
///                 quit::with_code(1);
///             }
///         };
///         const node_id: u8 = 10;
///         let server = SdoServer::new(node_id, can_socket);
///         loop {
///             server.run().await;
///         }
///     }
///     let rt = tokio::runtime::Runtime::new().unwrap();
///     rt.block_on(my_future) // tokio async runtime
/// }
/// ```
impl SdoServer {
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
    pub fn new(node_id: u8, can_socket: CANSocket, object_dictionary: &ObjectDictionary) -> Self {
        if node_id > 0x7F {
            panic!("node_id is out of allowed range [0..0x7F] {:?}", node_id);
        }
        SdoServer {
            node_id,
            can_socket,
            object_dictionary,
        }
    }

    /// Run function ...
    pub fn async run(&self) {
        while let Some(Ok(frame)) = self.can_socket.next().await {
            let frame = CANOpenFrame::try_from(frame)?;
            if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoRx {
                if let Payload::SdoWithIndex(payload) = frame.payload {
                    if payload.expedited_flag {
                        // Expedited response
                        len = extract_length(payload.size);
                        for (i, item) in data.iter_mut().enumerate() {
                            if i < len {
                                *item = (payload.data >> (8 * i) & 0xff) as u8;
                            }
                        }
                        break;
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
                }
            }
        }
    }
}

/*
fn sdo_download_expedited(...)
    od.download_expedited(...).maperr(|_| -> CanOpenError::SdoAbortCode { SDOAbortCode::ObjectDoesNotExist })?
    Ok(())
}
*/
