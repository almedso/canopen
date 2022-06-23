use core::convert::TryFrom;
use failure::{Error, Fail};
use num_enum::TryFromPrimitive;

use enum_display_derive::*;
use std::fmt::Display;
use tokio_socketcan::CANFrame;

use super::super::sdo::SDOServerResponse;

#[derive(Debug, Fail)]
pub enum CANOpenFrameError {
    #[fail(display = "the COB-ID of this frame is invalid ({})", cob_id)]
    InvalidCOBID { cob_id: u32 },
    #[fail(display = "data length should not exceed 8 bytes ({} > 8)", length)]
    InvalidDataLength { length: usize },
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Display, Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum FrameType {
    Nmt = 0b0000,           // Broadcast only
    SyncEmergency = 0b0001, // Sync = broadcast, Emergency = point to point
    Time = 0b0010,          // Broadcast only
    Tpdo1 = 0b0011,         // Point to point
    Rpdo1 = 0b0100,         // Point to point
    Tpdo2 = 0b0101,         // Point to point
    Rpdo2 = 0b0110,         // Point to point
    Tpdo3 = 0b0111,         // Point to point
    Rpdo3 = 0b1000,         // Point to point
    Tpdo4 = 0b1001,         // Point to point
    Rpdo4 = 0b1010,         // Point to point
    SsdoTx = 0b1011,        // Point to point
    SsdoRx = 0b1100,        // Point to point
    // Unused_1101, causes an error
    NmtErrorControl = 0b1110, // Point to point
                              // Unused_1111, causes an error
}

#[derive(Debug, PartialEq)]
pub struct CANOpenFrame {
    _node_id: u8,
    _frame_type: FrameType,
    _length: u8,
    _data: [u8; 8],
    _is_rtr: bool,
}

impl std::fmt::Display for CANOpenFrame {
    fn fmt(
        self: &Self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{}: {:02X} [{}]\t",
            self._frame_type, self._node_id, self._length
        )?;

        match self._frame_type {
            FrameType::SsdoTx | FrameType::SsdoRx => {
                let sdo_response = SDOServerResponse::parse(self).map_err(|_| std::fmt::Error)?;
                write!(f, "{}", sdo_response);
            }
            _ => {
                for byte in self._data.iter() {
                    write!(f, "{:02X} ", byte);
                }
            }
        }

        Ok(())
    }
}

pub type CANOpenFrameResult = Result<CANOpenFrame, Error>;

impl CANOpenFrame {
    pub fn new(cob_id: u32, data: &[u8]) -> CANOpenFrameResult {
        CANOpenFrame::new_with_rtr(cob_id, data, false)
    }

    pub fn new_rtr(cob_id: u32, data: &[u8]) -> CANOpenFrameResult {
        CANOpenFrame::new_with_rtr(cob_id, data, true)
    }

    pub fn new_with_rtr(cob_id: u32, data: &[u8], is_rtr: bool) -> CANOpenFrameResult {
        let (_frame_type, _node_id) = extract_frame_type_and_node_id(cob_id)?;

        if data.len() > 8 {
            return Err(CANOpenFrameError::InvalidDataLength { length: data.len() }.into());
        }

        let mut frame = CANOpenFrame {
            _node_id,
            _frame_type,
            _length: data.len() as u8,
            _data: [0; 8],
            _is_rtr: is_rtr,
        };

        frame._data[..data.len()].clone_from_slice(&data[..]);
        Ok(frame)
    }

    #[inline(always)]
    pub fn node_id(self: &Self) -> u8 {
        self._node_id
    }

    #[inline(always)]
    pub fn frame_type(self: &Self) -> FrameType {
        self._frame_type
    }

    #[inline(always)]
    pub fn length(self: &Self) -> u8 {
        self._length
    }

    #[inline(always)]
    pub fn data(self: &Self) -> [u8; 8] {
        self._data
    }

    #[inline(always)]
    pub fn is_rtr(self: &Self) -> bool {
        self._is_rtr
    }

    #[inline(always)]
    pub fn cob_id(self: &Self) -> u32 {
        const TYPE_START_BIT: u8 = 7;
        self._node_id as u32 + ((self._frame_type as u32) << TYPE_START_BIT)
    }
}

impl Into<CANFrame> for CANOpenFrame {
    fn into(self) -> CANFrame {
        // every CANOpen frame is a CAN frame this conversion shall not cause an error
        CANFrame::new(self.cob_id(), &self.data(), self.is_rtr(), false).unwrap()
    }
}

impl TryFrom<CANFrame> for CANOpenFrame {
    type Error = Error;
    fn try_from(frame: CANFrame) -> Result<Self, Self::Error> {
        CANOpenFrame::new_with_rtr(frame.id(), frame.data(), frame.is_rtr())
    }
}

fn extract_frame_type_and_node_id(cob_id: u32) -> Result<(FrameType, u8), CANOpenFrameError> {
    if cob_id > 0x77F {
        // 0x77f is equivalent 11 bit
        return Err(CANOpenFrameError::InvalidCOBID { cob_id }.into());
    }
    const TYPE_START_BIT: u8 = 7;
    const TYPE_MASK: u32 = 0b1111 << TYPE_START_BIT; // 4 bit length
    const NODE_ID_START_BIT: u8 = 0;
    const NODE_MASK: u32 = 0b111_1111 << NODE_ID_START_BIT; // 7 bit length
    let node_id: u8 = ((cob_id & NODE_MASK) >> NODE_ID_START_BIT) as u8;
    let frame_type = FrameType::try_from(((cob_id & TYPE_MASK) >> TYPE_START_BIT) as u8)
        .map_err(|_| CANOpenFrameError::InvalidCOBID { cob_id }.into())?;
    Ok((frame_type, node_id))
}
