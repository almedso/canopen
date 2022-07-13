use futures_util::StreamExt;
use tokio_socketcan::{CANFrame, CANSocket};

use col::sdo::SDOServerResponse;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ValueType {
    None,
    U8,
    U16,
    U32,
    U64,
}

pub async fn write_remote_object(
    can_socket: &mut CANSocket,
    node: u8,
    index: u16,
    subindex: u8,
    value_type: ValueType,
    value: u32,
) -> () {
    const SDO_RECEIVE: u32 = 0x600;
    let frame: CANFrame = match value_type {
        ValueType::U8 => {
            col::download_1_byte_frame(node, SDO_RECEIVE, index, subindex, value as u8)
                .unwrap()
                .into()
        }
        ValueType::U16 => {
            let buffer: [u8; 2] = [
                // little endian encoded
                (value & 0xff_u32) as u8,
                ((value >> 8) & 0xff_u32) as u8,
            ];
            col::download_2_bytes_frame(node, SDO_RECEIVE, index, subindex, buffer)
                .unwrap()
                .into()
        }
        ValueType::U32 => {
            let buffer: [u8; 4] = [
                // little endian encoded
                (value & 0xff_u32) as u8,
                ((value >> 8) & 0xff_u32) as u8,
                ((value >> 16) & 0xff_u32) as u8,
                ((value >> 24) & 0xff_u32) as u8,
            ];
            col::download_4_bytes_frame(node, SDO_RECEIVE, index, subindex, buffer)
                .unwrap()
                .into()
        }
        _ => {
            panic!("{:?} is not supported for this SDO", value_type);
        }
    };

    match match can_socket.write_frame(frame) {
        Ok(x) => x,
        Err(error) => {
            panic!("Error instancing write: {}", error);
        }
    }
    .await
    {
        Ok(_) => (),
        Err(error) => {
            panic!("Error writing: {}", error);
        }
    }

    // read the response
    while let Some(Ok(frame)) = can_socket.next().await {
        match col::CANOpenFrame::try_from(frame) {
            Ok(frame) => {
                if frame.node_id() == node && frame.frame_type() == col::frame::FrameType::SsdoTx {
                    break;
                }
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
}

pub async fn read_remote_object(
    can_socket: &mut CANSocket,
    node: u8,
    index: u16,
    subindex: u8,
    expected_value: u32,
) -> () {
    const SDO_RECEIVE: u32 = 0x600;
    let frame: CANFrame = col::upload_request_frame(node, SDO_RECEIVE, index, subindex)
        .unwrap()
        .into();
    match match can_socket.write_frame(frame) {
        Ok(x) => x,
        Err(error) => {
            panic!("Error instancing write: {}", error);
        }
    }
    .await
    {
        Ok(_) => (),
        Err(error) => {
            panic!("Error writing: {}", error);
        }
    }

    // read the response
    while let Some(Ok(frame)) = can_socket.next().await {
        match col::CANOpenFrame::try_from(frame) {
            Ok(frame) => {
                if frame.node_id() == node && frame.frame_type() == col::frame::FrameType::SsdoTx {
                    let sdo_response = SDOServerResponse::parse(&frame)
                        .map_err(|x| panic!("{}", x))
                        .unwrap();
                    if sdo_response.index == index && sdo_response.subindex == subindex {
                        if sdo_response.data as u32 == expected_value {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
}
