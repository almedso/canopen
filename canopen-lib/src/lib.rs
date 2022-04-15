
use core::convert::TryFrom;
use num_enum::TryFromPrimitive;

#[macro_use]
extern crate enum_display_derive;

use std::fmt::Display;

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
    Unused_1101,
    NmtErrorControl = 0b1110,  // Point to point
    Unused_1111,
}

pub struct CANOpenFrame {
    id: u8,
    frame_type: FrameType,
    index: u16,
    sub_index: u8,
    payload: u32
}

pub fn extract_node_and_type(can_id: u32) -> ( FrameType, u8) {
    const TYPE_START_BIT: u8 = 7;
    const TYPE_MASK: u32 = 0b1111 << TYPE_START_BIT; // 4 bit length
    const NODE_ID_START_BIT: u8 = 0;
    const NODE_MASK: u32 = 0b111_1111 << NODE_ID_START_BIT; // 7 bit length
    let node_id: u8 = ((can_id & NODE_MASK) >> NODE_ID_START_BIT) as u8;
    let frame_type = FrameType::try_from(((can_id & TYPE_MASK) >> TYPE_START_BIT) as u8).unwrap();
    ( frame_type, node_id )
}
