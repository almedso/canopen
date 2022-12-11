//! # SDO Frames
//!
//! As for any other can open frames the the specialized SDO CANOpen frame
//! supports
//!
//! - create an SDO Frame by parsing a from CANOpen frame
//! - pretty formatting
//! - conversion into a CAN Frame
//! - public access to all elements
//!
//! SDO Frame payload details are determined by it's 1st byte payload aka *command byte*.
//!
//! There is
//! - **expedited SDO transfer** payload is less then or equal four bytes.
//!   Index (2nd and 3rd byte) and sub-index (4th byte) are always part of the data
//! - **segmented SDO transfer** if the payload is greater then four bytes.
//!   It is always always initialed by an expedited frame and each frame is acknowledged.
//! - **block SDO transfer** is similar to segmented transfer, except an acknowledgement
//!   is send after a block of frames.
//!
//! ## References for format specification
//!
//! - http://www.byteme.org.uk/canopenparent/canopen/sdo-service-data-objects-canopen/
//!   note client and server is mismatched - somehow buggy but helpful for sequences
//!
//! - https://docs.octave.dev/docs/canopen-reference-guide
//!   very compact and another view on CCS = client command specifier
//!   SCS server command specifier
//!
//! - https://www.motorpowerco.com/media/filer_public/32/c2/32c2f3a8-17cb-4204-8249-ffe5fc4e6c04/bpro_canopen_implementationguide.pdf
//!

use num_enum::IntoPrimitive;

use super::*;
use std::fmt::{self, Formatter};

pub use crate::Split;

pub mod builder;

/// All SDO Frames are composed of always 8 bytes data payload formatted in little endian
pub type SdoPayloadData = [u8; 8];

/// Client command specifier (ccs)
/// /// Frame type is SdoRx (x600 + node)
#[derive(IntoPrimitive)]
#[repr(u8)]
#[derive(Display, Debug, PartialEq, Clone)]
pub enum ClientCommandSpecifier {
    #[allow(clippy::unusual_byte_groupings)]
    DownloadSegment = 0b_000_00000,
    #[allow(clippy::unusual_byte_groupings)]
    Download = 0b_001_00000,
    #[allow(clippy::unusual_byte_groupings)]
    Upload = 0b_010_00000,
    #[allow(clippy::unusual_byte_groupings)]
    UploadSegment = 0b_011_00000,

    #[allow(clippy::unusual_byte_groupings)]
    BlockUpload = 0b_101_00000,
    #[allow(clippy::unusual_byte_groupings)]
    BlockDownload = 0b_110_00000,

    #[allow(clippy::unusual_byte_groupings)]
    Unspecified = 0b_111_00000,
}

impl From<u8> for ClientCommandSpecifier {
    fn from(data: u8) -> ClientCommandSpecifier {
        #[allow(clippy::unusual_byte_groupings)]
        match data & 0b_111_00000 {
            #[allow(clippy::unusual_byte_groupings)]
            0b_000_00000 => ClientCommandSpecifier::DownloadSegment,
            #[allow(clippy::unusual_byte_groupings)]
            0b_001_00000 => ClientCommandSpecifier::Download,
            #[allow(clippy::unusual_byte_groupings)]
            0b_010_00000 => ClientCommandSpecifier::Upload,
            #[allow(clippy::unusual_byte_groupings)]
            0b_011_00000 => ClientCommandSpecifier::UploadSegment,
            #[allow(clippy::unusual_byte_groupings)]
            0b_101_00000 => ClientCommandSpecifier::BlockUpload,
            #[allow(clippy::unusual_byte_groupings)]
            0b_110_00000 => ClientCommandSpecifier::BlockDownload,
            _ => ClientCommandSpecifier::Unspecified,
        }
    }
}

/// Server command specifier (scs)
/// Frame type is SdoTx (x580 + node)
#[derive(IntoPrimitive)]
#[repr(u8)]
#[derive(Display, Debug, PartialEq, Clone)]
pub enum ServerCommandSpecifier {
    #[allow(clippy::unusual_byte_groupings)]
    UploadSegment = 0b_000_00000,
    #[allow(clippy::unusual_byte_groupings)]
    DownloadSegment = 0b_001_00000,
    #[allow(clippy::unusual_byte_groupings)]
    Upload = 0b_010_00000,
    #[allow(clippy::unusual_byte_groupings)]
    Download = 0b_011_00000,
    #[allow(clippy::unusual_byte_groupings)]
    Abort = 0b_100_00000,

