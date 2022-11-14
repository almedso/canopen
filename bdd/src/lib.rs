use futures_util::StreamExt;
use tokio_socketcan::{CANFrame, CANSocket};

use col::{CANOpenFrame, CanOpenFrameBuilder, FrameType, Payload};

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
) {
    let builder = CanOpenFrameBuilder::sdo_request(node)
        .unwrap()
        .with_index(index, subindex);
    let frame: CANFrame = match value_type {
        ValueType::U8 => builder.download_one_byte(value as u8).build().into(),
        ValueType::U16 => builder.download_two_bytes(value as u16).build().into(),
        ValueType::U32 => builder.download_four_bytes(value as u32).build().into(),
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
                if frame.node_id() == node && frame.frame_type() == col::frame::FrameType::SdoTx {
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
) {
    let frame = CanOpenFrameBuilder::sdo_request(node)
        .unwrap()
        .with_index(index, subindex)
        .upload_request()
        .build()
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
        match CANOpenFrame::try_from(frame) {
            Ok(frame) => {
                if frame.node_id() == node && frame.frame_type() == FrameType::SdoTx {
                    if let Payload::SdoWithIndex(payload) = frame.payload {
                        if payload.index == index
                            && payload.subindex == subindex
                            && payload.data == expected_value
                        {
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
