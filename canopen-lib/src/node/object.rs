//! Object module
//!
//! # Example
//!
//! ```rust
//! use col::{ObjectDictionaryBuilder, ValueVariant, object::CanOpenObject};
//!
//! let device_type = 0x_ffff_0000_u32;  // LSB part is profile number e.g. 402; MSB is additional information
//! let vendor_id = 0_u32; // need to be registered/purchased at CANOpen authority
//! let node_id = 20_u8;  // needed to build up the object dictionary for a node
//! let od = ObjectDictionaryBuilder::new(device_type, vendor_id)
//!         .device_name("Device Name")
//!         .hardware_version("Rev 1.0")
//!         .software_version("1.0.0")
//!         .product_identifier(1_u32)  // up to the vendor to decide
//!         .product_revision(1_u32) // up to the vendor to decide
//!         .serial_number(123456_u32)
//!         .custom_entry(CanOpenObject::new_const_object(0x6000,0x01, ValueVariant::U8(0)))
//!         .build(node_id);
//!
//! ```

use crate::{map_index, CanOpenError, ValueVariant};
use core::cmp::Ordering;
use core::mem::discriminant;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug)]
pub enum SdoAccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Clone, Debug)]
pub enum StoredValue<'a> {
    Const(ValueVariant<'a>), // cannot be modified - implies ReadOnly SdoAccessType
    Persistent(Arc<Mutex<ValueVariant<'a>>>), // is persistent and survives power cycling, e.g. saved in NVRAM
    Variable(Arc<Mutex<ValueVariant<'a>>>),   // has default value after power cycling
    NoStorage(ObjectChangeHandler), // implies WriteOnly access like a command or download of software
}

/// Synchronous callback type
pub type ObjectChangeHandler = fn(u32, ValueVariant) -> Result<(), CanOpenError>;

/// Default callback function invoked after the value is updated
pub fn default_handler(_mapped_index: u32, _new_value: ValueVariant) -> Result<(), CanOpenError> {
    Ok(())
}

#[derive(Clone, Debug)]
pub struct CanOpenObject<'a> {
    pub mapped_index: u32,
    pub value: StoredValue<'a>,
    pub privilege: SdoAccessType,
}

impl<'a> CanOpenObject<'a> {
    pub fn new_const_object(index: u16, subindex: u8, value: ValueVariant<'a>) -> CanOpenObject {
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Const(value),
            privilege: SdoAccessType::ReadOnly,
        }
    }

    pub fn new_variable_object(
        index: u16,
        subindex: u8,
        value: ValueVariant<'a>,
    ) -> CanOpenObject<'a> {
        if discriminant(&value) == discriminant(&ValueVariant::S("")) {
            panic!("Only constant storage class is supported for string typed objects");
        }
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Variable(Arc::new(Mutex::new(value))),
            privilege: SdoAccessType::ReadWrite,
        }
    }

    pub fn new_persistent_object(
        index: u16,
        subindex: u8,
        value: ValueVariant<'a>,
    ) -> CanOpenObject<'a> {
        if discriminant(&value) == discriminant(&ValueVariant::S("")) {
            panic!("Only constant storage class is supported for string typed objects");
        }
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Persistent(Arc::new(Mutex::new(value))),
            privilege: SdoAccessType::ReadWrite,
        }
    }

    pub fn new_nostorage_object(
        index: u16,
        subindex: u8,
        handler: ObjectChangeHandler,
    ) -> CanOpenObject<'a> {
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::NoStorage(handler),
            privilege: SdoAccessType::WriteOnly,
        }
    }

    ///  Set the value of the object
    ///
    /// # Returns
    ///
    /// - `true` - if the value has been modified
    /// - `false` - if the value has not be modified
    ///
    /// CanOpenError
    ///
    /// - if Write Access is impossible
    /// - if Type mismatch
    /// - if mutext shared access is not possible
    pub fn set_value(&self, value: ValueVariant<'a>) -> Result<bool, CanOpenError> {
        match &self.value {
            StoredValue::NoStorage(f) => f(self.mapped_index, value).map(|_x| true),
            StoredValue::Const(_v) => Err(CanOpenError::CannotWriteToConstStorage),
            StoredValue::Variable(v) | StoredValue::Persistent(v) => {
                if let Ok(mut x) = v.lock() {
                    // compare the enum variant not the value
                    if discriminant(&value) != discriminant(&x) {
                        return Err(CanOpenError::InvalidNumberType {
                            number_type: "todo".to_string(),
                        });
                    }
                    if *x != value {
                        *x = value;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Err(CanOpenError::SharedOdAccessError)
                }
            }
        }
    }

    /// Retrieve a value from the object
    ///
    /// # Returns
    ///
    /// - The value as ValueVariant
    /// - CanOpenError of the object is not stored at all
    pub fn get_value(&self) -> Result<ValueVariant<'a>, CanOpenError> {
        match &self.value {
            StoredValue::Const(v) => Ok(v.clone()),
            StoredValue::NoStorage(_f) => Err(CanOpenError::ReadAccessImpossible),
            StoredValue::Variable(v) | StoredValue::Persistent(v) => {
                if let Ok(x) = v.lock() {
                    Ok(x.clone())
                } else {
                    Err(CanOpenError::SharedOdAccessError)
                }
            }
        }
    }
}

