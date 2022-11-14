use super::*;
use crate::frame::sdo::builder::SdoFrameBuilder;

#[derive(Clone, Copy, Default)]
pub struct CanOpenFrameBuilder {
    is_rtr: bool,
}

impl CanOpenFrameBuilder {
    pub fn sdo_request(node_id: u8) -> Result<SdoFrameBuilder, CanOpenError> {
        if node_id > 0x7f {
            return Err(CanOpenError::InvalidNodeId { node_id });
        }
        Ok(SdoFrameBuilder {
            node_id,
            frame_type: FrameType::SdoRx, // 0x600 (lower prio)
        })
    }

    pub fn sdo_response(node_id: u8) -> Result<SdoFrameBuilder, CanOpenError> {
        if node_id > 0x7f {
            return Err(CanOpenError::InvalidNodeId { node_id });
        }
        Ok(SdoFrameBuilder {
            node_id,
            frame_type: FrameType::SdoTx, // 0x580 (higher prio)
        })
    }

    pub fn pdo(self, cob_id: u32) -> Result<UnspecificPayloadBuilder, CanOpenError> {
        let (frame_type, node_id) = extract_frame_type_and_node_id(cob_id)?;

        match frame_type {
            FrameType::Tpdo1
            | FrameType::Tpdo2
            | FrameType::Tpdo3
            | FrameType::Tpdo4
            | FrameType::Rpdo1
            | FrameType::Rpdo2
            | FrameType::Rpdo3
            | FrameType::Rpdo4 => (),
            _ => {
                return Err(CanOpenError::InvalidCobId { cob_id });
            }
        }

        Ok(UnspecificPayloadBuilder {
            frame_type,
            node_id,
            is_rtr: self.is_rtr,
            length: 0,
            data: [0; 8],
        })
    }

    pub fn set_rtr(mut self, rtr_flag: bool) -> Self {
        self.is_rtr = rtr_flag;
        self
    }
}

#[derive(Clone, Copy)]
pub struct UnspecificPayloadBuilder {
    node_id: u8,
    frame_type: FrameType,
    is_rtr: bool,
    length: usize,
    data: [u8; 8],
}

impl UnspecificPayloadBuilder {
    /// Set the payload
    ///
    /// Any size of the slice equal or less then 8 bytes are accepted
    /// larger size leads to an error return
    pub fn payload(mut self, data: &[u8]) -> Result<Self, CanOpenError> {
        self.length = data.len();
        if self.length > 8 {
            return Err(CanOpenError::InvalidDataLength {
                length: (self.length),
            });
        }
        self.data[..data.len()].clone_from_slice(data);
        Ok(self)
    }

    /// Actually build the CANOpen frame
    pub fn build(&self) -> CANOpenFrame {
        let payload = UnspecificPayload {
            length: self.length,
            data: self.data,
        };
        CANOpenFrame {
            _node_id: self.node_id,
            _frame_type: self.frame_type,
            is_rtr: self.is_rtr,
            payload: Payload::Unspecific(payload),
        }
    }
}

// pub fn sync_frame() -> CANOpenFrameResult {
//     CANOpenFrame::new(0x080u32, &[])
// }

// pub fn set_mode_frame(id: u8, mode: Mode) -> CANOpenFrameResult {
//     let mode_value = match mode {
//         Mode::Operational => 1,
//         Mode::Stop => 2,
//         Mode::PreOperational => 80,
//         Mode::ResetApplication => 81,
//         Mode::ResetCommunication => 82,
//     };

//     CANOpenFrame::new(0x000u32, &[mode_value, id])
// }

// pub fn set_all_mode_frame(mode: Mode) -> CANOpenFrameResult {
//     set_mode_frame(0u8, mode)
// }

// pub fn request_mode_frame(id: u8) -> CANOpenFrameResult {
//     CANOpenFrame::new_with_rtr(0x700u32 + u32::from(id), &[], true)
// }

// pub fn guarding_frame(id: u8, state: State, toggle: bool) -> CANOpenFrameResult {
//     let mut state_value = match state {
//         State::BootUp => 0x00,
//         State::Operational => 0x05,
//         State::Stopped => 0x04,
//         State::PreOperational => 0x7F,
//         _ => panic!("will not send unknown state"),
//     };

//     if toggle {
//         state_value |= 0x80;
//     }

//     CANOpenFrame::new(0x700u32 + u32::from(id), &[state_value])
// }

// pub fn heartbeat_frame(id: u8, state: State) -> CANOpenFrameResult {
//     guarding_frame(id, state, false)
// }

// pub fn emergency_frame(
//     id: u8,
//     error_code: u16,
//     error_register: u8,
//     data: [u8; 5],
// ) -> CANOpenFrameResult {
//     CANOpenFrame::new(
//         0x80u32 + u32::from(id),
//         &[
//             error_code.lo(),
//             error_code.hi(),
//             error_register,
//             data[0],
//             data[1],
//             data[2],
//             data[3],
//             data[4],
//         ],
//     )
// }

// pub fn get_mode(message: &CANOpenFrame) -> State {
//     match message.data()[0] & 0x80 {
//         0x04 => State::Stopped,
//         0x05 => State::Operational,
//         0x7F => State::PreOperational,
//         _ => State::UnknownState,
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdo_builder_ok() {
        let sut = CanOpenFrameBuilder::default();
        let _expected_builder = sut.pdo(0x1ef).unwrap();
    }
}
