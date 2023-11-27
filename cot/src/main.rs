use chrono::Local;
use clap::{ArgEnum, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::{debug, error, info};
use std::io::Write;

use futures_util::StreamExt;
use hex_slice::AsHex;
use tokio_socketcan::CANSocket;

use col::{self, sdo_client::SdoClient, util::ValueVariant, CanOpenError, CanOpenFrameBuilder};
use col::{nodeid_parser, number_parser, pdo_cobid_parser};
use parse_int::parse;

use std::time::Instant;

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
pub enum ValueType {
    None,
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    F32,
    STR,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, Debug)]
enum FrameType {
    Pdo,
    Sdo,
    Nmt,
    Emg,
    Err,
}

pub fn parse_buffer_as(
    value_type: ValueType,
    data: &[u8],
) -> Result<ValueVariant<'_>, CanOpenError> {
    match value_type {
        ValueType::U8 => {
            if data.len() == 1 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                Ok(ValueVariant::U8(data[0]))
            }
        }
        ValueType::U16 => {
            if data.len() == 2 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                Ok(ValueVariant::U16(data[0] as u16 + ((data[1] as u16) << 8)))
            }
        }
        ValueType::U32 => {
            if data.len() == 4 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                let buf_of_four_bytes = [data[0], data[1], data[2], data[3]];
                Ok(ValueVariant::U32(u32::from_le_bytes(buf_of_four_bytes)))
            }
        }
        ValueType::I8 => {
            if data.len() == 1 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                Ok(ValueVariant::I8(data[0] as i8))
            }
        }
        ValueType::I16 => {
            if data.len() == 2 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                Ok(ValueVariant::I16(
                    (data[0] as u16 + ((data[1] as u16) << 8)) as i16,
                ))
            }
        }
        ValueType::I32 => {
            if data.len() == 4 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                let buf_of_four_bytes = [data[0], data[1], data[2], data[3]];
                Ok(ValueVariant::I32(i32::from_le_bytes(buf_of_four_bytes)))
            }
        }
        ValueType::F32 => {
            if data.len() == 4 {
                Err(CanOpenError::InvalidNumberType {
                    number_type: "u8".to_string(),
                })
            } else {
                let buf_of_four_bytes = [data[0], data[1], data[2], data[3]];
                Ok(ValueVariant::F32(f32::from_le_bytes(buf_of_four_bytes)))
            }
        }
        ValueType::STR => {
            // copy transient data buffer into static buffer - is it a hacky solution?
            match std::str::from_utf8(data) {
                Ok(v) => Ok(ValueVariant::S(v)),
                Err(_e) => Err(CanOpenError::Formatting),
            }
        }

        _ => Err(CanOpenError::InvalidNumberType {
            number_type: "None".to_string(),
        }),
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Read object directory
    Rod {
        /// NodeId - range 0 .. 127 aka 0x00 .. 0x7f
        #[clap(value_parser = nodeid_parser)]
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        #[clap(value_parser = parse::<u16>)]
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        #[clap(default_value_t = 0x00, value_parser = parse::<u8>)]
        subindex: u8,

        /// ValueType of the value
        #[clap(short('p'), long("print-as"), arg_enum, default_value_t=ValueType::None)]
        value_type: ValueType,
    },

    /// Write object directory
    Wod {
        /// NodeId - range 0..127 aka 0x00 .. 0x7f
        #[clap(value_parser = nodeid_parser)]
        node: u8,

        /// Object index - range 0x0000 .. 0xffff
        #[clap(value_parser = parse::<u16>)]
        index: u16,

        /// Object subindex - index 0x00 .. 0xff
        #[clap(value_parser = parse::<u8>)]
        subindex: u8,

        /// Object value - a string containing the e.g. a number
        value: String,

        /// ValueType of the value
        #[clap(arg_enum, default_value_t=ValueType::STR)]
        value_type: ValueType,
    },

    /// write PDO
    Pdo {
        /// CobId - range 0x180...0x5ff aka 512 max
        #[clap(value_parser = pdo_cobid_parser)]
        cobid: u32,

        /// Remote frame flag
        #[clap(short, long)]
        remote: bool,

        /// Object value - a string containing the e.g. a number
        value: String,

        /// ValueType of the value
        #[clap(arg_enum, default_value_t=ValueType::STR)]
        value_type: ValueType,
    },

    /// Monitor traffic
    Mon {
        /// NodeId - range 0..127
        #[clap(short, long, value_parser = nodeid_parser, multiple_occurrences(true))]
        nodes: Vec<u8>,

        /// CobId - range 0..0x3ff
        #[clap(short, long, value_parser = pdo_cobid_parser, multiple_occurrences(true))]
        cobids: Vec<u32>,

        /// FrameType
        #[clap(arg_enum, short, long, multiple_occurrences(true))]
        frame_types: Vec<FrameType>,

        /// Show relative time stamps
        #[clap(short, long)]
        timestamp: bool,
    },
}

struct ArgumentError;

