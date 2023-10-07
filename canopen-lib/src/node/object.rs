//! Object module
//!
//! # Example
//!
//! ```rust
//! use col::{ObjectDictionaryBuilder, ValueVariant, CanOpenObject};
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
//!         .custom_entry(CanOpenObject::const_object(0x6000,0x01, ValueVariant::U8(0)))
//!         .build(node_id);
//!
//! ```

use crate::{map_index, CanOpenError, ValueVariant};
use array_init;
use core::cmp::Ordering;
use core::mem::discriminant;
use core::result::Iter;
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
    value: StoredValue<'a>,
    privilege: SdoAccessType,
}

impl<'a> CanOpenObject<'a> {
    pub fn const_object(index: u16, subindex: u8, value: ValueVariant<'a>) -> CanOpenObject {
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Const(value),
            privilege: SdoAccessType::ReadOnly,
        }
    }

    pub fn variable_object(index: u16, subindex: u8, value: ValueVariant<'a>) -> CanOpenObject<'a> {
        if discriminant(&value) != discriminant(&ValueVariant::S("")) {
            panic!("Only constant storage class is supported for string typed objects");
        }
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Variable(Arc::new(Mutex::new(value))),
            privilege: SdoAccessType::ReadWrite,
        }
    }

    pub fn persistent_object(
        index: u16,
        subindex: u8,
        value: ValueVariant<'a>,
    ) -> CanOpenObject<'a> {
        if discriminant(&value) != discriminant(&ValueVariant::S("")) {
            panic!("Only constant storage class is supported for string typed objects");
        }
        CanOpenObject {
            mapped_index: map_index(index, subindex),
            value: StoredValue::Persistent(Arc::new(Mutex::new(value))),
            privilege: SdoAccessType::ReadWrite,
        }
    }

    pub fn nostorage_object(
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

    fn make_persistent(&self) -> Result<(), CanOpenError> {
        Ok(())
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
        CanOpenObject::const_object(0, 0, ValueVariant::U8(0))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn can_open_object() {
        assert_eq!(
            CanOpenObject::const_object(1, 1, ValueVariant::U8(0))
                == CanOpenObject::const_object(1, 1, ValueVariant::U8(1)),
            true
        );
        assert_eq!(
            CanOpenObject::const_object(1, 2, ValueVariant::U8(0))
                > CanOpenObject::const_object(1, 1, ValueVariant::U8(0)),
            true
        );
        assert_eq!(
            CanOpenObject::const_object(1, 1, ValueVariant::U8(0))
                < CanOpenObject::const_object(2, 1, ValueVariant::U8(0)),
            true
        );
    }

    #[test]
    fn od_access() {
        let mut od = ObjectDictionary {
            number_of_objects: 0,
            object: array_init::array_init(|i| {
                CanOpenObject::const_object((i + 1) as u16, 0, ValueVariant::U8(0))
            }),
            node_id: 10,
        };
        assert_eq!(
            Err(CanOpenError::ObjectDoesNotExist {
                index: 1,
                subindex: 0
            }),
            od.array_index(1, 0)
        );
        od.number_of_objects = 1;
        assert_eq!(Ok(0), od.array_index(1, 0));
        assert!(od.array_index(0, 0).is_err());
        assert!(od.array_index(2, 0).is_err());

        od.number_of_objects = 2;
        assert!(od.array_index(0, 0).is_err());
        assert_eq!(Ok(0), od.array_index(1, 0));
        assert!(od.array_index(1, 5).is_err());
        assert_eq!(Ok(1), od.array_index(2, 0));
        assert!(od.array_index(3, 0).is_err());

        od.number_of_objects = 100;
        assert!(od.array_index(0, 0).is_err());
        assert_eq!(Ok(0), od.array_index(1, 0));
        assert!(od.array_index(1, 5).is_err());
        assert_eq!(Ok(23), od.array_index(24, 0));
        assert!(od.array_index(40, 1).is_err());
        assert_eq!(Ok(48), od.array_index(49, 0));
        assert_eq!(Ok(99), od.array_index(100, 0));
        assert!(od.array_index(99, 1).is_err());
    }

    mod builder {

        use super::*;

        impl ObjectDictionaryBuilder<'_> {
            fn is_ordered(&self) -> bool {
                if self.number_of_objects == 0 {
                    return true;
                }
                for i in 0..self.number_of_objects - 1 {
                    if self.object[i].mapped_index >= self.object[i + 1].mapped_index {
                        return false;
                    }
                }
                true
            }
        }

        #[test]
        fn ordering_at_entry_insert() {
            let sut = ObjectDictionaryBuilder::new(123, 456);
            assert!(sut.is_ordered());

            // Try lowest index at the start
            // let sut = sut.custom_entry(CanOpenObject::const_object(0, 0, ValueVariant::U8(0)));
            // assert!(sut.is_ordered());

            // Try highest index at the end
            let sut = sut.custom_entry(CanOpenObject::const_object(
                0xffff,
                0xff,
                ValueVariant::U8(0),
            ));
            assert!(sut.is_ordered());

            // Try  one before highest index
            let sut = sut.custom_entry(CanOpenObject::const_object(
                0xfffe,
                0xff,
                ValueVariant::U8(0),
            ));
            assert!(sut.is_ordered());

            // Try something in the middle
            let sut = sut.custom_entry(CanOpenObject::const_object(
                0x1200,
                0xff,
                ValueVariant::U8(0),
            ));
            assert!(sut.is_ordered());
        }

        #[test]
        #[should_panic]
        fn insert_if_entry_exists() {
            let sut = ObjectDictionaryBuilder::new(123, 456);
            // try to add device type again
            let _sut =
                sut.custom_entry(CanOpenObject::const_object(0x1000, 1, ValueVariant::U8(0)));
        }

        #[test]
        #[should_panic]
        fn insert_if_no_space_left_in_od() {
            let mut sut = ObjectDictionaryBuilder::new(123, 456);
            for index in 0..MAX_NUMBER_OF_OBJECTS {
                // use odd subindexes to prevent panic for entry exists
                let sut_new = sut.custom_entry(CanOpenObject::const_object(
                    index as u16,
                    13,
                    ValueVariant::U8(0),
                ));
                sut = sut_new;
            }
        }
    }
}
