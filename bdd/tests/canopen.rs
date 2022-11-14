use std::time::Duration;

use async_trait::async_trait;
use cucumber::{given, then, when, WorldInit};
use futures::{
    future::FutureExt, // for `.fuse()`
    pin_mut,
    select,
    StreamExt,
};
use futures_timer::Delay;
use parse_int::parse;
use tokio;
use tokio_socketcan::{CANFrame, CANSocket};

use bdd::{read_remote_object, write_remote_object, ValueType};
use col::{
    self, nodeid_parser, parse_payload_as_byte_sequence_semicolon_delimited, pdo_cobid_parser,
    CanOpenFrameBuilder,
};

async fn play_timeout(timeout_in_ms: u32) -> () {
    let _timeout = Delay::new(Duration::from_millis(timeout_in_ms.into())).await;
}

async fn expect_frame(
    can_socket: &mut CANSocket,
    expected_cob_id: u32,
    expected_payload: Option<&[u8]>,
) {
    while let Some(Ok(frame)) = can_socket.next().await {
        match expected_payload {
            Some(payload) => {
                if frame.id() == expected_cob_id && frame.data() == payload {
                    break;
                }
            }
            None => {
                if frame.id() == expected_cob_id {
                    break;
                }
            }
        }
    }
}

#[derive(Debug, WorldInit)]
struct World {
    cansocket: CANSocket,
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = tokio_socketcan::Error;

    async fn new() -> Result<Self, Self::Error> {
        let cansocket = CANSocket::open("can0")?;
        Ok(World { cansocket })
    }
}

#[given(regex = r"^.*[Ww]ait (\d*) ms.*$")]
#[when(regex = r"^.*[Ww]ait (\d*) ms.*$")]
#[then(regex = r"^.*[Ww]ait (\d*) ms.*$")]
async fn wait_some_time(_w: &mut World, time_in_ms: u32) {
    play_timeout(time_in_ms).await;
}

#[given(regex = r".*[Nn]ode (0x[0-9a-fA-F]{2}) is up$")]
#[then(regex = r".*[Nn]ode (0x[0-9a-fA-F]{2}) is up$")]
async fn expect_node_sends_nmt_heartbeat(w: &mut World, node: String) {
    const NMT_ERROR_CONTROL: u32 = 0b1110 << 7;
    let cob_id = nodeid_parser(&node).unwrap() as u32 + NMT_ERROR_CONTROL;
    let data: [u8; 1] = [5];

    let can_worker = expect_frame(&mut w.cansocket, cob_id, Some(&data)).fuse();
    let timeout_worker = play_timeout(1200).fuse();

    pin_mut!(can_worker, timeout_worker);

    select! {
        () = can_worker => (), // positive case, worker finishes first
        () = timeout_worker => panic!("No heartbeat received within 1200 ms"), // step failed
    }
}

#[given(regex = r".*PDO (0x[0-9a-fA-F]+) payload ([;_xb0-9a-fA-F]+)$")]
#[when(regex = r".*PDO (0x[0-9a-fA-F]+) payload ([;_xb0-9a-fA-F]+)$")]
async fn stimulus_send_pdo(w: &mut World, cob: String, payload: String) {
    let cob_id = pdo_cobid_parser(&cob).unwrap();
    let (data, len) = parse_payload_as_byte_sequence_semicolon_delimited(&payload);
    let frame = CanOpenFrameBuilder::default()
        .pdo(cob_id)
        .unwrap()
        .payload(&data[0..len])
        .unwrap()
        .build()
        .into();
    w.cansocket.write_frame(frame).unwrap().await.unwrap();
}

#[then(
    regex = r".*([Rr]eject|[Ee]xpect) PDO ([0-9_xa-fA-F]+) payload ([;_xb0-9a-fA-F]+) within (\d+) ms$"
)]
async fn response_read_pdo_with_payload(
    w: &mut World,
    expect_pdo: String,
    cob_id: String,
    payload: String,
    timeout: u32,
) {
    let cob_id = pdo_cobid_parser(&cob_id).unwrap();
    let (data, len) = parse_payload_as_byte_sequence_semicolon_delimited(&payload);

    let can_worker = expect_frame(&mut w.cansocket, cob_id.into(), Some(&data[0..len])).fuse();
    let timeout_worker = play_timeout(timeout).fuse();

    pin_mut!(can_worker, timeout_worker);

    match expect_pdo.as_str() {
        "reject" | "Reject" => {
            select! {
                () = can_worker => panic!("PDO received within {} ms", timeout),
                () = timeout_worker => (), // positive case, worker finishes first
            }
        }
        "expect" | "Expect" => {
            select! {
                () = can_worker => (), // positive case, worker finishes first
                () = timeout_worker => panic!("No PDO not received within {} ms", timeout), // step failed
            }
        }
        _ => unreachable!(),
    }
}

