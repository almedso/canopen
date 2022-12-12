//! The frame module
//!
//! It deals with all types of CANOpen frames.
//!
//! For every frame type, is provided:
//! - a builder function
//! - a format display
//! - a parser/converter from CAN-Frame
//! - means to inspect by public data elements
//!
//! The focus is on on the frame static structure. Any (dynamic) context context
//! information is not of interest in this module. All information that is
//! intrinsic to CANOpen frames is considered.
//!
//! # Examples
//!
//! Create an PDO
//!
//! ```
//! use col::CanOpenFrameBuilder;
//!
//! let builder = CanOpenFrameBuilder::default()
//!     .set_rtr(true)
//!     .pdo(0x1ef).unwrap()
//!     .payload(&[0x01, 0x02, 0x03]).unwrap();
//! let pdo_frame = builder.build();
//! println!("{}", pdo_frame);
//!
//! ```

mod builder;
pub use builder::*;

use super::CanOpenError;

pub mod sdo;
pub use sdo::*;

use core::convert::TryFrom;
use num_enum::TryFromPrimitive;

use std::fmt::Display;
use tokio_socketcan::CANFrame;

#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum FrameType {
    Nmt = 0b0000,           // Broadcast only
    SyncEmergency = 0b0001, // Sync = broadcast, Emergency = point to point
    Time = 0b0010,
    Tpdo1 = 0b0011,
    Rpdo1 = 0b0100,
    Tpdo2 = 0b0101,
    Rpdo2 = 0b0110,
    Tpdo3 = 0b0111,
    Rpdo3 = 0b1000,
    Tpdo4 = 0b1001,
    Rpdo4 = 0b1010,
    SdoTx = 0b1011, // 0x580 >> 7
    SdoRx = 0b1100, // 0x600 >> 7
    // Unused_1101, causes an error
    NmtErrorControl = 0b1110,
}

impl std::fmt::Display for FrameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            FrameType::Nmt => write!(f, "NMT  (000)")?,
            FrameType::SyncEmergency => write!(f, "SYEM (080)")?,
            FrameType::Time => write!(f, "TIME (100)")?,
            FrameType::Tpdo1 => write!(f, "TPDO (180)")?,
            FrameType::Rpdo1 => write!(f, "RPDO (200)")?,
            FrameType::Tpdo2 => write!(f, "TPDO (280)")?,
            FrameType::Rpdo2 => write!(f, "RPDO (300)")?,
            FrameType::Tpdo3 => write!(f, "TPDO (380)")?,
            FrameType::Rpdo3 => write!(f, "RPDO (400)")?,
            FrameType::Tpdo4 => write!(f, "TPDO (480)")?,
            FrameType::Rpdo4 => write!(f, "RPDO (500)")?,
            FrameType::SdoTx => write!(f, "TSDO (580)")?,
            FrameType::SdoRx => write!(f, "RSDO (600)")?,
            FrameType::NmtErrorControl => write!(f, "HBER (700)")?,
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UnspecificPayload {
    pub length: usize,
    pub data: [u8; 8],
}

impl std::fmt::Display for UnspecificPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.length > 0 && self.length < 9 {
            let data = &self.data[0..self.length as usize];
            for byte in data.iter() {
                write!(f, "{:02X} ", byte)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Payload {
    Unspecific(UnspecificPayload),
    SdoWithIndex(WithIndexPayload),
    SdoWithoutIndex(WithoutIndexPayload),
}

impl std::fmt::Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Payload::Unspecific(payload) => write!(f, "{}", payload)?,
            Payload::SdoWithIndex(payload) => write!(f, "{}", payload)?,
            Payload::SdoWithoutIndex(payload) => write!(f, "{}", payload)?,
        };
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct CANOpenFrame {
    _node_id: u8,
    _frame_type: FrameType,
    pub is_rtr: bool,
    pub payload: Payload,
}

impl std::fmt::Display for CANOpenFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}: ", self._frame_type,)?;

        match self._frame_type {
            FrameType::SdoTx | FrameType::SdoRx => {
                write!(f, "0x{:02X} \t", self._node_id)?;
            }
            FrameType::Tpdo1
            | FrameType::Tpdo2
            | FrameType::Tpdo3
            | FrameType::Tpdo4
            | FrameType::Rpdo1
            | FrameType::Rpdo2
            | FrameType::Rpdo3
            | FrameType::Rpdo4 => {
                write!(f, "0x{:02X} \t", self.cob_id())?;
            }
            _ => {
                write!(f, "0x{:02X} \t", self._node_id)?;
            }
        }
        write!(f, "{}", self.payload)?;
        Ok(())
    }
}

