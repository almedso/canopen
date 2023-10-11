//! SDO Client
//!
use crate::extract_length;
use crate::object_dictionary::ObjectDictionary;
use crate::CanOpenError;
use crate::CanOpenFrameBuilder;
use crate::CommandSpecifier;
use crate::{CANOpenFrame, FrameType, Payload, SDOAbortCode};
use std::rc::Rc;

// std/no-std dependent dependent
use log::debug;
use tokio_socketcan::{CANFrame, CANSocket}; // for reading next  from can socket

pub struct SdoServer<'a> {
    node_id: u8,
    can_socket: CANSocket,
    object_dictionary: Rc<ObjectDictionary<'a>>,
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
        }
    }

    /// Process a complete SDO request
    ///
    /// if it is an expedited request it is
    /// - receive a request
    /// - send a single repsonse.
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
    pub async fn process_complete_sdo_request(
        &self,
        frame: &CANOpenFrame,
    ) -> Result<(), CanOpenError> {
        if frame.node_id() == self.node_id && frame.frame_type() == FrameType::SdoRx {
            // only handle SDO Rx frames for of the node
            if let Payload::SdoWithIndex(payload) = &frame.payload {
                let index = payload.index;
                let subindex = payload.subindex;
                let response_frame_builder = match payload.cs {
                    CommandSpecifier::Ccs(crate::ClientCommandSpecifier::Download) => {
                        // download expedited request - aka write
                        CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .download_response()
                    }
                    CommandSpecifier::Ccs(crate::ClientCommandSpecifier::DownloadSegment) => {
                        // download segmented request - aka write --> send abort code
                        CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .abort(SDOAbortCode::UnsupportedAccess)
                    }
                    CommandSpecifier::Ccs(crate::ClientCommandSpecifier::Upload) => {
                        // download segmented request - aka write --> send abort code
                        CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .upload_one_byte_expedited_response(01_u8)
                    }
                    CommandSpecifier::Ccs(_) | CommandSpecifier::Scs(_) => {
                        // Invalid command specifier in request --> send abort code
                        CanOpenFrameBuilder::sdo_response(self.node_id)
                            .unwrap()
                            .with_index(index, subindex)
                            .abort(SDOAbortCode::GeneralError)
                    }
                };

                debug!("Send download response");
                self.can_socket
                    .write_frame(response_frame_builder.build().into())
                    .map_err(|_| -> CanOpenError { CanOpenError::SocketInstanciatingError })?
                    .await
                    .map_err(|_| -> CanOpenError { CanOpenError::SocketWriteError })?;
            } else {
                // no index and subindex; Data bigger than 4 byte -> segmented response is required
            }
        }
        Ok(())
    }
}

/*
fn sdo_download_expedited(...)
    od.download_expedited(...).maperr(|_| -> CanOpenError::SdoAbortCode { SDOAbortCode::ObjectDoesNotExist })?
    Ok(())
}
*/