#[then(regex = r".*([Rr]eject|[Ee]xpect) PDO ([0-9_xa-fA-F]+) within (\d+) ms$")]
async fn response_read_pdo(w: &mut World, expect_pdo: String, cob_id: String, timeout: u32) {
    let cob_id = pdo_cobid_parser(&cob_id).unwrap();

    let can_worker = expect_frame(&mut w.cansocket, cob_id.into(), None).fuse();
    let timeout_worker = play_timeout(timeout).fuse();

    pin_mut!(can_worker, timeout_worker);

    match expect_pdo.as_str() {
        "reject" | "Reject" => {
            select! {
                () = can_worker => panic!("PDO received within {} ms", timeout),
                () = timeout_worker => (), // positive case, worker finishes first
            }
        }
        "expect" | "Expect" => {
            select! {
                () = can_worker => (), // positive case, worker finishes first
                () = timeout_worker => panic!("No PDO not received within {} ms", timeout), // step failed
            }
        }
        _ => unreachable!(),
    }
}

#[given(
    regex = r".*[Ss]et object (0x[0-9a-fA-F]{4}),(0x[0-9a-fA-F]{2}) at node (0x[0-9a-fA-F]{2}) as type (u8|u16|u32) to value ([0-9_xba-fA-F]+)$"
)]
#[when(
    regex = r".*[Ss]et object (0x[0-9a-fA-F]{4}),(0x[0-9a-fA-F]{2}) at node (0x[0-9a-fA-F]{2}) as type (u8|u16|u32) to value ([0-9_xba-fA-F]+)$"
)]
async fn write_object_at_node(
    w: &mut World,
    index: String,
    subindex: String,
    node: String,
    data_type: String,
    payload: String,
) {
    let node_id = nodeid_parser(&node).unwrap();
    let index = parse::<u16>(&index).unwrap();
    let subindex = parse::<u8>(&subindex).unwrap();
    let value = parse::<u32>(&payload).unwrap() as u32;
    let value_type = match data_type.as_str() {
        "u8" => ValueType::U8,
        "u16" => ValueType::U16,
        "u32" => ValueType::U32,
        _ => panic!("Unsupported payload type {}", data_type),
    };

    let can_worker = write_remote_object(
        &mut w.cansocket,
        node_id as u8,
        index,
        subindex,
        value_type,
        value,
    )
    .fuse();
    let timeout_worker = play_timeout(500).fuse();

    pin_mut!(can_worker, timeout_worker);

    select! {
        () = can_worker => (), // positive case, worker finishes first
        () = timeout_worker => panic!("No SDO write acknowledge within 500 ms"), // step failed
    }
}

#[given(
    regex = r".*[Ee]xpect object (0x[0-9a-fA-F]{4}),(0x[0-9a-fA-F]{2}) at node (0x[0-9a-fA-F]{2}) to be ([0-9_xa-fA-F]+)$"
)]
#[then(
    regex = r".*[Ee]xpect object (0x[0-9a-fA-F]{4}),(0x[0-9a-fA-F]{2}) at node (0x[0-9a-fA-F]{2}) to be ([0-9_xa-fA-F]+)$"
)]
async fn read_object_at_node(
    w: &mut World,
    index: String,
    subindex: String,
    node: String,
    payload: String,
) {
    let node_id = nodeid_parser(&node).unwrap();
    let index = parse::<u16>(&index).unwrap();
    let subindex = parse::<u8>(&subindex).unwrap();
    let expected_value = parse::<u32>(&payload).unwrap();

    let can_worker = read_remote_object(
        &mut w.cansocket,
        node_id as u8,
        index,
        subindex,
        expected_value,
    )
    .fuse();
    let timeout_worker = play_timeout(500).fuse();

    pin_mut!(can_worker, timeout_worker);

    select! {
        () = can_worker => (), // positive case, worker finishes first
        () = timeout_worker => panic!("No SDO read response within 500 ms"), // step failed
    }
}

#[tokio::main]
async fn main() {
    World::run("tests/features").await;
}