pub type CANOpenFrameResult = Result<CANOpenFrame, CanOpenError>;

impl CANOpenFrame {
    #[inline(always)]
    pub fn node_id(&self) -> u8 {
        self._node_id
    }

    #[inline(always)]
    pub fn frame_type(&self) -> FrameType {
        self._frame_type
    }

    #[inline(always)]
    pub fn cob_id(&self) -> u32 {
        const TYPE_START_BIT: u8 = 7;
        self._node_id as u32 + ((self._frame_type as u32) << TYPE_START_BIT)
    }
}

#[allow(clippy::from_over_into)]
impl Into<CANFrame> for CANOpenFrame {
    fn into(self) -> CANFrame {
        // every CANOpen frame is a CAN frame this conversion shall not cause an error
        match self.payload.clone() {
            Payload::Unspecific(p) => {
                CANFrame::new(self.cob_id(), &p.data[0..p.length], self.is_rtr, false).unwrap()
            }
            Payload::SdoWithIndex(p) => {
                let payload: SdoPayloadData = p.into();
                CANFrame::new(self.cob_id(), &payload, self.is_rtr, false).unwrap()
            }
            Payload::SdoWithoutIndex(p) => {
                let payload: SdoPayloadData = p.into();
                CANFrame::new(self.cob_id(), &payload, self.is_rtr, false).unwrap()
            }
        }
    }
}

impl TryFrom<CANFrame> for CANOpenFrame {
    type Error = CanOpenError;

    fn try_from(frame: CANFrame) -> Result<Self, Self::Error> {
        // CANOpenFrame::new_with_rtr(frame.id(), frame.data(), frame.is_rtr())
        // pub fn new_with_rtr(cob_id: u32, data: &[u8], is_rtr: bool) -> CANOpenFrameResult {
        let (_frame_type, _node_id) = extract_frame_type_and_node_id(frame.id())?;

        let length = frame.data().len();
        if length > 8 {
            return Err(CanOpenError::InvalidDataLength {
                length: frame.data().len(),
            });
        }
        let mut data = [0_u8; 8];
        data[..length].clone_from_slice(frame.data());

        let payload: Payload = match _frame_type {
            FrameType::SdoTx => {
                if length != 8 {
                    Payload::Unspecific(UnspecificPayload { data, length })
                } else if let Ok(p) = WithIndexPayload::parse_as_server_payload(data) {
                    Payload::SdoWithIndex(p)
                } else if let Ok(p) = WithoutIndexPayload::parse_as_server_payload(data) {
                    Payload::SdoWithoutIndex(p)
                } else {
                    Payload::Unspecific(UnspecificPayload { data, length })
                }
            }
            FrameType::SdoRx => {
                if length != 8 {
                    Payload::Unspecific(UnspecificPayload { data, length })
                } else if let Ok(p) = WithIndexPayload::parse_as_client_payload(data) {
                    Payload::SdoWithIndex(p)
                } else if let Ok(p) = WithoutIndexPayload::parse_as_client_payload(data) {
                    Payload::SdoWithoutIndex(p)
                } else {
                    Payload::Unspecific(UnspecificPayload { data, length })
                }
            }
            #[allow(clippy::wildcard_in_or_patterns)]
            FrameType::Tpdo1
            | FrameType::Tpdo2
            | FrameType::Tpdo3
            | FrameType::Tpdo4
            | FrameType::Rpdo1
            | FrameType::Rpdo2
            | FrameType::Rpdo3
            | FrameType::Rpdo4
            | _ => Payload::Unspecific(UnspecificPayload { data, length }),
        };

        Ok(CANOpenFrame {
            _node_id,
            _frame_type,
            is_rtr: frame.is_rtr(),
            payload,
        })
    }
}

