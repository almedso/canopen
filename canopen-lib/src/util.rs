use crate::CanOpenError;
use parse_int::parse;
use regex::Regex;

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

#[derive(Debug, PartialEq, Clone)]
pub enum TypeVariant {
    U8(u8),
    U16(u16),
    U32(u32),
    I8(i8),
    I32(i32),
    I16(i16),
    F32(f32),
    S(String),
}

/// Parse a number into a byte representation
///
/// # Arguments
///
/// * `s` - The string to parse e.g. `123_u8, 0x44ff_u32, -128_i16, -1.34e-5_f32`
///         The type is added at the end if no valid type is set the string is taken as bytes
pub fn number_parser(s: &str) -> Result<TypeVariant, CanOpenError> {
    let re = Regex::new(r"(.*)_(.{2,3})").unwrap();
    if re.is_match(s) {
        let caps = re.captures(s).unwrap();
        let number_type = caps.get(2).unwrap().as_str();
        let number_value = caps.get(1).unwrap().as_str();
        match number_type {
            "u8" => {
                let u = parse::<u8>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::U8(u));
            }
            "u16" => {
                let u = parse::<u16>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;

                return Ok(TypeVariant::U16(u));
            }
            "u32" => {
                let u = parse::<u32>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::U32(u));
            }
            "i8" => {
                let u = parse::<i8>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::I8(u));
            }
            "i16" => {
                let u = parse::<i16>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::I16(u));
            }
            "i32" => {
                let u = parse::<i32>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::I32(u));
            }
            "f32" => {
                let u = parse::<f32>(number_value).map_err(|_| CanOpenError::InvalidNumber {
                    invalid_number: String::from(number_value),
                })?;
                return Ok(TypeVariant::F32(u));
            }
            x => {
                let number_type = String::from(x);
                return Err(CanOpenError::InvalidNumberType { number_type });
            }
        }
    }
    Ok(TypeVariant::S(String::from(s)))
}

impl TypeVariant {
    pub fn to_little_endian_buffer<'a>(&self, buf: &'a mut [u8]) -> &'a[u8] {
        match self {
            TypeVariant::U8(n) => {
                if 1 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = *n as u8;
                &buf[0..1]
            }
            TypeVariant::I8(n) => {
                if 1 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = *n as u8;
                &buf[0..1]
            }
            TypeVariant::U16(n) => {
                if 2 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = (*n as u16).lo();
                buf[1] = (*n as u16).hi();
                &buf[0..2]
            }
            TypeVariant::I16(n) => {
                if 2 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = (*n as u16).lo();
                buf[1] = (*n as u16).hi();
                &buf[0..2]
            }

            TypeVariant::U32(n) => {
                if 4 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = (*n as u32).lo().lo();
                buf[1] = (*n as u32).lo().hi();
                buf[2] = (*n as u32).hi().lo();
                buf[3] = (*n as u32).hi().hi();
                &buf[0..4]
            }
            TypeVariant::I32(n) => {
                if 4 > buf.len() {
                    panic!("Buffer to small");
                }
                buf[0] = (*n as u32).lo().lo();
                buf[1] = (*n as u32).lo().hi();
                buf[2] = (*n as u32).hi().lo();
                buf[3] = (*n as u32).hi().hi();
                &buf[0..4]
            }
            TypeVariant::F32(n) => {
                if 4 > buf.len() {
                    panic!("Buffer to small");
                }
                let bytes = n.to_le_bytes();
                buf[0] = bytes[0];
                buf[1] = bytes[1];
                buf[2] = bytes[2];
                buf[3] = bytes[3];
                &buf[0..4]
            }
            _ => &buf[0..0],
        }
    }


}

pub trait Split {
    type Output;
    fn lo(&self) -> Self::Output;
    fn hi(&self) -> Self::Output;
    fn split(&self) -> (Self::Output, Self::Output);
}

impl Split for u16 {
    type Output = u8;

    fn lo(&self) -> Self::Output {
        *self as u8
    }
    fn hi(&self) -> Self::Output {
        (*self >> 8) as u8
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
    }
}

impl Split for u32 {
    type Output = u16;

    fn lo(&self) -> Self::Output {
        *self as u16
    }
    fn hi(&self) -> Self::Output {
        (*self >> 16) as u16
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
    }
}

