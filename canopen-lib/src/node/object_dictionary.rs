//! Object dictionary module
//!
//! Allows to construct aka build a object dictionary for a CANOpen node.
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
use array_init;

use super::object::*;

const MAX_NUMBER_OF_OBJECTS: usize = 256;

pub struct ObjectDictionaryBuilder<'a> {
    number_of_objects: usize,
    object: [CanOpenObject<'a>; MAX_NUMBER_OF_OBJECTS],
}

impl<'a> ObjectDictionaryBuilder<'a> {
    /// Create an object dictionary builder filled with device type, error flag and vendor id
    /// such that an minimal CANOpen node can be constructed from.
    ///
    /// The builder is a consuming nature.
    ///
    /// # Arguments
    ///
    /// * `device_type` - A u32 number declaring the profile of the CANOpen Node.
    ///                   LSB part is profile number e.g. 402; MSB is additional information.
    /// * `vendor_id`   - A u32 that is bound to the vendor.
    ///                   can be 0 at delivery of the software vendor.
    ///                   Need to be obtained/purchased from CANOpen authority
    ///                   Is unique for every vendor.
    /// # Returns
    ///
    /// A builder object that allows to construct an minimal Object Dictionary as required for a
    /// CANOpen node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use col::ObjectDictionaryBuilder;
    ///
    /// let device_type = 0x_ffff_0000_u32;
    /// let vendor_id = 0_u32; // need to be registered/purchased at CANOpen authority
    /// let odb = ObjectDictionaryBuilder::new(device_type, vendor_id);
    /// ```
    pub fn new(device_type: u32, vendor_id: u32) -> ObjectDictionaryBuilder<'static> {
        let mut odb = ObjectDictionaryBuilder {
            object: array_init::array_init(|_| {
                CanOpenObject::new_const_object(0, 0, ValueVariant::U8(0))
            }),
            number_of_objects: 3,
        };
        odb.object[0] =
            CanOpenObject::new_const_object(0x1000, 0x01, ValueVariant::U32(device_type));
        let error_flags = 0_u8;
        odb.object[1] =
            CanOpenObject::new_const_object(0x1001, 0x01, ValueVariant::U8(error_flags));
        odb.object[2] = CanOpenObject::new_const_object(0x1018, 0x01, ValueVariant::U32(vendor_id));

        odb
    }

    /// Register any CANOpen object to the object dictionary via a consuming builder pattern.
    ///
    /// Objects are stored in order. This allows binary search in accessing objects.
    ///
    /// # Panics
    ///
    /// * `Object dictionary too small. -
    ///       in case the compile time array cannot hold any further objects.
    ///       Fix by increasing the constant MAX_NUMBER_OF_OBJECTS
    /// * `Object at index,subindex is already registered` -
    ///       There cannot be two objects with same index/subindex.
    /// * `Object is not added in index/subindex ordered sequence` -
    ///       This is temporary. The implementation can be improved by ordering at insert
    ///
    pub fn custom_entry(mut self, object: CanOpenObject<'a>) -> ObjectDictionaryBuilder<'a> {
        if self.number_of_objects >= MAX_NUMBER_OF_OBJECTS {
            panic!("Object dictionary too small.");
        }
        let mut array_index = 0_usize;
        while array_index < self.number_of_objects {
            if self.object[array_index] == object {
                panic!("Object already registered");
            }

            if self.object[array_index] > object {
                // array_index memorizes the place where to insert
                break;
            }
            array_index += 1;
        }

        // Move everything one step to the end to free the slot for the new object
        for i in (array_index + 1..self.number_of_objects + 1).rev() {
            self.object[i] = self.object[i - 1].clone(); // array_index is always greater than 1
        }

        // Insert the new object
        self.number_of_objects += 1;
        self.object[array_index] = object;

        self
    }

    /// Short hand registry of device name object ad 0x1008,0x01
    pub fn device_name(self, device_name: &'a str) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x1008,
            0x01,
            ValueVariant::S(device_name),
        ))
    }

    /// Short hand registry of hardware_version object ad 0x1009,0x01
    pub fn hardware_version(self, hardware_version: &'a str) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x1009,
            0x01,
            ValueVariant::S(hardware_version),
        ))
    }

    /// Short hand registry of software_version object ad 0x100A,0x01
    pub fn software_version(self, software_version: &'a str) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x100A,
            0x01,
            ValueVariant::S(software_version),
        ))
    }

    /// Short hand registry of product_identifier object ad 0x1018,0x02
    pub fn product_identifier(self, product_identifier: u32) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x1018,
            0x02,
            ValueVariant::U32(product_identifier),
        ))
    }

    /// Short hand registry of product revision object ad 0x1018,0x03
    pub fn product_revision(self, product_revision: u32) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x1018,
            0x03,
            ValueVariant::U32(product_revision),
        ))
    }

    /// Short hand registry of serial number object ad 0x1018,0x04
    pub fn serial_number(self, serial_number: u32) -> ObjectDictionaryBuilder<'a> {
        self.custom_entry(CanOpenObject::new_const_object(
            0x1018,
            0x04,
            ValueVariant::U32(serial_number),
        ))
    }

    pub fn build(self, node_id: u8) -> ObjectDictionary<'a> {
        ObjectDictionary {
            number_of_objects: self.number_of_objects,
            object: self.object,
            node_id,
        }
    }
}

