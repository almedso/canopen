use super::*;
use failure::{Error, Fail};
use std::fmt;

type Result<T> = std::result::Result<T, Error>;

/// Command specifier is a harmonized mix of
/// client command specifier (ccs) and server command specifier (scs)
#[derive(Fail, Debug, PartialEq)]
pub enum CommandSpecifier {
    #[fail(display = "DownloadSegment: Read of n-th segment success")]
    Download,
    #[fail(display = "InitiateDownload: Confirm write of segment or expedited read success")]
    InitiateDownload,
    #[fail(display = "InitiateUpload: First segmented read response success")]
    InitiateUpload,
    #[fail(display = "Upload")]
    Upload,
    #[fail(display = "Abort")]
    Abort,
    #[fail(display = "Block upload")]
    BlockUpload,
    #[fail(display = "Block download")]
    BlockDownload,
    #[fail(display = "Code Byte: {}", _0)]
    Unspecified(u8),
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

// http://www.byteme.org.uk/canopenparent/canopen/sdo-service-data-objects-canopen/
// note client and server is mismatched - somehow buggy but helpful for sequences

// https://docs.octave.dev/docs/canopen-reference-guide
// very compact and another view on CCS = cleint command specifier
// SCS server command specifier
//
// https://www.motorpowerco.com/media/filer_public/32/c2/32c2f3a8-17cb-4204-8249-ffe5fc4e6c04/bpro_canopen_implementationguide.pdf


impl From<u8> for CommandSpecifier {
    fn from(data: u8) -> CommandSpecifier {
        match data & 0b_111_00000 {
            0b_000_00000 => CommandSpecifier::Download,
            0b_001_00000 => CommandSpecifier::InitiateDownload,
            0b_010_00000 => CommandSpecifier::InitiateUpload,
            0b_011_00000 => CommandSpecifier::Upload,
            0b_100_00000 => CommandSpecifier::Abort,
            0b_101_00000 => CommandSpecifier::BlockUpload,
            0b_110_00000 => CommandSpecifier::BlockDownload,
            _ => CommandSpecifier::Unspecified(data),  // only 0b_111_00000 is possible
        }
    }
}

impl Into<u8> for CommandSpecifier {
    fn into(self) -> u8 {
        match self {
            CommandSpecifier::Download => 0b_000_00000,
            CommandSpecifier::InitiateDownload => 0b_001_00000,
            CommandSpecifier::InitiateUpload => 0b_010_00000,
            CommandSpecifier::Upload => 0b_011_00000,
            CommandSpecifier::Abort => 0b_100_00000,
            CommandSpecifier::BlockUpload => 0b_101_00000,
            CommandSpecifier::BlockDownload => 0b_110_00000,
            CommandSpecifier::Unspecified(x) => x,
        }
    }
}

impl From<u8> for CommandDataSize {
    fn from(data: u8) -> CommandDataSize {
        match data & 0b_0000_11_00 {
            0b_00000_00_00 => CommandDataSize::FourBytes,
            0b_00000_01_00 => CommandDataSize::ThreeBytes,
            0b_00000_10_00 => CommandDataSize::TwoBytes,
            0b_00000_11_00 => CommandDataSize::OneByte,
            other_impossible => CommandDataSize::FourBytes,  // cannot happen; please the compiler
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
#[derive(Debug)]
pub struct SdoExpeditedFrame {
    pub cs: CommandSpecifier,
    pub size: CommandDataSize,
    pub index: u16,
    pub subindex: u8,
    pub data: u32,
}

#[derive(Debug)]
pub struct SdoSegmentedFrame {
    pub cs: CommandSpecifier,
    pub toggle: bool,
    // has some value if the sized flag is set
    pub length_of_empty_bytes: Option<u8>,
    pub data: [u8; 7],
}

#[derive(Debug)]
pub enum SdoFrame {
    Expedited(SdoExpeditedFrame),
    Segmented(SdoSegmentedFrame),
}

#[derive(Debug)]
pub enum CommandToggleFlag {
    Set = 0b000_1_0000, // 0x10
    Clear = 0b000_0_000,
}

#[derive(Debug)]
pub enum CommandDataSize {
    OneByte = 0b0000_11_00, // 0x0c
    TwoBytes = 0b0000_10_00, // 0x08
    ThreeBytes = 0b0000_01_00, // 0x04
    FourBytes = 0b0000_00_00,
}

#[derive(Debug)]
pub enum CommandTransferFlag {
    Expedited = 0b000000_1_0, // 0x02
    NotExpedited = 0b000000_0_0,
}

#[derive(Debug)]
pub enum CommandSizeFlag {
    Indicated = 0b0000000_1, // 0x01
    NotIndicated = 0b0000000_0,
}

fn is_size_flag_set(command_byte: u8) -> bool {
    const SIZE_MASK : u8 = 0b00000000_1;
    (command_byte & SIZE_MASK) != 0
}

fn is_toggle_flag_set(command_byte: u8) -> bool {
    const TOGGLE_MASK : u8 = 0b000_1_0000;
    (command_byte & TOGGLE_MASK) != 0
}

fn length_of_empty_bytes(command_byte: u8) -> Option<u8> {
    if is_size_flag_set(command_byte) {
        const LENGTH_MASK : u8 = 0b0000_111_0;
        let l = (command_byte & LENGTH_MASK) >> 1;
        Some(l)
    } else {
        None
    }
}

impl SdoFrame {
    pub fn parse(frame: &CANOpenFrame) -> Result<SdoFrame> {
        match frame.frame_type() {
            FrameType::SsdoTx | FrameType::SsdoRx => {
                let data = frame.data();
                let command_specifier: CommandSpecifier = data[0].into();
                match command_specifier {
                    CommandSpecifier::Abort | CommandSpecifier::Download | CommandSpecifier::Upload =>
                    Ok(SdoFrame::Expedited( SdoExpeditedFrame {
                        cs: command_specifier,
                        size: data[0].into(),
                        index: (data[1] as u16) + ((data[2] as u16) << 8), // this is little endian
                        subindex: data[3],
                        data: (data[4] as u32)
                            + ((data[5] as u32) << 8)
                            + ((data[6] as u32) << 16)
                            + ((data[7] as u32) << 24), // this is little endian
                    })),
                    _ => {
                        Ok(SdoFrame::Segmented ( SdoSegmentedFrame {
                            cs: command_specifier,
                            toggle: is_toggle_flag_set(data[0]),
                            length_of_empty_bytes: length_of_empty_bytes(data[0]),
                            data: [ data[1], data[2], data[3], data[4], data[5], data[6],data[7]],
                        }))
                    }
                }

            }
            _ => Err(SDOError::new("SDO frame parse error").into()),
        }
    }
}

impl std::fmt::Display for SdoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            SdoFrame::Expedited(x) => {

                // CommandSpecifier::Failure => write!(
                //     f,
                //     "{} - {:#04x},{:#02x} {}\t",
                //     xf.cs,
                //     x.index,
                //     x.subindex,
                //     SDOAbortCode::from(self.data)
                // )?,
                // _ => write!(
                //     f,
                //     "{} - {:#04x},{:#02x} [{:#x}]\t",
                //     x.cs, x.index, x.subindex, x.data
                // )?,
            }
            SdoFrame::Segmented(x) => { 
                // write!(
            //     f,
            //     "{} - {:#04x},{:#02x} {}\t",
            //     x.cs,
            //     x.data,
            // )?,
            // _ => write!(
            //     f,
            //     "{} - {:#04x},{:#02x} [{:#x}]\t",
            //     self.result, self.index, self.subindex, self.data
            // )?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_into_command_specifier() {
        //assert_eq!(0b_000_00000_u8, CommandSpecifier::DownloadSegment.into());
        assert_eq!(sdo::CommandSpecifier::Download, 0b_000_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::InitiateDownload, 0b_001_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::InitiateUpload, 0b_010_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::Upload, 0b_011_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::Abort, 0b_100_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::BlockUpload, 0b_101_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::BlockDownload, 0b_110_00001_u8.into());
        assert_eq!(sdo::CommandSpecifier::Unspecified(0b111_00001), 0b_111_00001_u8.into());
    }

    #[test]
    fn u8_into_command_specifier_and_back() {
        for i in 0_u8..=210 {
            let cs: CommandSpecifier = i.into();
            if i & 0b_111_00000 == 0b_111_00000 {
                assert_eq!(i, cs.into())
            } else {
                assert_eq!(  i & 0b_111_00000_u8, cs.into());
            }


        }
    }

    #[test]
    fn test_is_size_flag_set() {
        assert!( is_size_flag_set(0b1) );
        assert!( ! is_size_flag_set(0b0) );
        assert!( is_size_flag_set(0b1001) );
        assert!( ! is_size_flag_set(0b111110) );

    }

    #[test]
    fn test_is_toggle_flag_set() {
        assert!( is_toggle_flag_set(0b000_1_0000) );
        assert!( ! is_toggle_flag_set(0b000_0_0000) );
        assert!( is_toggle_flag_set(0b010_1_0000) );
        assert!( ! is_toggle_flag_set(0b000_0_0001) );
    }

    #[test]
    fn test_length_of_empty_bytes() {
        todo!("test it");
    }
}
