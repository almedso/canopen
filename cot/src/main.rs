use clap::{Parser, Subcommand};
use log::{debug, error, warn, info };
use clap_verbosity_flag::{Verbosity};
use std::io::Write;
use chrono::Local;

use futures_timer::Delay;
use std::time::Duration;
use futures_util::StreamExt;
use tokio;
use tokio_socketcan::{CANFrame, CANSocket};

use col;

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

        /// Object value - u32 for now
        value: u32,
    },

    /// Monitor traffic
    Mon  {
        /// NodeId - range 0..127
        #[clap(short, long)]
        node: Option<u8>,
    },

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
            }
            Some(Commands::Wod { node, index, subindex, value }) => {
                    info!("Read Object Directory {}@{},{} -> {}", node, index, subindex, value);
                    let frame = CANFrame::new(0x1, &[0], false, false).unwrap();
                    // can_socket.write_frame(frame)?.await?;
                    can_socket.write_frame(frame).unwrap().await;
                    // match can_socket.write_frame(frame) {
                    //     Ok(x) => x.await,
                    //     Err(error) => { error!("Error writing to {}: {}", cli.interface, error); quit::with_code(1); }
                    // }
                    debug!("Waiting 3 seconds");
                    Delay::new(Duration::from_secs(3)).await;
            }
            Some(Commands::Mon { node }) => {
                match node {
                    Some(n) => { info!("Monitor traffic for node {}", n); }
                    None => { info!("Monitor all traffic"); }
                }
                while let Some(Ok(frame)) = can_socket.next().await {
                        let (frame_type, node_id ) = col::extract_node_and_type(frame.id());
                        warn!("Frame: {} node-id: {}: payload", frame_type, node_id, );

                }
            }
            None => {}
        };
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(my_future)// tokio async runtime
}
