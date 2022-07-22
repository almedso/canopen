pub mod canopen;
#[allow(unused_must_use)]
#[allow(unused_variables)]
pub mod frame;
pub mod split;

pub use canopen::*;
pub use frame::*;
pub use parse_int::parse;

use std::ops::RangeInclusive;

pub fn parse_hex_payload(s: &str) -> ([u8; 8], usize) {
    let without_prefix = s.trim_start_matches("0x");
    let len: usize = without_prefix.len() / 2;
    let mut result: [u8; 8] = [0, 0, 0, 0 ,0 ,0 ,0 ,0];
    for index in 0..len {
        result[index] = parse::<u8>(&without_prefix[index*2 ..=index*2+1]).unwrap();
    }
    (result, len)
}

const PDO_COBID_RANGE: RangeInclusive<u32> = 0x180..=0x5ff;

pub fn pdo_cobid_parser(s: &str) -> Result<u32, String> {
    let cobid = parse::<u32>(s).map_err(|x| format!("{} is not an integer", x))?;
    if PDO_COBID_RANGE.contains(&cobid) {
        Ok(cobid)
    } else {
        Err(format!(
            "Cob Id is not in range {:x}-{:x}",
            PDO_COBID_RANGE.start(),
            PDO_COBID_RANGE.end()
        ))
    }
}

const NODE_ID_RANGE: RangeInclusive<u32> = 0x00..=0x7f;

pub fn nodeid_parser(s: &str) -> Result<u32, String> {
    let nodeid = parse::<u32>(s).map_err(|x| format!("{} is not an integer", x))?;
    if NODE_ID_RANGE.contains(&nodeid) {
        Ok(nodeid)
    } else {
        Err(format!(
            "Node Id is not in range {:x}-{:x}",
            NODE_ID_RANGE.start(),
            NODE_ID_RANGE.end()
        ))
    }
}
