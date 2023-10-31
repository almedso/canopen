use crate::{CANOpenFrame, ObjectDictionary, SdoServer};
use col::NodeStateMachine;
use tokio::{runtime, spawn};
use tokio_socketcan::{CANFrame, CANSocket};

fn main() {
    const node_id: u8 = 10;
    let od = Rc::new(ObjectDictionary::new(0x12, 0x33));

    let sdo_server = async {
        let mut can_socket = match CANSocket::open("can0") {
            Ok(socket) => socket,
            Err(error) => {
                error!("Error opening {}: {}", cli.interface, error);
                quit::with_code(1);
                Err(error)
            }
        };
        let server = SdoServer::new(node_id, can_socket, od);
        while let Some(Ok(frame)) = self.can_socket.next().await {
            let frame = CANOpenFrame::try_from(frame)?;
            server.process_complete_sdo_request(frame).await;
        }
    };

    let node_state_machine = async {
        let mut can_socket = match CANSocket::open("can0") {
            Ok(socket) => socket,
            Err(error) => {
                error!("Error opening {}: {}", cli.interface, error);
                quit::with_code(1);
                Err(error)
            }
        };
        let mut sm = NodeStateMachine::new(can_socket, od);
        loop {
            sm.operate().await;
        }
    };

    let my_future = async {
        s = spawn(sdo_server);
        n = spawn(node_state_machine);
        s.await();
        n.await();
    };

    let rt = runtime::Runtime::new().unwrap();
    rt.block_on(my_future) // tokio async runtime
}

//  https://stackoverflow.com/questions/66863385/how-can-i-use-tokio-to-trigger-a-function-every-period-or-interval-in-seconds