fn convert<'a>(
    value: &'a String,
    value_type: ValueType,
) -> Result<ValueVariant<'a>, ArgumentError> {
    match value_type {
        ValueType::U8 => {
            let v = value.parse::<u8>().map_err(|_| ArgumentError {})?;
            Ok(ValueVariant::U8(v))
        }
        _ => Ok(ValueVariant::S(&value[..])),
    }
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
                value_type,
                node,
                index,
                subindex,
            }) => {
                info!("Read Object Directory {}@{},{}", node, index, subindex);
                let mut sdo_client = SdoClient::new(*node, can_socket);
                let mut data = [0_u8; 80];
                match sdo_client.read_object(*index, *subindex, &mut data).await {
                    Ok(len) => match parse_buffer_as(*value_type, &data[0..len]) {
                        Ok(variant) => {
                            println!(
                                "Object {:x}@{:x},{:x} value {:?}",
                                *node, *index, *subindex, variant
                            );
                        }
                        Err(error) => {
                            println!("Error result formatting {}", error);
                            println!(
                                "Object {:x}@{:x},{:x} value {:?}",
                                *node,
                                *index,
                                *subindex,
                                &data[0..len]
                            );
                        }
                    },
                    Err(error) => {
                        println!("Error {}", error);
                    }
                }
            }
            Some(Commands::Wod {
                node,
                index,
                subindex,
                value,
                value_type,
            }) => {
                info!(
                    "Write Communication Object: {}@{},{} -> {:?}",
                    node, index, subindex, value
                );

                let mut sdo_client = SdoClient::new(*node, can_socket);
                let mut buffer = [0_u8; 80];
                let variant = match convert(value, *value_type) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Cannot convert {} into a {:?}", value, value_type);
                        quit::with_code(1);
                    }
                };
                let data = variant.to_little_endian_buffer(&mut buffer);
                debug!("Raw Buffer: {:?}", data);
                match sdo_client.write_object(*index, *subindex, data).await {
                    Ok(()) => {
                        println!("Success");
                    }
                    Err(error) => {
                        println!("Error {}", error);
                        quit::with_code(1);
                    }
                }
            }
            Some(Commands::Pdo {
                cobid,
                remote,
                value,
                value_type,
            }) => {
                info!(
                    "Inject PDO cobid 0x{:x} RFR {} Value: {:?}",
                    cobid,
                    remote,
                    value.clone()
                );

                let mut buffer = [0_u8; 8];
                let variant = match convert(value, *value_type) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Cannot convert {} into a {:?}", value, value_type);
                        quit::with_code(1);
                    }
                };
                let data = variant.to_little_endian_buffer(&mut buffer);
                if data.len() > 8 {
                    println!("Error: Value needs to be equal or less than 8 bytes");
                    quit::with_code(1);
                }
                let builder = CanOpenFrameBuilder::default()
                    .set_rtr(false)
                    .pdo(*cobid)
                    .unwrap()
                    .payload(data)
                    .unwrap();
                let frame = builder.build().into();
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
            Some(Commands::Mon {
                nodes,
                cobids,
                frame_types,
                timestamp,
            }) => {
                if !nodes.is_empty() {
                    info!("Monitor traffic for node {:02x}", nodes.as_hex());
                } else {
                    info!("Monitor traffic for all nodes");
                }
                if !frame_types.is_empty() {
                    info!("Monitor traffic for frame types {:0?}", frame_types);
                } else {
                    info!("Monitor traffic for all frametypes");
                }
                let all_frame_types = vec![
                    col::FrameType::NmtErrorControl,
                    col::FrameType::Nmt,
                    col::FrameType::SdoRx,
                    col::FrameType::SdoTx,
                    col::FrameType::Rpdo1,
                    col::FrameType::Rpdo2,
                    col::FrameType::Rpdo3,
                    col::FrameType::Rpdo4,
                    col::FrameType::Tpdo1,
                    col::FrameType::Tpdo2,
                    col::FrameType::Tpdo3,
                    col::FrameType::Tpdo4,
                ];

                let mut frame_types = frame_types
                    .iter()
                    .flat_map(|x| match *x {
                        FrameType::Pdo => [
                            col::FrameType::Rpdo1,
                            col::FrameType::Rpdo2,
                            col::FrameType::Rpdo3,
                            col::FrameType::Rpdo4,
                            col::FrameType::Tpdo1,
                            col::FrameType::Tpdo2,
                            col::FrameType::Tpdo3,
                            col::FrameType::Tpdo4,
                        ],
                        FrameType::Sdo => [
                            col::FrameType::SdoRx,
                            col::FrameType::SdoTx,
                            col::FrameType::SdoRx,
                            col::FrameType::SdoTx,
                            col::FrameType::SdoRx,
                            col::FrameType::SdoTx,
                            col::FrameType::SdoRx,
                            col::FrameType::SdoTx,
                        ],
                        FrameType::Nmt => [
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                            col::FrameType::Nmt,
                        ],
                        FrameType::Emg => [
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                            col::FrameType::NmtErrorControl,
                        ],
                        FrameType::Err => [
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
                if frame_types.is_empty() {
                    frame_types = all_frame_types;
                }

                let start_time = Instant::now();
                while let Some(Ok(frame)) = can_socket.next().await {
                    match col::CANOpenFrame::try_from(frame) {
                        Ok(frame) => {
                            if frame_types.contains(&frame.frame_type())
                                && (nodes.is_empty()
                                    && (cobids.is_empty() || cobids.contains(&frame.cob_id()))
                                    || nodes.contains(&frame.node_id()))
                            {
                                if *timestamp {
                                    let elapsed = start_time.elapsed();
                                    let seconds = elapsed.as_secs() % 1000;
                                    let millis = elapsed.subsec_millis();
                                    print!("{:03}.{:03}s - ", seconds, millis);
                                }
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
    rt.block_on(my_future) // tokio async runtime
}
