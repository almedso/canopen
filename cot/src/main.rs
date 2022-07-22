use chrono::Local;
use clap::{ArgEnum, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::{debug, error, info};
use std::io::Write;

use futures_timer::Delay;
use futures_util::StreamExt;
use hex_slice::AsHex;
use std::time::Duration;
use tokio;
use tokio_socketcan::{CANFrame, CANSocket};

use col::{self, sdo::SDOServerResponse, pdo_cobid_parser};
use parse_int::parse;

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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Debug)]
enum ValueType {
    None,
    U8,
    U16,
    U32,
    U64,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Debug)]
enum FrameType {
    PDO,
    SDO,
    NMT,
    EMG,
    ERR,
}

#[derive(Subcommand)]
enum Commands {
    /// Read object directory
    Rod {
        /// NodeId - range 0 .. 127 aka 0x00 .. 0x7f
        #[clap(value_parser = parse::<u8>)]
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        #[clap(value_parser = parse::<u16>)]
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        #[clap(default_value_t = 0x00, value_parser = parse::<u32>)]
        subindex: u8,
    },

    /// Write object directory
    Wod {
        /// NodeId - range 0..127 aka 0x00 .. 0x7f
        #[clap(value_parser = parse::<u8>)]
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        #[clap(value_parser = parse::<u16>)]
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        #[clap(value_parser = parse::<u8>)]
        subindex: u8,

        /// ValueType of the value
        #[clap(arg_enum)]
        value_type: ValueType,

        /// Object value - u32 as 0xabc_def01 or b0011_1001_0 or 123
        #[clap(value_parser = parse::<u32>)]
        value: u32,
    },

    /// write PDO
    Pdo {
        /// CobId - range 0x180...0x5ff aka 512 max
        #[clap(value_parser = pdo_cobid_parser)]
        cobid: u16,

        /// Remote frame flag
        #[clap(short, long)]
        remote: bool,

        /// ValueType of the value
        #[clap(arg_enum)]
        value_type: ValueType,

        /// PDO payload in hexadecimal with leading 0x maximum 8 bytes
        #[clap(value_parser = parse::<u64>)]
        value: u64,
    },

    /// Monitor traffic
    Mon {
        /// NodeId - range 0..127
        #[clap(short, long, multiple_occurrences(true))]
        nodes: Vec<u8>,

        /// FrameType
        #[clap(arg_enum, short, long, multiple_occurrences(true))]
        frame_types: Vec<FrameType>,
    },
}

async fn client_server_communication_timeout() -> () {
    debug!("Set response timeout to 3 seconds");
    let _timeout = Delay::new(Duration::from_secs(3)).await;
}

async fn write_remote_object(
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
            error!("{:?} is not supported for this SDO", value_type);
            col::upload_request_frame(node, SDO_RECEIVE, index, subindex)
                .unwrap()
                .into()
        }
    };

    match match can_socket.write_frame(frame) {
        Ok(x) => x,
        Err(error) => {
            error!("Error instancing write: {}", error);
            quit::with_code(1);
        }
    }
    .await
    {
        Ok(_) => (),
        Err(error) => {
            error!("Error writing: {}", error);
            quit::with_code(1);
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
                error!("{}", e);
                break;
            }
        }
    }
}

async fn write_remote_object_with_acknowledge_check(
    can_socket: &mut CANSocket,
    node: u8,
    index: u16,
    subindex: u8,
    value_type: ValueType,
    value: u32,
) {
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
    const SDO_RECEIVE: u32 = 0x600;
    let frame: CANFrame = col::upload_request_frame(node, SDO_RECEIVE, index, subindex)
        .unwrap()
        .into();
    match match can_socket.write_frame(frame) {
        Ok(x) => x,
        Err(error) => {
            error!("Error instancing write: {}", error);
            quit::with_code(1);
        }
    }
    .await
    {
        Ok(_) => (),
        Err(error) => {
            error!("Error writing: {}", error);
            quit::with_code(1);
        }
    }

    // read the response
    while let Some(Ok(frame)) = can_socket.next().await {
        match col::CANOpenFrame::try_from(frame) {
            Ok(frame) => {
                if frame.node_id() == node && frame.frame_type() == col::frame::FrameType::SsdoTx {
                    let sdo_response = SDOServerResponse::parse(&frame)
                        .map_err(|x| error!("{}", x))
                        .unwrap();
                    if sdo_response.index == index && sdo_response.subindex == subindex {
                        println!(
                            "CANOpen Object {:#06x},{:#04x} @ {:#04x}: {:#x}",
                            index, subindex, node, sdo_response.data
                        );
                        break;
                    } else {
                        println!(
                            "CANOpen -- Object {:#06x},{:#04x} @ {:#04x}: {:#x}",
                            sdo_response.index, sdo_response.subindex, node, sdo_response.data
                        );
                        break;
                    }
                }
            }
            Err(e) => {
                error!("{}", e);
                break;
            }
        }
    }
}

