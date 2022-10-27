//! CANOpen library
pub mod eds;
#[allow(unused_must_use)]
#[allow(unused_variables)]
pub mod frame;
pub mod split;

pub use eds::*;
pub use frame::*;
pub use parse_int::parse;

use std::ops::RangeInclusive;

pub fn parse_payload_as_byte_sequence_semicolon_delimited(s: &str) -> ([u8; 8], usize) {
    let mut index: usize = 0;
    let mut result: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
    for byte in s.split(';') {
        result[index] = parse::<u8>(byte).unwrap();
        index += 1;
        if index > 7 {
            // do not parse beyond the 8 bytes
            break;
        }
    }
    (result, index)
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

pub fn nodeid_parser(s: &str) -> Result<u8, String> {
    let nodeid = parse::<u32>(s).map_err(|x| format!("{} is not an integer", x))?;
    if NODE_ID_RANGE.contains(&nodeid) {
        Ok(nodeid.try_into().unwrap())
    } else {
        Err(format!(
            "Node Id is not in range {:x}-{:x}",
            NODE_ID_RANGE.start(),
            NODE_ID_RANGE.end()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payload_as_byte_sequence_() {
        let expected_data: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            (expected_data, 1),
            parse_payload_as_byte_sequence_semicolon_delimited("1")
        );

        let expected_data: [u8; 8] = [1, 2, 3, 0, 0, 0, 0, 0];
        assert_eq!(
            (expected_data, 3),
            parse_payload_as_byte_sequence_semicolon_delimited("01;0b10;0x0_3")
        );
        let expected_data: [u8; 8] = [06, 0x38, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            (expected_data, 4),
            parse_payload_as_byte_sequence_semicolon_delimited("0x06;0x38;0;0")
        );
    }
}
