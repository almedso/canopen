//! CANOpen library
//!

#[allow(unused_must_use)]
#[allow(unused_variables)]
pub mod frame;
pub use frame::*;

pub mod node;
pub use node::*;

pub mod sdo_client;
pub use sdo_client::*;

pub mod util;
pub use util::*;

use thiserror::Error;

/// General Errors of the CANOpen library
/// Implement the Error trait by implementing Debug and Display
#[derive(Debug, Error, PartialEq)]
pub enum CanOpenError {
    #[error("The COB-ID of this frame is invalid ({cob_id})")]
    InvalidCobId { cob_id: u32 },
    #[error("The Node id of this frame is invalid ({node_id})")]
    InvalidNodeId { node_id: u8 },
    #[error("Invalid number {invalid_number}")]
    InvalidNumber { invalid_number: String },
    #[error("Data length should not exceed 8 bytes ({length} > 8)")]
    InvalidDataLength { length: usize },
    #[error("Invalid number type ({number_type})")]
    InvalidNumberType { number_type: String },
    #[error("Frame builder error")]
    BuilderError,
    #[error("SDO Payload parse error")]
    SdoPayloadParseError,
    #[error("SDO Payload not implemented yet")]
    SdoPayloadNotImplementedYet,
    #[error("SDO Request timed out")]
    SdoProtocolTimedOut,
    #[error("SDO AbortCode {abort_code}")]
    SdoAbortCode { abort_code: SDOAbortCode },
    #[error("Instanciation of can socket failed")]
    SocketInstanciatingError,
    #[error("Writing to can socket failed")]
    SocketWriteError,
    #[error("String is too long: allowed max {max_length} given {given_length}")]
    StringIsTooLong {
        max_length: usize,
        given_length: usize,
    },
    #[error("Object at: {index}, {subindex} does not exist")]
    ObjectDoesNotExist { index: u16, subindex: u8 },
    #[error("Cannot be formatted as a string")]
    Formatting,
    #[error("Cannot write to const storage")]
    CannotWriteToConstStorage,
    #[error("Writing to object is forbidden")]
    WritingForbidden,
    #[error("Reading from object is not possible")]
    ReadAccessImpossible,
    #[error("Sharded access to object failed")]
    SharedOdAccessError,
}