pub struct ObjectDictionary<'a> {
    number_of_objects: usize,
    object: [CanOpenObject<'a>; MAX_NUMBER_OF_OBJECTS],
    node_id: u8,
}

impl<'a> ObjectDictionary<'a> {
    fn array_index(&self, index: u16, subindex: u8) -> Result<usize, CanOpenError> {
        let mapped_index = map_index(index, subindex);
        let mut min = 0_usize;
        let mut max = self.number_of_objects;
        if max == min {
            return Err(CanOpenError::ObjectDoesNotExist { index, subindex });
        }
        loop {
            let center = (max - min) / 2 + min;
            if self.object[center].mapped_index == mapped_index {
                return Ok(center);
            }
            if max == min + 1 {
                return Err(CanOpenError::ObjectDoesNotExist { index, subindex });
            }

            if self.object[center].mapped_index > mapped_index {
                max = center;
            }
            if self.object[center].mapped_index < mapped_index {
                min = center;
            }
        }
    }

    /// Load persistently stored values of objects into the object dictionary.
    ///
    /// Persistent object need to be initialized from the persistant storage
    /// at startup before a node can go operational.
    pub fn initialize_from_persistent_storage(&self) {
        todo!("initialize_from_persistent_storage");
    }

    /// Store a value persistently
    pub fn persist_value(&self, index: u16, subindex: u8) -> Result<(), CanOpenError> {
        todo!("persist_value");
    }

    /// Emit PDO if the changed object at index,subindex is configured for
    /// PDO mapping, otherwise it does not.
    ///
    ///# Returns
    ///
    ///Error if the a PDO needs to be emitted but cannot for whatever reason
    pub fn update_pdo(&self, index: u16, subindex: u8) -> Result<(), CanOpenError> {
        todo!("update_pdo");
    }

    /// SDO server interface - invoked only by SDO server
    /// Write an object where the type size is less or equal four bytes
    pub fn download_expedited(
        &self,
        index: u16,
        subindex: u8,
        value: ValueVariant<'a>,
    ) -> Result<(), CanOpenError> {
        let array_index = self.array_index(index, subindex)?;
        match self.object[array_index].privilege {
            SdoAccessType::ReadOnly => {
                return Err(CanOpenError::WritingForbidden);
            }
            SdoAccessType::WriteOnly | SdoAccessType::ReadWrite => {}
        };
        self.set_object_value(index, subindex, value)
    }

