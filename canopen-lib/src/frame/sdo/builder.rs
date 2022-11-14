//! # SDO frame builders
//!
//! ## Examples
//!
//! Download request to (write) data to a remote node.
//! The Remote node is the SDO server
//!
//! ```
//! use col::CanOpenFrameBuilder;
//!
//! let builder = CanOpenFrameBuilder::sdo_request(0x30).unwrap()  // sdo client -> sdo server
//!     .with_index(0x2000, 0x00)  // object at index 0x2000 and subindex 00
//!     .download_one_byte(0x03);
//! let sdo_frame = builder.build();
//! println!("{}", sdo_frame);
//!
//! ```
//!
//! Download response by the remote node - the Remote node is the SDO server
//!
//! ```
//! use col::CanOpenFrameBuilder;
//!
//! let builder = CanOpenFrameBuilder::sdo_response(0x30).unwrap()  // sdo server -> sdo client
//!     .with_index(0x2000, 0x00)  // object at index 0x2000 and subindex 00
//!     .download_response();
//! let sdo_frame = builder.build();
//! println!("{}", sdo_frame);
//!
//! ```
//!
//! Upload response to (read) data  remote node.
//! Segmented transfer
//! ```
//! use col::CanOpenFrameBuilder;
//!
//! let builder = CanOpenFrameBuilder::sdo_request(0x30).unwrap()  // sdo client -> sdo server
//!     .with_index(0x2000, 0x00)  // object at index 0x2000 and subindex 00
//!     .download(&[0x01, 0x02, 0x03]);
//! let sdo_frame = builder.build();
//! println!("{}", sdo_frame);
//! let toggle = false;
//!
//! let mut builder = CanOpenFrameBuilder::sdo_response(0x30).unwrap()  // sdo client -> sdo server
//!     .without_index()
//!     .upload_response(&[0x01, 0x02, 0x03]);  // maximum 7 bytes
//! let sdo_frame = builder.build();
//!
//! println!("{}", sdo_frame);
//!
//! ```

use crate::frame::sdo::*;
use crate::CANOpenFrame;

#[derive(Clone)]
pub struct SdoFrameBuilder {
    pub node_id: u8,
    pub frame_type: FrameType,
}

impl SdoFrameBuilder {
    pub fn with_index(&self, index: u16, subindex: u8) -> WithIndexFrameBuilder {
        let command_specifier = match self.frame_type {
            FrameType::SdoRx => CommandSpecifier::Ccs(ClientCommandSpecifier::Unspecified),
            _ => CommandSpecifier::Scs(ServerCommandSpecifier::Unspecified),
        };
        WithIndexFrameBuilder {
            node_id: self.node_id,
            frame_type: self.frame_type,
            index,
            subindex,
            command_specifier,
            size: CommandDataSize::NotSet,
            expedited_flag: false,
            data: 0,
        }
    }
    pub fn without_index(&self) -> WithoutIndexFrameBuilder {
        let command_specifier = match self.frame_type {
            FrameType::SdoRx => CommandSpecifier::Ccs(ClientCommandSpecifier::Unspecified),
            _ => CommandSpecifier::Scs(ServerCommandSpecifier::Unspecified),
        };
        WithoutIndexFrameBuilder {
            node_id: self.node_id,
            frame_type: self.frame_type,
            command_specifier,
            toggle: true, // gets toggled at the first build call before building
            length_of_empty_bytes: None,
            data: [0_u8; 7],
        }
    }
}


#[derive(Clone)]
pub struct WithIndexFrameBuilder {
    node_id: u8,
    frame_type: FrameType,
    command_specifier: CommandSpecifier,
    size: CommandDataSize,
    expedited_flag: bool,
    index: u16,
    subindex: u8,
    data: u32,
}

impl WithIndexFrameBuilder {
    pub fn build(&self) -> CANOpenFrame {
        CANOpenFrame {
            _node_id: self.node_id,
            _frame_type: self.frame_type,
            is_rtr: false,
            payload: Payload::SdoWithIndex(WithIndexPayload {
                cs: self.command_specifier.clone(),
                size: self.size.clone(),
                expedited_flag: self.expedited_flag,
                index: self.index,
                subindex: self.subindex,
                data: self.data,
            }),
        }
    }

