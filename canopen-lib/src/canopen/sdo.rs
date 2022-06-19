use super::*;
use byteorder::{LittleEndian, ReadBytesExt};
use failure::{Error, Fail};
use std::io::Cursor;
use std::fmt;


type Result<T> = std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum SDOResult {
    #[fail(display = "Success")]
    Success,
    #[fail(display = "Failure")]
    Failure,
    #[fail(display = "Return Code: {} ", _0)]
    UnknownResult(u8),
}

#[derive(Fail, Debug)]
pub enum SDOAbortCode {
    #[fail(display = "Unknown abort code")]
    UnknownAbortCode,
    #[fail(display = "Toggle bit not alternated")]
    ToggleBitNotAlternated,
    #[fail(display = "SDO protocol timed out")]
    SDOProtocolTimedOut,
    #[fail(display = "Client/server command specifier not valid or unknown")]
    CommandSpecifierError,
    #[fail(display = "Invalid block size (block mode only)")]
    InvalidBlockSize,
    #[fail(display = "Invalid sequence number (block mode only)")]
    InvalidSequenceNumber,
    #[fail(display = "CRC error (block mode only)")]
    CRCError,
    #[fail(display = "Out of memory")]
    OutOfMemory,
    #[fail(display = "Unsupported access to an object")]
    UnsupportedAccess,
    #[fail(display = "Attempt to read a write only object")]
    ReadWriteOnlyError,
    #[fail(display = "Attempt to write a read only object")]
    WriteReadOnlyError,
    #[fail(display = "Object does not exist in the object dictionary")]
    ObjectDoesNotExist,
    #[fail(display = "Object cannot be mapped to the PDO")]
    ObjectCannotBeMapped,
    #[fail(display = "The number and length of the objects to be mapped would exceed PDO length")]
    PDOOverflow,
    #[fail(display = "General parameter incompatibility reason")]
    ParameterIncompatibility,
    #[fail(display = "General internal incompatibility in the device")]
    InternalIncompatibility,
    #[fail(display = "Access failed due to a hardware error")]
    HardwareError,
    #[fail(display = "Data type does not match, length of service parameter does not match")]
    WrongLength,
    #[fail(display = "Data type does not match, length of service parameter too high")]
    TooLong,
    #[fail(display = "Data type does not match, length of service parameter too low")]
    TooShort,
    #[fail(display = "Sub-index does not exist")]
    SubindexDoesNotExist,
    #[fail(display = "Value range of parameter exceeded (only for write access)")]
    WrongValue,
    #[fail(display = "Value of parameter written too high")]
    ValueTooHigh,
    #[fail(display = "Value of parameter written too low")]
    ValueTooLow,
    #[fail(display = "Maximum value is less than minimum value")]
    RangeError,
    #[fail(display = "General error")]
    GeneralError,
    #[fail(display = "Data cannot be transferred or stored to the application")]
    StorageError,
    #[fail(
        display = "Data cannot be transferred or stored to the application because of local control"
    )]
    LocalControlError,
    #[fail(
        display = "Data cannot be transferred or stored to the application because ofthe present device state"
    )]
    DeviceStateError,
    #[fail(
        display = "Object dictionary dynamic generation fails or no object dictionary is present"
    )]
    DictionaryError,
}

impl From<u64> for SDOAbortCode {
    fn from(abort_code: u64) -> Self {
        match abort_code {
            0x0503_0000 => SDOAbortCode::ToggleBitNotAlternated,
            0x0504_0000 => SDOAbortCode::SDOProtocolTimedOut,
            0x0504_0001 => SDOAbortCode::CommandSpecifierError,
            0x0504_0002 => SDOAbortCode::InvalidBlockSize,
            0x0504_0003 => SDOAbortCode::InvalidSequenceNumber,
            0x0504_0004 => SDOAbortCode::CRCError,
            0x0504_0005 => SDOAbortCode::OutOfMemory,
            0x0601_0000 => SDOAbortCode::UnsupportedAccess,
            0x0601_0001 => SDOAbortCode::ReadWriteOnlyError,
            0x0601_0002 => SDOAbortCode::WriteReadOnlyError,
            0x0602_0000 => SDOAbortCode::ObjectDoesNotExist,
            0x0604_0041 => SDOAbortCode::ObjectCannotBeMapped,
            0x0604_0042 => SDOAbortCode::PDOOverflow,
            0x0604_0043 => SDOAbortCode::ParameterIncompatibility,
            0x0604_0047 => SDOAbortCode::InternalIncompatibility,
            0x0606_0000 => SDOAbortCode::HardwareError,
            0x0607_0010 => SDOAbortCode::WrongLength,
            0x0607_0012 => SDOAbortCode::TooLong,
            0x0607_0013 => SDOAbortCode::TooShort,
            0x0609_0011 => SDOAbortCode::SubindexDoesNotExist,
            0x0609_0030 => SDOAbortCode::WrongValue,
            0x0609_0031 => SDOAbortCode::ValueTooHigh,
            0x0609_0032 => SDOAbortCode::ValueTooLow,
            0x0609_0036 => SDOAbortCode::RangeError,
            0x0800_0000 => SDOAbortCode::GeneralError,
            0x0800_0020 => SDOAbortCode::StorageError,
            0x0800_0021 => SDOAbortCode::LocalControlError,
            0x0800_0022 => SDOAbortCode::DeviceStateError,
            0x0800_0023 => SDOAbortCode::DictionaryError,
            _ => SDOAbortCode::UnknownAbortCode,
        }
    }
}

impl From<u8> for SDOResult {
    fn from(data: u8) -> SDOResult {
        match data {
            0x60 => SDOResult::Success,
            0x80 => SDOResult::Failure,
            result => SDOResult::UnknownResult(result),
        }
    }
}

#[derive(Debug)]
pub struct SDOError {
    msg: &'static str
}

impl SDOError {
    pub fn new(msg: &'static str) -> Self {
        SDOError{ msg }
    }
}

impl std::error::Error for SDOError {}

impl fmt::Display for SDOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}


#[derive(Debug)]
pub struct SDOServerResponse {
    result: SDOResult,
    index: u16,
    subindex: u8,
    data: u64,
}

impl SDOServerResponse {
    pub fn parse(frame: &CANOpenFrame) -> Result<SDOServerResponse> {
        match frame.frame_type()
        {
            FrameType::SsdoTx =>  Ok(SDOServerResponse {
                result: frame.data()[0].try_into()?,
                index: Cursor::new(&frame.data()[1..=2]).read_u16::<LittleEndian>()?,
                subindex: frame.data()[3],
                data: Cursor::new(&frame.data()[4..]).read_u64::<LittleEndian>()?,
            }),
            _ => Err(SDOError::new("SDO frame parse error").into())
        }
    }
}

impl std::fmt::Display for SDOServerResponse {
    fn fmt(
        self: &Self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
            write!(f, "{} - {:04},{:02X} [{:x}]\t", self.result, self.index, self.subindex, self.data)?;
            Ok(()
        )
        }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn main() {

    }
}