impl Split for u64 {
    type Output = u32;

    fn lo(&self) -> Self::Output {
        *self as u32
    }
    fn hi(&self) -> Self::Output {
        (*self >> 32) as u32
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
    }
}

impl Split for i16 {
    type Output = u8;

    fn lo(&self) -> Self::Output {
        *self as u8
    }
    fn hi(&self) -> Self::Output {
        (*self >> 8) as u8
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
    }
}

impl Split for i32 {
    type Output = u16;

    fn lo(&self) -> Self::Output {
        *self as u16
    }
    fn hi(&self) -> Self::Output {
        (*self >> 16) as u16
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
    }
}

impl Split for i64 {
    type Output = u32;

    fn lo(&self) -> Self::Output {
        *self as u32
    }
    fn hi(&self) -> Self::Output {
        (*self >> 32) as u32
    }
    fn split(&self) -> (Self::Output, Self::Output) {
        (self.hi(), self.lo())
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

    #[test]
    fn test_number_parser_ok() {
        assert_eq!(
            TypeVariant::S(String::from("abc")),
            number_parser("abc").unwrap()
        );
        assert_eq!(TypeVariant::U8(1), number_parser("1_u8").unwrap());
        assert_eq!(TypeVariant::U16(1), number_parser("1_u16").unwrap());
        assert_eq!(
            TypeVariant::U32(0x01020304),
            number_parser("0x01020304_u32").unwrap()
        );
        assert_eq!(TypeVariant::I8(-1), number_parser("-1_i8").unwrap());
        assert_eq!(TypeVariant::I16(-1), number_parser("-1_i16").unwrap());
        assert_eq!(
            TypeVariant::I32(-1020304),
            number_parser("-1020304_i32").unwrap()
        );
        assert_eq!(
            TypeVariant::F32(-0.123e-2),
            number_parser("-0.123e-2_f32").unwrap()
        );
    }

    #[test]
    fn test_number_parser_error() {
        assert_eq!(
            CanOpenError::InvalidNumberType {
                number_type: String::from("f64")
            },
            number_parser("-0.123e-2_f64").unwrap_err()
        );
        assert_eq!(
            CanOpenError::InvalidNumber {
                invalid_number: String::from("-0.123e-2")
            },
            number_parser("-0.123e-2_u32").unwrap_err()
        );
    }

    #[test]
    fn test_into_little_endian_buffer() {
        let mut buf = [0_u8; 20];

        let sut = TypeVariant::I32(-1);
        assert_eq!(
            &[0xff, 0xff, 0xff, 0xff],
            sut.to_little_endian_buffer(buf.as_mut())
        );

        let sut = TypeVariant::U32(0x01020304);
        assert_eq!(
            &[0x04, 0x03, 0x02, 0x01],
            sut.to_little_endian_buffer(buf.as_mut())
        );

        let sut = TypeVariant::U16(0x0102);
        assert_eq!(&[0x02, 0x01], sut.to_little_endian_buffer(buf.as_mut()));

        let sut = TypeVariant::I16(-256);
        assert_eq!(&[0x00, 0xff], sut.to_little_endian_buffer(buf.as_mut()));

        let sut = TypeVariant::U8(0x01);
        assert_eq!(&[0x01], sut.to_little_endian_buffer(buf.as_mut()));

        let sut = TypeVariant::I8(-1);
        assert_eq!(&[0xff], sut.to_little_endian_buffer(buf.as_mut()));

        let sut = TypeVariant::F32(1.0e0);
        assert_eq!(
            &[0x0, 0x0, 0x80, 0x3f],
            sut.to_little_endian_buffer(buf.as_mut())
        );

        let sut = TypeVariant::F32(1.0e1);
        assert_eq!(
            &[0x0, 0x0, 0x20, 0x41],
            sut.to_little_endian_buffer(buf.as_mut())
        );

        let sut = TypeVariant::F32(1.0e2);
        assert_eq!(
            &[0x0, 0x0, 0xc8, 0x42],
            sut.to_little_endian_buffer(buf.as_mut())
        );

        let sut = TypeVariant::F32(2.0e2);
        assert_eq!(
            &[0x0, 0x0, 0x48, 0x43],
            sut.to_little_endian_buffer(buf.as_mut())
        );
    }
}