    pub fn abort(mut self, abort_code: SDOAbortCode) -> Self {
        // 0x80  command byte
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::Abort);
        self.size = CommandDataSize::NotSet;
        self.data = abort_code.into();
        self.expedited_flag = false;
        self
    }

    pub fn download_one_byte(mut self, data: u8) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::InitiateDownload);
        self.size = CommandDataSize::OneByte;
        self.data = data as u32 & 0xff_u32;
        self.expedited_flag = true;
        self
    }

    pub fn download_two_bytes(mut self, data: u16) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::InitiateDownload);
        self.size = CommandDataSize::TwoBytes;
        self.data = data as u32 & 0xffff_u32;
        self.expedited_flag = true;
        self
    }

    pub fn download_four_bytes(mut self, data: u32) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::InitiateDownload);
        self.size = CommandDataSize::FourBytes;
        self.data = data;
        self.expedited_flag = true;
        self
    }

    pub fn download_response(mut self) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::DownloadResponse);
        self.size = CommandDataSize::NotSet;
        self.expedited_flag = false;
        self.data = 0;
        self
    }

    pub fn upload_request(mut self) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::NotSet; // all zero
        self.expedited_flag = false;
        self.data = 0;
        self
    }

    pub fn upload_one_byte_expedited_response(mut self, data: u8) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::OneByte;
        self.data = data as u32 & 0xff_u32;
        self.expedited_flag = true;
        self
    }

    pub fn upload_two_bytes_response_expedited_response(mut self, data: u16) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::TwoBytes;
        self.data = data as u32 & 0xffff_u32;
        self.expedited_flag = true;
        self
    }

    pub fn upload_three_bytes_expedited_response(mut self, data: u32) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::ThreeBytes;
        self.data = data & 0x00ffffff_u32;
        self.expedited_flag = true;
        self
    }

    pub fn upload_four_bytes_expedited_response(mut self, data: u32) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::FourBytes;
        self.data = data;
        self.expedited_flag = true;
        self
    }

    pub fn upload_segmented_response(mut self, length: u32) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::UploadResponse);
        self.size = CommandDataSize::FourBytes;
        self.data = length;
        self.expedited_flag = false;  // this indicates that next frames will be segmented payload
        self
    }

    pub fn download(mut self, data: &[u8]) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::InitiateDownload);
        match data.len() {
            4 => {
                self.size = CommandDataSize::FourBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
                self.data += (data[2] as u32) << 16;
                self.data += (data[3] as u32) << 24;
            }
            3 => {
                self.size = CommandDataSize::ThreeBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
                self.data += (data[2] as u32) << 16;
            }
            2 => {
                self.size = CommandDataSize::TwoBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
            }
            1 => {
                self.size = CommandDataSize::OneByte;
                self.data = data[0] as u32;
            }
            0 => {
                self.size = CommandDataSize::NotSet;
                self.data = 0;
            }
            _ => panic!(
                "More than four byte data is not allowed for a sdo frame with index/subindex"
            ),
        }
        self.expedited_flag = true;
        self
    }

    pub fn upload_expedited_response(mut self, data: &[u8]) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::InitiateUpload);
        match data.len() {
            4 => {
                self.size = CommandDataSize::FourBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
                self.data += (data[2] as u32) << 16;
                self.data += (data[3] as u32) << 24;
            }
            3 => {
                self.size = CommandDataSize::ThreeBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
                self.data += (data[2] as u32) << 16;
            }
            2 => {
                self.size = CommandDataSize::TwoBytes;
                self.data = data[0] as u32;
                self.data += (data[1] as u32) << 8;
            }
            1 => {
                self.size = CommandDataSize::OneByte;
                self.data = data[0] as u32;
            }
            0 => {
                self.size = CommandDataSize::NotSet;
                self.data = 0;
            }
            _ => panic!(
                "More than four byte data is not allowed for a sdo frame with index/subindex"
            ),
        }
        self.expedited_flag = true;
        self
    }
}

#[derive(Clone)]
pub struct WithoutIndexFrameBuilder {
    node_id: u8,
    frame_type: FrameType,
    command_specifier: CommandSpecifier,
    // has some value if the sized flag is set
    length_of_empty_bytes: Option<u8>,
    toggle: bool,
    data: [u8; 7],
}

impl WithoutIndexFrameBuilder {

    pub fn build(&mut self) -> CANOpenFrame {
        self.toggle = ! self.toggle;
        CANOpenFrame {
            _node_id: self.node_id,
            _frame_type: self.frame_type,
            is_rtr: false,
            payload: Payload::SdoWithoutIndex(WithoutIndexPayload {
                cs: self.command_specifier.clone(),
                toggle: self.toggle,
                length_of_empty_bytes: self.length_of_empty_bytes.clone(),
                data: self.data.clone(),
            }),
        }
    }

