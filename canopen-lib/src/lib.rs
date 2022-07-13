pub mod canopen;
#[allow(unused_must_use)]
#[allow(unused_variables)]
pub mod frame;
pub mod split;

pub use canopen::*;
pub use frame::*;

use std::ops::RangeInclusive;

pub fn parse_hex_payload(s: &str) -> ([u8; 8], usize) {
    let without_prefix = s.trim_start_matches("0x");
    let len: usize = without_prefix.len() / 2;
    let mut result: [u8; 8] = [0, 0, 0, 0 ,0 ,0 ,0 ,0];
    for index in 0..len {
        result[index] = hex_number_parser(&without_prefix[index*2 ..=index*2+1]).unwrap() as u8;
    }
    (result, len)
}


pub fn hex_number_parser(s: &str) -> Result<u64, String> {
    let without_prefix = s.trim_start_matches("0x");
    let number = u64::from_str_radix(without_prefix, 16)
        .map_err(|_| format!("`{}` is not a hex number", s))?;
    Ok(number)
}

const PDO_COBID_RANGE: RangeInclusive<u64> = 0x180..=0x5ff;

pub fn pdo_cobid_parser(s: &str) -> Result<u32, String> {
    let cobid = hex_number_parser(s)?;
    if PDO_COBID_RANGE.contains(&cobid) {
        Ok(cobid as u32)
    } else {
        Err(format!(
            "Cob Id is not in range {:x}-{:x}",
            PDO_COBID_RANGE.start(),
            PDO_COBID_RANGE.end()
        ))
    }
}

const NODE_ID_RANGE: RangeInclusive<u64> = 0x00..=0x7f;

pub fn nodeid_parser(s: &str) -> Result<u32, String> {
    let nodeid = hex_number_parser(s)?;
    if NODE_ID_RANGE.contains(&nodeid) {
        Ok(nodeid as u32)
    } else {
        Err(format!(
            "Cob Id is not in range {:x}-{:x}",
            NODE_ID_RANGE.start(),
            NODE_ID_RANGE.end()
        ))
    }
}

