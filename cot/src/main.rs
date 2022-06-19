use clap::{Parser, Subcommand, ArgEnum};
use log::{debug, error, info };
use clap_verbosity_flag::{Verbosity};
use std::io::Write;
use chrono::Local;

use futures_timer::Delay;
use std::time::Duration;
use futures_util::StreamExt;
use tokio;
use tokio_socketcan::{CANFrame, CANSocket};
use hex_slice::AsHex;

use col;

use futures::{
    future::FutureExt, // for `.fuse()`
    pin_mut,
    select,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// CAN interface to read from, write to
    #[clap(short, long, default_value_t = String::from("can0"))]
    interface: String,

    /// Allow verbose output
    #[clap(flatten)]
    verbose: Verbosity,

    #[clap(subcommand)]
    command: Option<Commands>,
}


#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ValueType {
    U8,
    U16,
    U32,
}

#[derive(Subcommand)]
enum Commands {
    /// Read object directory
    Rod  {
        /// NodeId - range 0..127
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        #[clap(default_value_t = 0x00)]
        subindex: u8,
    },

    /// Write object directory
    Wod  {
        /// NodeId - range 0..127
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        subindex: u8,

        /// ValueType of the value
        #[clap(arg_enum)]
        value_type: ValueType,

        /// Object value - u32 for now
        value: u32,
    },

    /// Monitor traffic
    Mon  {
        /// NodeId - range 0..127
        #[clap(short, long,  multiple_occurrences(true))]
        nodes: Vec<u8>,
    },

}

async fn client_server_communication_timeout() -> () {
    debug!("Set response timeout to 3 seconds");
    let _timeout = Delay::new(Duration::from_secs(3)).await;
}

async fn write_remote_object(can_socket: &mut CANSocket, node: u8, index: u16, subindex: u8, value_type: ValueType, value: u32) -> () {

    const SDO_TRANSMIT : u32 = 0x580;
    let frame: CANFrame = match value_type {
        ValueType::U8 => {
            col::download_1_byte_frame(node, SDO_TRANSMIT, index, subindex, value as u8).unwrap().into()
        }
        ValueType::U16 => {
            let buffer: [u8; 2] = [
                (( value >> 8 ) & 0xff_u32) as u8,
                (value & 0xff_u32) as u8,
                ];
            col::download_2_bytes_frame(node, SDO_TRANSMIT, index, subindex, buffer).unwrap().into()
        }
        ValueType::U32 => {
            let buffer: [u8; 4] = [
                (( value >> 24 ) & 0xff_u32) as u8,
                (( value >> 16 ) & 0xff_u32) as u8,
                (( value >> 8 ) & 0xff_u32) as u8,
                (value & 0xff_u32) as u8,
                ];
            col::download_4_bytes_frame(node, SDO_TRANSMIT, index, subindex, buffer).unwrap().into()
        }
    };

    match
        match can_socket.write_frame(frame) {
            Ok(x) => x,
            Err(error) => { error!("Error instancing write: {}", error); quit::with_code(1); }
        }.await
    {
        Ok(_) => (),
        Err(error) => { error!("Error writing: {}", error); quit::with_code(1); }
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
                error!("{}", e);
                break;
            }
        }

    }
}

async fn write_remote_object_with_acknowledge_check(can_socket: &mut CANSocket, node: u8, index: u16, subindex: u8, value_type: ValueType, value: u32) {
    let worker = write_remote_object(can_socket, node, index, subindex, value_type, value).fuse();
    let timeout = client_server_communication_timeout().fuse();

    pin_mut!(worker, timeout);

    select! {
        () = worker => info!("Remote object has been updated"),
        () = timeout => {
            error!("Error: Object directory writing not acknowledged within 3 sec timeout");
            quit::with_code(1);
        }
    }
}

async fn read_remote_object(can_socket: &mut CANSocket, node: u8, index: u16, subindex: u8) -> () {
    const SDO_TRANSMIT : u32 = 0x580;
    let frame : CANFrame = col::upload_request_frame(node, SDO_TRANSMIT, index, subindex).unwrap().into();
    match
        match can_socket.write_frame(frame) {
            Ok(x) => x,
            Err(error) => { error!("Error instancing write: {}", error); quit::with_code(1); }
        }.await
    {
        Ok(_) => (),
        Err(error) => { error!("Error writing: {}", error); quit::with_code(1); }
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
                error!("{}", e);
                break;
            }
        }

    }
}

async fn read_remote_object_with_acknowledge_check(can_socket: &mut CANSocket, node: u8, index: u16, subindex: u8) {
    let worker = read_remote_object(can_socket, node, index, subindex).fuse();
    let timeout = client_server_communication_timeout().fuse();

    pin_mut!(worker, timeout);

    select! {
        () = worker => info!("Remote object has been updated"),
        () = timeout => {
            error!("Error: Object directory reading not responded within 3 sec timeout");
            quit::with_code(1);
        }
    }
}


#[quit::main]
fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
    .format_timestamp_millis()
    .format(|buf, record| {
        let level_style = buf.default_level_style(record.level());
        writeln!(buf, "{} {}: {}",
            Local::now().format("%H:%M:%S%.3f"),
            level_style.value(record.level()),
            record.args())
    })
    .filter_level(cli.verbose.log_level_filter())
    .init();

    debug!("Verbose: {:?}", cli.verbose);
    info!("CAN interface: {}", cli.interface);

    let my_future = async {

        let mut can_socket = match CANSocket::open(&cli.interface) {
            Ok(socket) => { socket },
            Err(error) => { error!("Error opening {}: {}", cli.interface, error); quit::with_code(1); }
        };

        match &cli.command {
            Some(Commands::Rod { node, index, subindex }) => {
                info!("Read Object Directory {}@{},{}", node, index, subindex);
                read_remote_object_with_acknowledge_check(&mut can_socket, *node, *index, *subindex).await ;
            }
            Some(Commands::Wod { node, index, subindex, value_type, value }) => {
                info!("Write Communication Object: {}@{},{} -> {}", node, index, subindex, value);
                write_remote_object_with_acknowledge_check(&mut can_socket, *node, *index, *subindex, *value_type, *value).await ;
            }
            Some(Commands::Mon { nodes }) => {
                if nodes.len() > 0 {
                    info!("Monitor traffic for node {:02x}", nodes.as_hex());
                } else  {
                    info!("Monitor all traffic");
                }
                while let Some(Ok(frame)) = can_socket.next().await {
                    match col::CANOpenFrame::try_from(frame) {
                        Ok(frame) => {
                            if nodes.is_empty() || nodes.contains(&frame.node_id()) {
                                println!("{}", frame);
                            }
                        }
                        Err(e) => error!("{}", e),
                    }
                }
            }
            None => {}
        };
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(my_future)// tokio async runtime
}