    /// SDO server interface - invoked only by SDO server
    /// Read an object
    pub fn upload(&self, index: u16, subindex: u8) -> Result<ValueVariant<'a>, CanOpenError> {
        let array_index = self.array_index(index, subindex)?;
        match self.object[array_index].privilege {
            SdoAccessType::WriteOnly => Err(CanOpenError::ReadAccessImpossible),
            SdoAccessType::ReadOnly | SdoAccessType::ReadWrite => {
                self.object[array_index].get_value()
            }
        }
    }

    /// Take care for all activities a new object value requires.
    ///
    /// (Is supposed to be called form the CANOpen Node locally as also by the CANOpen
    /// communication stack).
    ///
    /// If the new value differs from the existing value and storage class is variable or
    /// persistent - an it is checked if an PDO needs to be emitted.
    ///
    /// If the storage class is variable the value is made persistent.
    ///
    /// If the storage class is no-storage the command is executed
    ///
    /// # returns
    ///
    /// Errof if:
    ///
    /// - if value type does not match the defined object (contract violation.
    /// - if object not found
    /// - Const storage class is tried to write
    pub fn set_object_value(
        &self,
        index: u16,
        subindex: u8,
        value: ValueVariant<'a>,
    ) -> Result<(), CanOpenError> {
        let array_index = self.array_index(index, subindex)?;
        match &(self.object[array_index].value) {
            StoredValue::NoStorage(_h) => {
                let _ignore_return = self.object[array_index].set_value(value)?;
            }
            StoredValue::Const(_v) => {
                let _ignore_return = self.object[array_index].set_value(value)?;
            }
            StoredValue::Variable(_v) => {
                if self.object[array_index].set_value(value)? {
                    self.update_pdo(index, subindex)?;
                }
            }
            StoredValue::Persistent(_v) => {
                if self.object[array_index].set_value(value)? {
                    self.update_pdo(index, subindex)?;
                    self.persist_value(index, subindex)?;
                }
            }
        }
        Ok(())
    }

    /// Get the value of the object
    ///
    /// # Returns
    ///
    /// - The value as variant
    /// - A CanOpenError indicating:
    ///   - Object does not exist
    ///   - Object is not readable
    pub fn get_object_value(
        &self,
        index: u16,
        subindex: u8,
    ) -> Result<ValueVariant<'a>, CanOpenError> {
        let array_index = self.array_index(index, subindex)?;
        self.object[array_index].get_value()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn od_access() {
        let mut od = ObjectDictionary {
            number_of_objects: 0,
            object: array_init::array_init(|i| {
                CanOpenObject::new_const_object((i + 1) as u16, 0, ValueVariant::U8(0))
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
            let sut = sut.custom_entry(CanOpenObject::new_const_object(
                0xffff,
                0xff,
                ValueVariant::U8(0),
            ));
            assert!(sut.is_ordered());

            // Try  one before highest index
            let sut = sut.custom_entry(CanOpenObject::new_const_object(
                0xfffe,
                0xff,
                ValueVariant::U8(0),
            ));
            assert!(sut.is_ordered());

            // Try something in the middle
            let sut = sut.custom_entry(CanOpenObject::new_const_object(
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
            let _sut = sut.custom_entry(CanOpenObject::new_const_object(
                0x1000,
                1,
                ValueVariant::U8(0),
            ));
        }

        #[test]
        #[should_panic]
        fn insert_if_no_space_left_in_od() {
            let mut sut = ObjectDictionaryBuilder::new(123, 456);
            for index in 0..MAX_NUMBER_OF_OBJECTS {
                // use odd subindexes to prevent panic for entry exists
                let sut_new = sut.custom_entry(CanOpenObject::new_const_object(
                    index as u16,
                    13,
                    ValueVariant::U8(0),
                ));
                sut = sut_new;
            }
        }
    }
}