async fn read_remote_object_with_acknowledge_check(
    can_socket: &mut CANSocket,
    node: u8,
    index: u16,
    subindex: u8,
) {
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

async fn send_pdo(
    can_socket: &mut CANSocket,
    cob_id: u16,
    is_rtr: bool,
    value_type: ValueType,
    value: u64,
) {
    let buffer: [u8; 8];
    let data: &[u8] = match value_type {
        ValueType::None => &[],
        ValueType::U8 => {
            buffer = [value as u8, 0, 0, 0, 0, 0, 0, 0];
            &buffer[0..=0]
        }
        ValueType::U16 => {
            buffer = [value as u8, (value >> 8) as u8, 0, 0, 0, 0, 0, 0];
            &buffer[0..=1]
        }
        ValueType::U32 => {
            buffer = [
                value as u8,
                (value >> 8) as u8,
                (value >> 16) as u8,
                (value >> 24) as u8,
                0,
                0,
                0,
                0,
            ];
            &buffer[0..=3]
        }
        ValueType::U64 => {
            buffer = [
                value as u8,
                (value >> 8) as u8,
                (value >> 16) as u8,
                (value >> 24) as u8,
                (value >> 32) as u8,
                (value >> 40) as u8,
                (value >> 48) as u8,
                (value >> 56) as u8,
            ];
            &buffer[0..=7]
        }
    };
    let frame: CANFrame = col::CANOpenFrame::new_with_rtr(cob_id as u32, data, is_rtr)
        .unwrap()
        .into();
    match can_socket.write_frame(frame) {
        Ok(x) => x,
        Err(error) => {
            error!("Error instancing write: {}", error);
            quit::with_code(1);
        }
    }
    .await
    .unwrap();
}

#[quit::main]
fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .format_timestamp_millis()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{} {}: {}",
                Local::now().format("%H:%M:%S%.3f"),
                level_style.value(record.level()),
                record.args()
            )
        })
        .filter_level(cli.verbose.log_level_filter())
        .init();

    debug!("Verbose: {:?}", cli.verbose);
    info!("CAN interface: {}", cli.interface);

    let my_future = async {
        let mut can_socket = match CANSocket::open(&cli.interface) {
            Ok(socket) => socket,
            Err(error) => {
                error!("Error opening {}: {}", cli.interface, error);
                quit::with_code(1);
            }
        };

        match &cli.command {
            Some(Commands::Rod {
                node,
                index,
                subindex,
            }) => {
                info!("Read Object Directory {}@{},{}", node, index, subindex);
                read_remote_object_with_acknowledge_check(
                    &mut can_socket,
                    *node,
                    *index,
                    *subindex,
                )
                .await;
            }
            Some(Commands::Wod {
                node,
                index,
                subindex,
                value_type,
                value,
            }) => {
                info!(
                    "Write Communication Object: {}@{},{} -> {}",
                    node, index, subindex, value
                );
                write_remote_object_with_acknowledge_check(
                    &mut can_socket,
                    *node,
                    *index,
                    *subindex,
                    *value_type,
                    *value,
                )
                .await;
            }
            Some(Commands::Pdo {
                cobid,
                remote,
                value_type,
                value,
            }) => {
                info!(
                    "Inject PDO cobid 0x{:x} RFR {} Value: 0x{:x}",
                    cobid, remote, value
                );
                send_pdo(&mut can_socket, *cobid, *remote, *value_type, *value).await;
            }
            Some(Commands::Mon { nodes, frame_types }) => {
                if nodes.len() > 0 {
                    info!("Monitor traffic for node {:02x}", nodes.as_hex());
                } else {
                    info!("Monitor traffic for all nodes");
                }
                if frame_types.len() > 0 {
                    info!("Monitor traffic for frame types {:0?}", frame_types);
                } else {
                    info!("Monitor traffic for all frametypes");
                }
                let frame_types = frame_types
                    .into_iter()
                    .flat_map(|x| match *x {
                        FrameType::PDO => [
                            col::FrameType::Rpdo1,
                            col::FrameType::Rpdo2,
                            col::FrameType::Rpdo3,
                            col::FrameType::Rpdo4,
                            col::FrameType::Tpdo1,
                            col::FrameType::Tpdo2,
                            col::FrameType::Tpdo3,
                            col::FrameType::Tpdo4,
                        ],
                        FrameType::SDO => [
                            col::FrameType::SsdoRx,
                            col::FrameType::SsdoTx,
                            col::FrameType::SsdoRx,
                            col::FrameType::SsdoTx,
                            col::FrameType::SsdoRx,
                            col::FrameType::SsdoTx,
                            col::FrameType::SsdoRx,
                            col::FrameType::SsdoTx,
                        ],
                        FrameType::NMT => [
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                        ],
                        FrameType::EMG => [
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                        ],
                        FrameType::ERR => [
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                        ],

                    })
                    .collect::<Vec<col::FrameType>>();
                while let Some(Ok(frame)) = can_socket.next().await {
                    match col::CANOpenFrame::try_from(frame) {
                        Ok(frame) => {
                            if nodes.is_empty() || nodes.contains(&frame.node_id()) {
                                if frame_types.is_empty()
                                    || frame_types.contains(&frame.frame_type())
                                {
                                    println!("{}", frame);
                                }
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
    rt.block_on(my_future) // tokio async runtime
}