impl PartialEq for CanOpenObject<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.mapped_index == other.mapped_index
    }
}

impl PartialOrd for CanOpenObject<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.mapped_index.partial_cmp(&other.mapped_index)
    }
}

impl Default for CanOpenObject<'_> {
    fn default() -> Self {
        CanOpenObject::new_const_object(0, 0, ValueVariant::U8(0))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn partial_eq_partial_ord() {
        assert_eq!(
            CanOpenObject::new_const_object(1, 1, ValueVariant::U8(0))
                == CanOpenObject::new_const_object(1, 1, ValueVariant::U8(1)),
            true
        );
        assert_eq!(
            CanOpenObject::new_const_object(1, 2, ValueVariant::U8(0))
                > CanOpenObject::new_const_object(1, 1, ValueVariant::U8(0)),
            true
        );
        assert_eq!(
            CanOpenObject::new_const_object(1, 1, ValueVariant::U8(0))
                < CanOpenObject::new_const_object(2, 1, ValueVariant::U8(0)),
            true
        );
    }

    #[test]
    fn variable_access() {
        let sut = CanOpenObject::new_variable_object(1, 1, ValueVariant::U8(0));
        // modify the object
        assert_eq!(true, sut.set_value(ValueVariant::U8(1)).unwrap());
        // do not modify the object
        assert_eq!(false, sut.set_value(ValueVariant::U8(1)).unwrap());
        assert_eq!(ValueVariant::U8(1), sut.get_value().unwrap());
    }

    #[test]
    #[should_panic]
    fn variable_access_string_type() {
        let _sut =
            CanOpenObject::new_variable_object(1, 1, ValueVariant::S("this is a str reference"));
    }

    #[test]
    fn persistent_access() {
        let sut = CanOpenObject::new_persistent_object(1, 1, ValueVariant::U8(0));
        // modify the object
        assert_eq!(true, sut.set_value(ValueVariant::U8(1)).unwrap());
        // do not modify the object
        assert_eq!(false, sut.set_value(ValueVariant::U8(1)).unwrap());
        assert_eq!(ValueVariant::U8(1), sut.get_value().unwrap());
    }

    #[test]
    #[should_panic]
    fn persistent_access_string_type() {
        let _sut =
            CanOpenObject::new_persistent_object(1, 1, ValueVariant::S("this is a str reference"));
    }

    #[test]
    fn const_get() {
        let sut = CanOpenObject::new_const_object(1, 1, ValueVariant::S("const str ref"));
        assert_eq!(ValueVariant::S("const str ref"), sut.get_value().unwrap());
    }

    #[test]
    fn const_set_returns_error() {
        let sut = CanOpenObject::new_const_object(1, 1, ValueVariant::U8(2));
        assert!(sut.set_value(ValueVariant::U8(3)).is_err());
    }

    fn nostorage_spy(_mapped_index: u32, new_value: ValueVariant) -> Result<(), CanOpenError> {
        if ValueVariant::U8(42) == new_value {
            Ok(())
        } else {
            Err(CanOpenError::SharedOdAccessError)
        }
    }

    #[test]
    fn nostorage_set() {
        let sut = CanOpenObject::new_nostorage_object(1, 1, nostorage_spy);
        assert!(sut.set_value(ValueVariant::U8(42)).unwrap());
        assert!(sut.set_value(ValueVariant::U8(1)).is_err());
    }

    #[test]
    fn nostorage_get_returns_error() {
        let sut = CanOpenObject::new_nostorage_object(1, 1, nostorage_spy);
        assert!(sut.get_value().is_err());
    }
}