    #[allow(clippy::unusual_byte_groupings)]
    BlockUpload = 0b_101_00000,
    #[allow(clippy::unusual_byte_groupings)]
    BlockDownload = 0b_110_00000,

    #[allow(clippy::unusual_byte_groupings)]
    Unspecified = 0b_111_00000,
}

impl From<u8> for ServerCommandSpecifier {
    fn from(data: u8) -> ServerCommandSpecifier {
        #[allow(clippy::unusual_byte_groupings)]
        match data & 0b_111_00000 {
            #[allow(clippy::unusual_byte_groupings)]
            0b_000_00000 => ServerCommandSpecifier::UploadSegment,
            #[allow(clippy::unusual_byte_groupings)]
            0b_001_00000 => ServerCommandSpecifier::DownloadSegment,
            #[allow(clippy::unusual_byte_groupings)]
            0b_010_00000 => ServerCommandSpecifier::Upload,
            #[allow(clippy::unusual_byte_groupings)]
            0b_011_00000 => ServerCommandSpecifier::Download,
            #[allow(clippy::unusual_byte_groupings)]
            0b_100_00000 => ServerCommandSpecifier::Abort,
            #[allow(clippy::unusual_byte_groupings)]
            0b_101_00000 => ServerCommandSpecifier::BlockUpload,
            #[allow(clippy::unusual_byte_groupings)]
            0b_110_00000 => ServerCommandSpecifier::BlockDownload,
            _ => ServerCommandSpecifier::Unspecified,
        }
    }
}

/// Can be either server side command code or client side command code
#[derive(Debug, PartialEq, Clone)]
pub enum CommandSpecifier {
    Ccs(ClientCommandSpecifier),
    Scs(ServerCommandSpecifier),
}

impl Display for CommandSpecifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CommandSpecifier::Ccs(x) => write!(f, "{}", x)?,
            CommandSpecifier::Scs(x) => write!(f, "{}", x)?,
        }
        Ok(())
    }
}

