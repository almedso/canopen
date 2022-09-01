use byteorder::{LittleEndian, WriteBytesExt};
use encoding::all::ASCII;
use encoding::{EncoderTrap, Encoding};
use failure::{Error, Fail};
use num_traits::Num;
pub use std::convert::{TryFrom, TryInto};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Error>;

#[derive(Fail, Debug)]
pub enum DataConversionError {
    #[fail(display = "invalid data type: {}", _0)]
    InvalidDataType(u32),
    #[fail(display = "mismatching data type")]
    MismatchingDataType,
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DataType {
    NIL,
    BOOLEAN,
    VOID,
    UNSIGNED8,
    UNSIGNED16,
    UNSIGNED24,
    UNSIGNED32,
    UNSIGNED40,
    UNSIGNED48,
    UNSIGNED56,
    UNSIGNED64,
    INTEGER8,
    INTEGER16,
    INTEGER24,
    INTEGER32,
    INTEGER40,
    INTEGER48,
    INTEGER56,
    INTEGER64,
    REAL32,
    REAL64,
    OCTETSTRING,
    VISIBLESTRING,
    UNICODESTRING,
    TIMEOFDAY,
    TIMEDIFFERENCE,
    DOMAIN,
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Data {
    NIL,
    BOOLEAN(bool),
    VOID(usize),
    UNSIGNED8(u8),
    UNSIGNED16(u16),
    UNSIGNED24(i32),
    UNSIGNED32(u32),
    UNSIGNED40(u64),
    UNSIGNED48(u64),
    UNSIGNED56(u64),
    UNSIGNED64(u64),
    INTEGER8(i8),
    INTEGER16(i16),
    INTEGER24(i32),
    INTEGER32(i32),
    INTEGER40(i64),
    INTEGER48(i64),
    INTEGER56(i64),
    INTEGER64(i64),
    REAL32(f32),
    REAL64(f64),
    OCTETSTRING(Vec<u8>),
    VISIBLESTRING(Vec<u8>),
    UNICODESTRING(String),
    TIMEOFDAY(Instant),
    TIMEDIFFERENCE(Duration),
    DOMAIN(Vec<u8>),
}

impl From<Data> for DataType {
    fn from(data: Data) -> DataType {
        match data {
            Data::NIL => DataType::NIL,
            Data::BOOLEAN(_) => DataType::BOOLEAN,
            Data::VOID(_) => DataType::VOID,
            Data::UNSIGNED8(_) => DataType::UNSIGNED8,
            Data::UNSIGNED16(_) => DataType::UNSIGNED16,
            Data::UNSIGNED32(_) => DataType::UNSIGNED32,
            Data::UNSIGNED64(_) => DataType::UNSIGNED64,
            Data::INTEGER8(_) => DataType::INTEGER8,
            Data::INTEGER16(_) => DataType::INTEGER16,
            Data::INTEGER32(_) => DataType::INTEGER32,
            Data::INTEGER64(_) => DataType::INTEGER64,
            Data::REAL32(_) => DataType::REAL32,
            Data::REAL64(_) => DataType::REAL64,
            Data::OCTETSTRING(_) => DataType::OCTETSTRING,
            Data::VISIBLESTRING(_) => DataType::VISIBLESTRING,
            Data::UNICODESTRING(_) => DataType::UNICODESTRING,
            Data::DOMAIN(_) => DataType::DOMAIN,
            _ => unimplemented!(),
        }
    }
}

impl TryFrom<Data> for bool {
    type Error = Error;

    fn try_from(data: Data) -> Result<bool> {
        match data {
            Data::BOOLEAN(value) => Ok(value),
            _ => Err(DataConversionError::MismatchingDataType.into()),
        }
    }
}

impl TryFrom<Data> for u8 {
    type Error = Error;

    fn try_from(data: Data) -> Result<u8> {
        match data {
            Data::UNSIGNED8(value) => Ok(value),
            _ => Err(DataConversionError::MismatchingDataType.into()),
        }
    }
}

impl TryFrom<Data> for u16 {
    type Error = Error;

    fn try_from(data: Data) -> Result<u16> {
        match data {
            Data::UNSIGNED16(value) => Ok(value),
            _ => Err(DataConversionError::MismatchingDataType.into()),
        }
    }
}

impl TryFrom<Data> for u32 {
    type Error = Error;

    fn try_from(data: Data) -> Result<u32> {
        match data {
            Data::UNSIGNED32(value) => Ok(value),
            _ => Err(DataConversionError::MismatchingDataType.into()),
        }
    }
}

impl TryFrom<Data> for u64 {
    type Error = Error;

    fn try_from(data: Data) -> Result<u64> {
        match data {
            Data::UNSIGNED64(value) => Ok(value),
            _ => Err(DataConversionError::MismatchingDataType.into()),
        }
    }
}

impl Data {
    pub fn len(&self) -> usize {
        match self {
            Data::NIL => 0,
            Data::BOOLEAN(_) => 1,
            Data::VOID(length) => *length,
            Data::UNSIGNED8(_) => 8,
            Data::UNSIGNED16(_) => 16,
            Data::UNSIGNED32(_) => 32,
            Data::UNSIGNED64(_) => 64,
            Data::INTEGER8(_) => 8,
            Data::INTEGER16(_) => 16,
            Data::INTEGER32(_) => 32,
            Data::INTEGER64(_) => 64,
            Data::REAL32(_) => 32,
            Data::REAL64(_) => 64,
            Data::OCTETSTRING(value) => value.len(),
            Data::VISIBLESTRING(value) => value.len(),
            Data::UNICODESTRING(value) => value.len(),
            Data::DOMAIN(value) => value.len(),
            _ => unimplemented!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Data::NIL
    }
}

impl std::str::FromStr for DataType {
    type Err = Error;

    fn from_str(data_type_str: &str) -> Result<Self> {
        data_type_str.parse::<u32>()?.try_into()
    }
}

impl TryFrom<u32> for DataType {
    type Error = Error;

    fn try_from(data_type: u32) -> Result<Self> {
        match data_type {
            0x01 => Ok(DataType::BOOLEAN),
            0x02 => Ok(DataType::INTEGER8),
            0x03 => Ok(DataType::INTEGER16),
            0x04 => Ok(DataType::INTEGER32),
            0x05 => Ok(DataType::UNSIGNED8),
            0x06 => Ok(DataType::UNSIGNED16),
            0x07 => Ok(DataType::UNSIGNED32),
            0x08 => Ok(DataType::REAL32),
            0x09 => Ok(DataType::VISIBLESTRING),
            0x0A => Ok(DataType::OCTETSTRING),
            0x0B => Ok(DataType::UNICODESTRING),
            0x0C => Ok(DataType::TIMEOFDAY),
            0x0D => Ok(DataType::TIMEDIFFERENCE),
            0x0F => Ok(DataType::DOMAIN),
            0x10 => Ok(DataType::INTEGER24),
            0x11 => Ok(DataType::REAL64),
            0x12 => Ok(DataType::INTEGER40),
            0x13 => Ok(DataType::INTEGER48),
            0x14 => Ok(DataType::INTEGER56),
            0x15 => Ok(DataType::INTEGER64),
            0x16 => Ok(DataType::UNSIGNED24),
            0x18 => Ok(DataType::UNSIGNED40),
            0x19 => Ok(DataType::UNSIGNED48),
            0x1A => Ok(DataType::UNSIGNED56),
            0x1B => Ok(DataType::UNSIGNED64),
            _ => Err(DataConversionError::InvalidDataType(data_type).into()),
        }
    }
}

impl TryFrom<Data> for Vec<u8> {
    type Error = Error;

    fn try_from(data: Data) -> Result<Self> {
        let mut bytes = vec![];

        match data {
            Data::NIL => {}
            Data::BOOLEAN(true) => bytes = vec![1u8],
            Data::BOOLEAN(false) => bytes = vec![0u8],
            Data::VOID(length) => bytes = vec![0u8; length],
            Data::UNSIGNED8(value) => bytes.write_u8(value)?,
            Data::UNSIGNED16(value) => bytes.write_u16::<LittleEndian>(value)?,
            Data::UNSIGNED32(value) => bytes.write_u32::<LittleEndian>(value)?,
            Data::UNSIGNED64(value) => bytes.write_u64::<LittleEndian>(value)?,
            Data::INTEGER8(value) => bytes.write_i8(value)?,
            Data::INTEGER16(value) => bytes.write_i16::<LittleEndian>(value)?,
            Data::INTEGER32(value) => bytes.write_i32::<LittleEndian>(value)?,
            Data::INTEGER64(value) => bytes.write_i64::<LittleEndian>(value)?,
            Data::REAL32(value) => bytes.write_f32::<LittleEndian>(value)?,
            Data::REAL64(value) => bytes.write_f64::<LittleEndian>(value)?,
            Data::OCTETSTRING(value) => bytes = value,
            Data::VISIBLESTRING(value) => bytes = value,
            Data::UNICODESTRING(value) => bytes = value.as_bytes().to_vec(),
            Data::DOMAIN(value) => bytes = value,
            _ => unimplemented!(),
        };

        Ok(bytes)
    }
}

impl Data {
    pub fn from_str(value: &str, data_type: DataType) -> Result<Self> {
        Ok(match data_type {
            DataType::NIL => Data::NIL,
            DataType::BOOLEAN => match value {
                "0" => Data::BOOLEAN(false),
                "1" => Data::BOOLEAN(true),
                &_ => panic!("invalid boolean format"),
            },
            DataType::VOID => unimplemented!(),
            DataType::UNSIGNED8 => Data::UNSIGNED8(
                value
                    .as_num::<u8>()
                    .unwrap_or_else(|_| panic!("invalid U8 format: {}\n", value)),
            ),
            DataType::UNSIGNED16 => {
                Data::UNSIGNED16(value.as_num::<u16>().expect("invalid U16 format"))
            }
            DataType::UNSIGNED32 => {
                Data::UNSIGNED32(value.as_num::<u32>().expect("invalid U32 format"))
            }
            DataType::UNSIGNED64 => {
                Data::UNSIGNED64(value.as_num::<u64>().expect("invalid U64 format"))
            }
            DataType::INTEGER8 => {
                Data::INTEGER8(value.as_num::<u8>().expect("invalid I8 format: {}") as i8)
            }
            DataType::INTEGER16 => {
                Data::INTEGER16(value.as_num::<u16>().expect("invalid I16 format") as i16)
            }
            DataType::INTEGER32 => {
                Data::INTEGER32(value.as_num::<u32>().expect("invalid I32 format") as i32)
            }
            DataType::INTEGER64 => {
                Data::INTEGER64(value.as_num::<u64>().expect("invalid I64 format") as i64)
            }
            DataType::REAL32 => {
                Data::REAL32(value.as_num::<u32>().expect("invalid F32 format") as f32)
            }
            DataType::REAL64 => {
                Data::REAL64(value.as_num::<u64>().expect("invalid F64 format") as f64)
            }
            DataType::OCTETSTRING => Data::OCTETSTRING(value.as_bytes().to_vec()),
            DataType::VISIBLESTRING => {
                let data = ASCII.encode(value, EncoderTrap::Strict).unwrap();
                if data.iter().all(|&c| c == 0 || (0x20..=0x7E).contains(&c)) {
                    return Ok(Data::VISIBLESTRING(data));
                } else {
                    panic!("non visible character");
                }
            }
            DataType::UNICODESTRING => Data::UNICODESTRING(value.to_string()),
            _ => unimplemented!(),
        })
    }
}

pub trait AsNum {
    fn as_num<T>(&self) -> Result<T>
    where
        T: Num,
        <T as Num>::FromStrRadixErr: std::error::Error,
        <T as Num>::FromStrRadixErr: std::marker::Send,
        <T as Num>::FromStrRadixErr: std::marker::Sync,
        <T as Num>::FromStrRadixErr: 'static;
}

impl<'a> AsNum for &'a str {
    fn as_num<T>(&self) -> Result<T>
    where
        T: Num,
        <T as Num>::FromStrRadixErr: std::error::Error,
        <T as Num>::FromStrRadixErr: std::marker::Send,
        <T as Num>::FromStrRadixErr: std::marker::Sync,
        <T as Num>::FromStrRadixErr: 'static,
    {
        if let Some(stripped) = self.strip_prefix("0x") {
            Ok(T::from_str_radix(stripped, 16)?)
        } else if self.len() > 1 && self.starts_with('0') {
            Ok(T::from_str_radix(&self[1..], 8)?)
        } else {
            Ok(T::from_str_radix(self, 10)?)
        }
    }
}
