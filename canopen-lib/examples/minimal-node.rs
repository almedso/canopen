use tokio_socketcan::{CANFrame, CANSocket};
use tokio;
use crate::{ ObjectDictionary, SdoServer CANOpenFrame};

fn main() {
    let my_future = async {
        let mut can_socket = match CANSocket::open("can0") {
            Ok(socket) => socket,
            Err(error) => {
                error!("Error opening {}: {}", cli.interface, error);
                quit::with_code(1);
            }
        };
        const node_id: u8 = 10;
        let od = Rc::new(ObjectDictionary::new(0x12, 0x33));
        let server = SdoServer::new(node_id, can_socket, od);
        while let Some(Ok(frame)) = self.can_socket.next().await {
            let frame = CANOpenFrame::try_from(frame)?;
            server.process_complete_sdo_request(frame).await;
        }
    }
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(my_future) // tokio async runtime
}