fn extract_frame_type_and_node_id(cob_id: u32) -> Result<(FrameType, u8), CanOpenError> {
    if cob_id > 0x77F {
        // 0x77f is equivalent 11 bit
        return Err(CanOpenError::InvalidCobId { cob_id });
    }
    const TYPE_START_BIT: u8 = 7;
    const TYPE_MASK: u32 = 0b1111 << TYPE_START_BIT; // 4 bit length
    const NODE_ID_START_BIT: u8 = 0;
    const NODE_MASK: u32 = 0b111_1111 << NODE_ID_START_BIT; // 7 bit length
    let node_id: u8 = ((cob_id & NODE_MASK) >> NODE_ID_START_BIT) as u8;
    let frame_type = FrameType::try_from(((cob_id & TYPE_MASK) >> TYPE_START_BIT) as u8)
        .map_err(|_| CanOpenError::InvalidCobId { cob_id })?;
    Ok((frame_type, node_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_frame_type_and_node_id_ok_return() {
        assert_eq!(
            (FrameType::SdoRx, 0x01),
            extract_frame_type_and_node_id(0x601).unwrap()
        );
        assert_eq!(
            (FrameType::SdoTx, 0x01),
            extract_frame_type_and_node_id(0x581).unwrap()
        );
    }

    #[test]
    fn extract_frame_type_and_node_id_error_return() {
        let e = extract_frame_type_and_node_id(0xffff).unwrap_err();
        if let CanOpenError::InvalidCobId { cob_id } = e {
            assert_eq!(cob_id, 0xffff);
        } else {
            panic!("Not expected Error");
        }
    }

    #[test]
    fn convert_can_frame_into_can_open_frame_pdo() {
        let can_frame = CANFrame::new(0x1ef, &[0x01, 0x02, 0x03], true, false).unwrap();
        let can_open_frame = CANOpenFrame::try_from(can_frame).unwrap();
        assert_eq!(FrameType::Tpdo1, can_open_frame.frame_type());
        assert_eq!(0x6f, can_open_frame.node_id());
        assert_eq!(true, can_open_frame.is_rtr);
        assert_eq!(
            Payload::Unspecific(UnspecificPayload {
                data: [01_u8, 2, 3, 0, 0, 0, 0, 0],
                length: 3
            }),
            can_open_frame.payload
        );
    }

    #[test]
    fn convert_can_frame_into_sdo_with_index() {
        let can_frame = CANFrame::new(
            0x601,
            &[0x2F, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
            false,
            false,
        )
        .unwrap();
        let can_open_frame = CANOpenFrame::try_from(can_frame).unwrap();
        assert_eq!(FrameType::SdoRx, can_open_frame.frame_type());
        assert_eq!(0x1, can_open_frame.node_id());
        assert_eq!(false, can_open_frame.is_rtr);
        assert_eq!(
            Payload::SdoWithIndex(WithIndexPayload {
                cs: CommandSpecifier::Ccs(ClientCommandSpecifier::Download),
                size: CommandDataSize::OneByte,
                expedited_flag: true,
                index: 0x0201,
                subindex: 0x03,
                data: 0x07060504,
            }),
            can_open_frame.payload
        );
    }

    #[test]
    fn convert_can_frame_into_sdo_abort() {
        let can_frame = CANFrame::new(
            0x581,
            &[0x80, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
            false,
            false,
        )
        .unwrap();
        let can_open_frame = CANOpenFrame::try_from(can_frame).unwrap();
        assert_eq!(FrameType::SdoTx, can_open_frame.frame_type());
        assert_eq!(0x1, can_open_frame.node_id());
        assert_eq!(false, can_open_frame.is_rtr);
        assert_eq!(
            Payload::SdoWithIndex(WithIndexPayload {
                cs: CommandSpecifier::Scs(ServerCommandSpecifier::Abort),
                size: CommandDataSize::NotSet,
                expedited_flag: false,
                index: 0x0201,
                subindex: 0x03,
                data: 0x07060504,
            }),
            can_open_frame.payload
        );
    }
}
