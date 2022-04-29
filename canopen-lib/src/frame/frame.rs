use failure::{Error, Fail};
use core::convert::TryFrom;
use num_enum::TryFromPrimitive;

use enum_display_derive::*;
use std::fmt::Display;
use tokio_socketcan::CANFrame;

#[derive(Debug, Fail)]
pub enum CANOpenFrameError {
    #[fail(display = "the COB-ID of this frame is invalid ({})", cob_id)]
    InvalidCOBID { cob_id: u32 },
    #[fail(
        display = "data length should not exceed 8 bytes ({} > 8)",
        length
    )]
    InvalidDataLength { length: usize },
}

#[derive(Debug, PartialEq)]
pub struct CANOpenFrame {
    pub cob_id: u32,
    pub length: u8,
    pub data: [u8; 8],
    pub is_rtr: bool,
}

impl std::fmt::Display for CANOpenFrame {
    fn fmt(
        self: &Self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:03X} [{}]\t", self.cob_id, self.length)?;

        for byte in self.data.iter() {
            write!(f, "{:02X} ", byte);
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
        if cob_id > 0x77F {
            return Err(CANOpenFrameError::InvalidCOBID { cob_id }.into());
        }
        if data.len() > 8 {
            return Err(CANOpenFrameError::InvalidDataLength { length: data.len() }.into());
        }

        let mut frame = CANOpenFrame {
            cob_id,
            length: data.len() as u8,
            data: [0; 8],
            is_rtr,
        };

        frame.data[..data.len()].clone_from_slice(&data[..]);

        Ok(frame)
    }

    pub fn cob_id(self: &Self) -> u32 {
        self.cob_id
    }

    pub fn length(self: &Self) -> u8 {
        self.length
    }

    pub fn data(self: &Self) -> &[u8; 8] {
        &self.data
    }

    pub fn is_rtr(self: &Self) -> bool {
        self.is_rtr
    }
}


impl Into<CANFrame> for CANOpenFrame {
    fn into(self) -> CANFrame {
        // every CANOpen frame is a CAN frame this conversion shall not cause an error
        CANFrame::new(self.cob_id(), self.data(), self.is_rtr(), false).unwrap() 
    }
}

// impl TryFrom<CANFrame> for CANOpenFrame {
//     type Error = ;
//     fn try_from(frame: CANFrame) -> Result<Self, Self::Error> {

//     }
// }

#[allow(non_camel_case_types, dead_code)]
#[derive(Display)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum FrameType {
    Nmt = 0b0000,  // Broadcast only
    SyncEmergency = 0b0001,  // Sync = broadcast, Emergency = point to point
    Time = 0b0010,  // Broadcast only
    Tpdo1 = 0b0011,  // Point to point
    Rpdo1 = 0b0100,  // Point to point
    Tpdo2 = 0b0101,  // Point to point
    Rpdo2 = 0b0110,  // Point to point
    Tpdo3 = 0b0111,  // Point to point
    Rpdo3 = 0b1000,  // Point to point
    Tpdo4 = 0b1001,  // Point to point
    Rpdo4 = 0b1010,  // Point to point
    SsdoTx = 0b1011,  // Point to point
    SsdoRx = 0b1100,  // Point to point
    // Unused_1101, causes an error
    NmtErrorControl = 0b1110,  // Point to point
    // Unused_1111, causes an error
}

pub fn extract_frame_type_and_node_id(cob_id: u32) -> Result <( FrameType, u8), CANOpenFrameError> {
    if cob_id > 0x77F { // 0x77f is equivalent 11 bit 
        return Err(CANOpenFrameError::InvalidCOBID { cob_id }.into());
    }
    const TYPE_START_BIT: u8 = 7;
    const TYPE_MASK: u32 = 0b1111 << TYPE_START_BIT; // 4 bit length
    const NODE_ID_START_BIT: u8 = 0;
    const NODE_MASK: u32 = 0b111_1111 << NODE_ID_START_BIT; // 7 bit length
    let node_id: u8 = ((cob_id & NODE_MASK) >> NODE_ID_START_BIT) as u8;
    let frame_type = FrameType::try_from(((cob_id & TYPE_MASK) >> TYPE_START_BIT) as u8)
        .map_err(|_| CANOpenFrameError::InvalidCOBID { cob_id }.into())?;
    Ok (( frame_type, node_id ))
}
