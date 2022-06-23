use super::*;
use failure::{Error, Fail};
use std::fmt;

type Result<T> = std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum SDOResult {
    #[fail(display = "Success")]
    Success,
    #[fail(display = "Failure")]
    Failure,
    #[fail(display = "Code Byte: {}", _0)]
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

impl From<u32> for SDOAbortCode {
    fn from(abort_code: u32) -> Self {
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
    msg: &'static str,
}

impl SDOError {
    pub fn new(msg: &'static str) -> Self {
        SDOError { msg }
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
    pub result: SDOResult,
    pub index: u16,
    pub subindex: u8,
    pub data: u32,
}

impl SDOServerResponse {
    pub fn parse(frame: &CANOpenFrame) -> Result<SDOServerResponse> {
        match frame.frame_type() {
            FrameType::SsdoTx | FrameType::SsdoRx => {
                let data = frame.data();
                Ok(SDOServerResponse {
                    result: data[0].into(),
                    index: (data[1] as u16) + ((data[2] as u16) << 8), // this is little endian
                    subindex: data[3],
                    data: (data[4] as u32)
                        + ((data[5] as u32) << 8)
                        + ((data[6] as u32) << 16)
                        + ((data[7] as u32) << 24), // this is little endian
                })
            }
            _ => Err(SDOError::new("SDO frame parse error").into()),
        }
    }
}

impl std::fmt::Display for SDOServerResponse {
    fn fmt(
        self: &Self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self.result {
            SDOResult::Failure => write!(
                f,
                "{} - {:#04x},{:#02x} {}\t",
                self.result,
                self.index,
                self.subindex,
                SDOAbortCode::from(self.data)
            )?,
            _ => write!(
                f,
                "{} - {:#04x},{:#02x} [{:#x}]\t",
                self.result, self.index, self.subindex, self.data
            )?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn main() {}
}