    pub fn upload_request(mut self) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::UploadSegment);
        self.length_of_empty_bytes = Some(0);
        self.data = [0_u8; 7];
        // do not toggle the toggle bit - this is done by build
        self
    }

    pub fn upload_response(mut self, data: &[u8]) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::DownloadSegment);
        let len = data.len();
        self.length_of_empty_bytes = if len < 7 {
            Some (7u8 - (len as u8))
        } else {
            None
        };
        for i in 0..7 {
            if i < len {
                self.data[i] = data[i];
            } else {
                self.data[i] = 0;
            }
        }
        // do not toggle the toggle bit - this is done by build
        self
    }

    pub fn download_request(mut self, data: &[u8]) -> Self {
        self.command_specifier = CommandSpecifier::Ccs(ClientCommandSpecifier::DownloadSegment);
        let len = data.len();
        self.length_of_empty_bytes = if len < 7 {
            Some (7u8 - (len as u8))
        } else {
            None
        };
        for i in 0..7 {
            if i < len {
                self.data[i] = data[i];
            } else {
                self.data[i] = 0;
            }
        }
        // do not toggle the toggle bit - this is done by build
        self
    }

    pub fn download_response(mut self) -> Self {
        self.command_specifier = CommandSpecifier::Scs(ServerCommandSpecifier::DownloadResponse);
        self.length_of_empty_bytes = Some(0);
        self.data = [0_u8; 7];
        // do not toggle the toggle bit - this is done by build
        self
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sdo_abort_response() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .abort(SDOAbortCode::InvalidSequenceNumber);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x80, 0x22, 0x11, 0x33, 0x03, 0x00, 0x04, 0x05]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_download_one_byte() {
        let builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .download_one_byte(0x44);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x2F, 0x22, 0x11, 0x33, 0x44, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_download_two_bytes() {
        let builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .download_two_bytes(0x5544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x2B, 0x22, 0x11, 0x33, 0x44, 0x55, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_download_four_bytes() {
        let builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .download_four_bytes(0x77665544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x23, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x77]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_request() {
        let builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_request();
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x40, 0x22, 0x11, 0x33, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_download_response() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .download_response();
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x60, 0x22, 0x11, 0x33, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_one_byte() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_one_byte_expedited_response(0x44);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x4F, 0x22, 0x11, 0x33, 0x44, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_two_bytes() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_two_bytes_response_expedited_response(0x5544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x4B, 0x22, 0x11, 0x33, 0x44, 0x55, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_three_bytes() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_three_bytes_expedited_response(0x77665544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x47, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x00]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_four_bytes() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_four_bytes_expedited_response(0x77665544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x43, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x77]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload_segmented_response() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_segmented_response(0x77665544);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0x41, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x77]
        );
        assert_eq!(expected.id(), 0x581);
        assert_eq!(expected.is_rtr(), false);
    }

    #[test]
    fn builder_sdo_upload() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33);
        {
            let builder = builder.clone().upload_expedited_response(&[0x44, 0x55, 0x66, 0x77]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x43, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x77]
            );
            assert_eq!(expected.id(), 0x581);
            assert_eq!(expected.is_rtr(), false);
        }
        {
            let builder = builder.clone().upload_expedited_response(&[0x44, 0x55, 0x66]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x47, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x00]
            );
        }
        {
            let builder = builder.clone().upload_expedited_response(&[0x44, 0x55]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x4B, 0x22, 0x11, 0x33, 0x44, 0x55, 0x00, 0x00]
            );
        }
        {
            let builder = builder.clone().upload_expedited_response(&[0x44]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x4F, 0x22, 0x11, 0x33, 0x44, 0x00, 0x00, 0x00]
            );
        }
        {
            let builder = builder.clone().upload_expedited_response(&[]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x42, 0x22, 0x11, 0x33, 0x00, 0x00, 0x00, 0x00]
            );
        }
    }

    #[test]
    #[should_panic]
    fn builder_sdo_upload_data_too_big() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .upload_expedited_response(&[0x44, 0x55, 0x66, 0x77, 0x88]);
    }

    #[test]
    fn builder_sdo_download() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33);
        {
            let builder = builder.clone().download(&[0x44, 0x55, 0x66, 0x77]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x23, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x77]
            );
            assert_eq!(expected.id(), 0x581);
            assert_eq!(expected.is_rtr(), false);
        }
        {
            let builder = builder.clone().download(&[0x44, 0x55, 0x66]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x27, 0x22, 0x11, 0x33, 0x44, 0x55, 0x66, 0x00]
            );
        }
        {
            let builder = builder.clone().download(&[0x44, 0x55]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x2B, 0x22, 0x11, 0x33, 0x44, 0x55, 0x00, 0x00]
            );
        }
        {
            let builder = builder.clone().download(&[0x44]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x2F, 0x22, 0x11, 0x33, 0x44, 0x00, 0x00, 0x00]
            );
        }
        {
            let builder = builder.clone().download(&[]);
            let expected: CANFrame = builder.build().into();
            assert_eq!(
                expected.data(),
                [0x22, 0x22, 0x11, 0x33, 0x00, 0x00, 0x00, 0x00]
            );
        }
    }

    #[test]
    #[should_panic]
    fn builder_sdo_download_data_too_big() {
        let builder = CanOpenFrameBuilder::sdo_response(1)
            .unwrap()
            .with_index(0x1122, 0x33)
            .download(&[0x44, 0x55, 0x66, 0x77, 0x88]);
    }

    #[test]
    fn builder_segmented_upload_request() {
        let mut builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .without_index().upload_request();
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0110_0001, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0111_0001, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0110_0001, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }


    #[test]
    fn builder_segmented_upload_response() {
        let mut builder = CanOpenFrameBuilder::sdo_request(1)
            .unwrap()
            .without_index().upload_response(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0000_0000, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
        );
        let mut builder = builder.upload_response(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0001_0000, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
        );
        let mut builder = builder.upload_response(&[0x01, 0x02, 0x03, 0x04]);
        let expected: CANFrame = builder.build().into();
        assert_eq!(
            expected.data(),
            [0b0000_0111, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00]
        );
        assert_eq!(expected.id(), 0x601);
        assert_eq!(expected.is_rtr(), false);
    }
}