impl Into<u8> for CommandSpecifier {
    fn into(self) -> u8 {
        match self {
            CommandSpecifier::Ccs(x) => x as u8,
            CommandSpecifier::Scs(x) => x as u8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SDOAbortCode {
    UnknownAbortCode(u32),
    ToggleBitNotAlternated,
    SDOProtocolTimedOut,
    ClientCommandSpecifierError,
    InvalidBlockSize,
    InvalidSequenceNumber,
    CRCError,
    OutOfMemory,
    UnsupportedAccess,
    ReadWriteOnlyError,
    WriteReadOnlyError,
    ObjectDoesNotExist,
    ObjectCannotBeMapped,
    PDOOverflow,
    ParameterIncompatibility,
    InternalIncompatibility,
    HardwareError,
    WrongLength,
    TooLong,
    TooShort,
    SubindexDoesNotExist,
    WrongValue,
    ValueTooHigh,
    ValueTooLow,
    RangeError,
    GeneralError,
    StorageError,
    LocalControlError,
    DeviceStateError,
    DictionaryError,
}

impl Display for SDOAbortCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAbortCode(x) => write!(f, "Unknown abort code {:?}", x),
            Self::ToggleBitNotAlternated => write!(f, "Toggle bit not alternated"),
            Self::SDOProtocolTimedOut => write!(f, "SDO protocol timed out"),
            Self::ClientCommandSpecifierError => write!(f, "Client/server command specifier not valid or unknown"),
            Self::InvalidBlockSize => write!(f, "Invalid block size (block mode only)"),
            Self::InvalidSequenceNumber => write!(f, "Invalid sequence number (block mode only)"),
            Self::CRCError => write!(f, "CRC error"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::UnsupportedAccess => write!(f, "Unsupported access to an object"),
            Self::ReadWriteOnlyError => write!(f, "Attempt to read a write only object"),
            Self::WriteReadOnlyError => write!(f, "Attempt to write a read only object"),
            Self::ObjectDoesNotExist => write!(f, "Object does not exist in the object dictionary"),
            Self::ObjectCannotBeMapped => write!(f, "Object cannot be mapped to the PDO"),
            Self::PDOOverflow => write!(f, "The number and length of the objects to be mapped would exceed PDO length"),
            Self::ParameterIncompatibility => write!(f, "General parameter incompatibility reason"),
            Self::InternalIncompatibility => write!(f, "General internal incompatibility in the device"),
            Self::HardwareError => write!(f, "Access failed due to a hardware error"),
            Self::WrongLength => write!(f, "Data type does not match, length of service parameter does not match"),
            Self::TooLong => write!(f, "Data type does not match, length of service parameter too high"),
            Self::TooShort => write!(f, "Data type does not match, length of service parameter too low"),
            Self::SubindexDoesNotExist => write!(f, "Sub-index does not exist"),
            Self::WrongValue => write!(f, "Value range of parameter exceeded (only for write access)"),
            Self::ValueTooHigh => write!(f, "Value of parameter written too high"),
            Self::ValueTooLow => write!(f, "Value of parameter written too low"),
            Self::RangeError => write!(f, "Maximum value is less than minimum value"),
            Self::GeneralError => write!(f, "General error"),
            Self::StorageError => write!(f, "Data cannot be transferred or stored to the application"),
            Self::LocalControlError => write!(f, "Data cannot be transferred or stored to the application because of local control"),
            Self::DeviceStateError => write!(f, "Data cannot be transferred or stored to the application because of the present device state"),
            Self::DictionaryError => write!(f, "Object dictionary dynamic generation fails or no object dictionary is present"),
        }
    }
}

impl From<u32> for SDOAbortCode {
    fn from(abort_code: u32) -> Self {
        match abort_code {
            0x0503_0000 => SDOAbortCode::ToggleBitNotAlternated,
            0x0504_0000 => SDOAbortCode::SDOProtocolTimedOut,
            0x0504_0001 => SDOAbortCode::ClientCommandSpecifierError,
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
            x => SDOAbortCode::UnknownAbortCode(x),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<u32> for SDOAbortCode {
    fn into(self) -> u32 {
        match self {
            SDOAbortCode::ToggleBitNotAlternated => 0x0503_0000,
            SDOAbortCode::SDOProtocolTimedOut => 0x0504_0000,
            SDOAbortCode::ClientCommandSpecifierError => 0x0504_0001,
            SDOAbortCode::InvalidBlockSize => 0x0504_0002,
            SDOAbortCode::InvalidSequenceNumber => 0x0504_0003,
            SDOAbortCode::CRCError => 0x0504_0004,
            SDOAbortCode::OutOfMemory => 0x0504_0005,
            SDOAbortCode::UnsupportedAccess => 0x0601_0000,
            SDOAbortCode::ReadWriteOnlyError => 0x0601_0000,
            SDOAbortCode::WriteReadOnlyError => 0x0601_0002,
            SDOAbortCode::ObjectDoesNotExist => 0x0602_0000,
            SDOAbortCode::ObjectCannotBeMapped => 0x0604_0041,
            SDOAbortCode::PDOOverflow => 0x0604_0042,
            SDOAbortCode::ParameterIncompatibility => 0x0604_0043,
            SDOAbortCode::InternalIncompatibility => 0x0604_0047,
            SDOAbortCode::HardwareError => 0x0606_0000,
            SDOAbortCode::WrongLength => 0x0607_0010,
            SDOAbortCode::TooLong => 0x0607_0012,
            SDOAbortCode::TooShort => 0x0607_0013,
            SDOAbortCode::SubindexDoesNotExist => 0x0609_0011,
            SDOAbortCode::WrongValue => 0x0609_0030,
            SDOAbortCode::ValueTooHigh => 0x0609_0031,
            SDOAbortCode::ValueTooLow => 0x0609_0032,
            SDOAbortCode::RangeError => 0x0609_0036,
            SDOAbortCode::GeneralError => 0x0800_0000,
            SDOAbortCode::StorageError => 0x0800_0020,
            SDOAbortCode::LocalControlError => 0x0800_0021,
            SDOAbortCode::DeviceStateError => 0x0800_0022,
            SDOAbortCode::DictionaryError => 0x0800_0023,
            SDOAbortCode::UnknownAbortCode(x) => x,
        }
    }
}

impl From<u8> for CommandDataSize {
    fn from(data: u8) -> CommandDataSize {
        #[allow(clippy::unusual_byte_groupings)]
        match data & 0b_0000_11_0_1 {
            #[allow(clippy::unusual_byte_groupings)]
            0b_00000_00_0_1 => CommandDataSize::FourBytes,
            #[allow(clippy::unusual_byte_groupings)]
            0b_00000_01_0_1 => CommandDataSize::ThreeBytes,
            #[allow(clippy::unusual_byte_groupings)]
            0b_00000_10_0_1 => CommandDataSize::TwoBytes,
            #[allow(clippy::unusual_byte_groupings)]
            0b_00000_11_0_1 => CommandDataSize::OneByte,
            // bit zero (a.k.a. size is given bit) is zero =b_0000_xx_0_0
            other => CommandDataSize::NotSet,
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

// the toggled flag is ignored
// the expedited flag is always set
// data size flag is always set
#[derive(Debug, PartialEq, Clone)]
pub struct WithIndexPayload {
    pub cs: CommandSpecifier,
    pub size: CommandDataSize,
    pub expedited_flag: bool,
    pub index: u16,
    pub subindex: u8,
    pub data: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithoutIndexPayload {
    pub cs: CommandSpecifier,
    pub toggle: bool,
    // has some value if the sized flag is set
    pub length_of_empty_bytes: Option<u8>,
    pub data: [u8; 7],
}

#[derive(Debug, PartialEq, Clone)]
pub enum CommandToggleFlag {
    #[allow(clippy::unusual_byte_groupings)]
    Set = 0b000_1_0000, // 0x10
    #[allow(clippy::unusual_byte_groupings)]
    Clear = 0b000_0_000,
}

/// Bit 0 - indicates if the data size is set
/// Bit 2 and 3 indicate the number of bits
#[derive(IntoPrimitive)]
#[repr(u8)]
#[derive(Debug, PartialEq, Clone)]
pub enum CommandDataSize {
    #[allow(clippy::unusual_byte_groupings)]
    OneByte = 0b0000_11_0_1, // 0x0c + 1 indication bit
    #[allow(clippy::unusual_byte_groupings)]
    TwoBytes = 0b0000_10_0_1, // 0x08 + 1 indication bit
    #[allow(clippy::unusual_byte_groupings)]
    ThreeBytes = 0b0000_01_0_1, // 0x04 + 1 indication bit
    #[allow(clippy::unusual_byte_groupings)]
    FourBytes = 0b0000_00_0_1, // 0x00 + 1 indication bit
    #[allow(clippy::unusual_byte_groupings)]
    NotSet = 0b0000_00_0_0, // zero indication bit
}

#[derive(Debug)]
pub enum CommandTransferFlag {
    #[allow(clippy::unusual_byte_groupings)]
    Expedited = 0b000000_1_0, // 0x02
    #[allow(clippy::unusual_byte_groupings)]
    NotExpedited = 0b000000_0_0,
}

#[derive(Debug)]
pub enum CommandSizeFlag {
    #[allow(clippy::unusual_byte_groupings)]
    Indicated = 0b0000000_1, // 0x01
    #[allow(clippy::unusual_byte_groupings)]
    NotIndicated = 0b0000000_0,
}

fn is_size_flag_set(command_byte: u8) -> bool {
    #[allow(clippy::unusual_byte_groupings)]
    const SIZE_MASK: u8 = 0b00000000_1;
    (command_byte & SIZE_MASK) != 0
}

fn is_toggle_flag_set(command_byte: u8) -> bool {
    #[allow(clippy::unusual_byte_groupings)]
    const TOGGLE_MASK: u8 = 0b000_1_0000;
    (command_byte & TOGGLE_MASK) != 0
}

fn length_of_empty_bytes(command_byte: u8) -> Option<u8> {
    if is_size_flag_set(command_byte) {
        #[allow(clippy::unusual_byte_groupings)]
        const LENGTH_MASK: u8 = 0b0000_111_0;
        let l = (command_byte & LENGTH_MASK) >> 1;
        Some(l)
    } else {
        None
    }
}

impl WithIndexPayload {
    ///  Parse a payload that is a priori a sdo payload, i.e. 8 byte in size
    pub fn parse_as_server_payload(data: SdoPayloadData) -> Result<WithIndexPayload, CanOpenError> {
        let command_specifier: ServerCommandSpecifier = data[0].into();
        match command_specifier {
            ServerCommandSpecifier::Abort
            | ServerCommandSpecifier::Upload
            | ServerCommandSpecifier::UploadSegment
            | ServerCommandSpecifier::Download
            | ServerCommandSpecifier::DownloadSegment => {
                Ok(WithIndexPayload {
                    cs: CommandSpecifier::Scs(command_specifier),
                    size: data[0].into(),
                    #[allow(clippy::unusual_byte_groupings)]
                    expedited_flag: ((data[0] & 0b0000_00_1_0) != 0),
                    index: (data[1] as u16) + ((data[2] as u16) << 8), // this is little endian
                    subindex: data[3],
                    data: (data[4] as u32)
                        + ((data[5] as u32) << 8)
                        + ((data[6] as u32) << 16)
                        + ((data[7] as u32) << 24), // this is little endian
                })
            }
            _ => Err(CanOpenError::SdoPayloadParseError),
        }
    }

    pub fn parse_as_client_payload(payload: SdoPayloadData) -> Result<Self, CanOpenError> {
        let command_specifier = ClientCommandSpecifier::from(payload[0]);
        match command_specifier {
            ClientCommandSpecifier::Download
            | ClientCommandSpecifier::Upload
            | ClientCommandSpecifier::UploadSegment => Ok(WithIndexPayload {
                cs: CommandSpecifier::Ccs(command_specifier),
                size: CommandDataSize::from(payload[0]),
                #[allow(clippy::unusual_byte_groupings)]
                expedited_flag: ((payload[0] & 0b0000_00_1_0) == 0b0000_00_1_0),
                index: payload[1] as u16 + ((payload[2] as u16) << 8),
                subindex: payload[3],

                data: payload[4] as u32
                    + ((payload[5] as u32) << 8)
                    + ((payload[6] as u32) << 16)
                    + ((payload[7] as u32) << 24),
            }),
            ClientCommandSpecifier::BlockDownload | ClientCommandSpecifier::BlockUpload => {
                Err(CanOpenError::SdoPayloadNotImplementedYet)
            }
            ClientCommandSpecifier::DownloadSegment | ClientCommandSpecifier::Unspecified => {
                Err(CanOpenError::SdoPayloadParseError)
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<SdoPayloadData> for WithIndexPayload {
    fn into(self) -> SdoPayloadData {
        let expedited_bit: u8 = if self.expedited_flag {
            #[allow(clippy::unusual_byte_groupings)]
            0b000000_1_0
        } else {
            #[allow(clippy::unusual_byte_groupings)]
            0b000000_0_0
        };
        let command_specifier: u8 = self.size.into();
        let command_data_size: u8 = self.cs.into();

        let mut payload: SdoPayloadData = [0_u8; 8];
        // command byte
        payload[0] = command_specifier + command_data_size + expedited_bit;
        // index (little endian)
        payload[1] = self.index.lo();
        payload[2] = self.index.hi();
        // subindex
        payload[3] = self.subindex;
        // object data (upper bytes are filled with zero) length is filled by command byte
        payload[4] = self.data.lo().lo();
        payload[5] = self.data.lo().hi();
        payload[6] = self.data.hi().lo();
        payload[7] = self.data.hi().hi();

        payload
    }
}

impl std::fmt::Display for WithIndexPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.cs == CommandSpecifier::Scs(ServerCommandSpecifier::Abort) {
            write!(
                f,
                "{} - {:#04x},{:#02x} {}\t",
                self.cs,
                self.index,
                self.subindex,
                SDOAbortCode::from(self.data)
            )?;
        } else {
            write!(
                f,
                "{} - {:#04x},{:#02x} [{:#x}]\t",
                self.cs, self.index, self.subindex, self.data
            )?;
        }
        Ok(())
    }
}

impl WithoutIndexPayload {
    pub fn parse_as_server_payload(payload: SdoPayloadData) -> Result<Self, CanOpenError> {
        let server_command_specifier = ServerCommandSpecifier::from(payload[0]);
        match server_command_specifier {
            ServerCommandSpecifier::Upload => Ok(WithoutIndexPayload {
                cs: CommandSpecifier::Scs(server_command_specifier),
                length_of_empty_bytes: WithoutIndexPayload::get_length_from_command_byte(
                    payload[0],
                ),
                #[allow(clippy::unusual_byte_groupings)]
                toggle: ((payload[0] & 0b000_1_0000) == 0b000_1_0000),

                data: [
                    payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
                    payload[7],
                ],
            }),
            _ => Err(CanOpenError::SdoPayloadParseError),
        }
    }

    pub fn parse_as_client_payload(payload: SdoPayloadData) -> Result<Self, CanOpenError> {
        let client_command_specifier = ClientCommandSpecifier::from(payload[0]);
        match client_command_specifier {
            ClientCommandSpecifier::UploadSegment => Ok(WithoutIndexPayload {
                cs: CommandSpecifier::Ccs(client_command_specifier),
                length_of_empty_bytes: WithoutIndexPayload::get_length_from_command_byte(
                    payload[0],
                ),
                #[allow(clippy::unusual_byte_groupings)]
                toggle: ((payload[0] & 0b000_1_0000) == 0b000_1_0000),

                data: [
                    payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
                    payload[7],
                ],
            }),
            _ => Err(CanOpenError::SdoPayloadParseError),
        }
    }

    fn get_length_from_command_byte(command_byte: u8) -> Option<u8> {
        if (command_byte & 0x01) == 0x00 {
            None
        } else {
            Some((command_byte & 0b0000_1110) >> 1)
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<SdoPayloadData> for WithoutIndexPayload {
    fn into(self) -> SdoPayloadData {
        let toggle_bit: u8 = if self.toggle {
            #[allow(clippy::unusual_byte_groupings)]
            0b000_1_0000
        } else {
            #[allow(clippy::unusual_byte_groupings)]
            0b000_0_0000
        };
        let size: u8 = match self.length_of_empty_bytes {
            #[allow(clippy::unusual_byte_groupings)]
            None => 0b000_000_0,
            Some(x) => {
                0b0000_000_1_u8 + // size bit is set
                ((x & 0b111_u8 ) << 1)
            }
        };
        let command_specifier: u8 = self.cs.into();

        [
            command_specifier + toggle_bit + size, // command byte
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
        ]
    }
}

impl std::fmt::Display for WithoutIndexPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{} - toggle: {} size: {} data: {:?}\t",
            self.cs,
            if self.toggle { "set" } else { "clear" },
            match self.length_of_empty_bytes {
                None => "No set".to_owned(),
                Some(s) => s.to_string(),
            },
            self.data,
        )?;
        Ok(())
    }
}

pub fn extract_length(size: CommandDataSize) -> usize {
    match size {
        CommandDataSize::NotSet => 0,
        CommandDataSize::OneByte => 1,
        CommandDataSize::TwoBytes => 2,
        CommandDataSize::ThreeBytes => 3,
        CommandDataSize::FourBytes => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_size_flag_set() {
        assert!(is_size_flag_set(0b1));
        assert!(!is_size_flag_set(0b0));
        assert!(is_size_flag_set(0b1001));
        assert!(!is_size_flag_set(0b111110));
    }

    #[test]
    fn test_is_toggle_flag_set() {
        assert!(is_toggle_flag_set(0b000_1_0000));
        assert!(!is_toggle_flag_set(0b000_0_0000));
        assert!(is_toggle_flag_set(0b010_1_0000));
        assert!(!is_toggle_flag_set(0b000_0_0001));
    }

    #[test]
    fn test_length_of_empty_bytes() {
        assert_eq!(Some(3), length_of_empty_bytes(0b000_011_1));
        assert_eq!(Some(0), length_of_empty_bytes(0b0000_000_1));
        assert_eq!(None, length_of_empty_bytes(0b0000_110_0));
    }

    #[test]
    fn test_command_data_size_into_u8() {
        assert_eq!(0_u8, CommandDataSize::NotSet.into());
    }

    #[allow(non_snake_case)]
    mod WithIndexPayload {
        use crate::{
            ClientCommandSpecifier, CommandDataSize, CommandSpecifier, SdoPayloadData,
            WithIndexPayload,
        };

        #[test]
        fn into_conversion() {
            let payload = WithIndexPayload {
                cs: CommandSpecifier::Ccs(ClientCommandSpecifier::BlockDownload),
                size: CommandDataSize::OneByte,
                expedited_flag: false,
                index: 0x2211,
                subindex: 0x33,
                data: 0x77665544,
            };
            let expected: SdoPayloadData = payload.into();
            assert_eq!(
                // expedited bit and size bit set (0x000000_11)
                [0b_110_0_11_0_1, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77],
                expected
            );

            let payload = WithIndexPayload {
                cs: CommandSpecifier::Ccs(ClientCommandSpecifier::DownloadSegment),
                size: CommandDataSize::FourBytes,
                expedited_flag: true,
                index: 0x2211,
                subindex: 0x33,
                data: 0x77665544,
            };
            let expected: SdoPayloadData = payload.into();
            assert_eq!(
                // expedited bit and size bit set (0x000000_11)
                [0b_000_0_00_1_1, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77],
                expected
            );
        }
    }
}
