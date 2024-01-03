use clap::Parser;
use col::{CANOpenFrame, CanOpenError, ObjectDictionaryBuilder, SdoServer};
use env_logger;
use log::{debug, error, info};
// use col::NodeStateMachine;
// use futures_util::StreamExt;
use std::sync::Arc;
use tokio::{runtime, spawn};
use tokio_socketcan::CANSocket;

use std::future::Future;

fn set_return_type<T, F: Future<Output = T>>(_arg: &F) {}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// CAN interface to read from, write to
    #[clap(short, long, default_value_t = String::from("vcan0"))]
    interface: String,
}

#[quit::main]
fn main() {
    env_logger::init();
    const NODE_ID: u8 = 10;
    let od = Arc::new(
        ObjectDictionaryBuilder::new(0x12, 0x33)
            .device_name("Example node")
            .hardware_version("Rev.00")
            .serial_number(0x00_01_00_00)
            .software_version("0.1.0")
            .build(NODE_ID),
    );

    let sdo_server = async {
        let cli = Cli::parse();
        let can_socket: CANSocket = CANSocket::open(&cli.interface).map_err(|err| {
            error!("Error opening {}: {}", cli.interface, err);
            quit::with_code(1);
        })?;

        info!("Instanciate SDO server");
        let mut server = SdoServer::new(NODE_ID, can_socket, od);
        debug!("Enter SDO request servicing");
        loop {
            let frame = server.next_sdo_frame().await;
            server.process_frame(frame).await;
        }
    };

    set_return_type::<Result<(), CanOpenError>, _>(&sdo_server);

    // let node_state_machine = async {
    //     let mut can_socket = match CANSocket::open("can0") {
    //         Ok(socket) => socket,
    //         Err(error) => {
    //             error!("Error opening {}: {}", cli.interface, error);
    //             quit::with_code(1);
    //         }
    //     };
    //     let mut sm = NodeStateMachine::new(can_socket, od);
    //     loop {
    //         drate().await;
    //     }
    // };

    let my_future = async {
        let s = spawn(sdo_server);
        // n = spawn(node_state_machine);
        s.await;
        // n.await;
    };
    debug!("Create and start async runtime");
    let rt = runtime::Runtime::new().unwrap();
    rt.block_on(my_future); // tokio async runtime
    debug!("Finsh the async runtime");
    ()
}

//  https://stackoverflow.com/questions/66863385/how-can-i-use-tokio-to-trigger-a-function-every-period-or-interval-in-seconds
