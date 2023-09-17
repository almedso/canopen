
//! Object dictionary module
//!
//! Allows to construct aka build a object dictionary for a CANOpen node.
//!
//! # Example
//!
//! ```ignore
//! use col::ObjectDictionaryBuilder;
//!
//! let device_type = 0x_ffff_0000_u32;  // LSB part is profile number e.g. 402; MSB is additional information
//! let vendor_id = 0_u32; // need to be registered/purchased at CANOpen authority
//! let node_id = 20_u8;  // needed to build up the object dictionary for a node
//! let od = ObjectDictionaryBuilder::new(device_type, vendor_id)
//!         .device_name("Device Name"),
//!         .hardware_version("Rev 1.0"),
//!         .software_version("1.0.0"),
//!         .product_identifier(1_u32),  // up to the vendor to decide
//!         .product_revision(1_u32), // up to the vendor to decide
//!         .serial_number(123456_u32),
//!         .custom_entry(CanOpenObject::const_object(0x6000,0x01, TypeVariant::U8(0))
//!         .build(node_id);
//! }
//! ```

use crate::{CanOpenError, TypeVariant};
use array_init;
use core::result::Iter;

pub struct SimpleObjectDictionary {}

#[derive(Clone, Copy, Debug)]
pub enum SdoAccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug)]
pub enum StorageType {
    Const,      // cannot be modified - implies ReadOnly SdoAccessType
    Persistent, // is persistent and survives power cycling, e.g. saved in NVRAM
    Variable,   // has default value after power cycling
    NoStorage,  // implies WriteOnly access causes a callback function called
}

/// Synchronous callback type
pub type ObjectChangeCallback = fn(u16, u8, TypeVariant) -> Result<(), CanOpenError>;

/// Default callback function invoked after the value is updated
pub fn default_callback(
    index: u16,
    subindex: u8,
    new_value: TypeVariant,
) -> Result<(), CanOpenError> {
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct CanOpenObject<'a> {
    index: u16,
    subindex: u8,
    value: TypeVariant<'a>,
    storage: StorageType,
    privilege: SdoAccessType,
    callback: ObjectChangeCallback,
}

impl<'a> CanOpenObject<'a> {
    pub fn const_object(index: u16, subindex: u8, value: TypeVariant<'a>) -> CanOpenObject {
        CanOpenObject {
            index,
            subindex,
            value,
            storage: StorageType::Const,
            privilege: SdoAccessType::ReadOnly,
            callback: default_callback,
        }
    }
}

impl Default for CanOpenObject<'_> {
    fn default() -> Self {
        CanOpenObject::const_object(0, 0, TypeVariant::U8(0))
    }
}

const MAX_NUMBER_OF_OBJECTS: usize = 256;

pub struct ObjectDictionaryBuilder<'a> {
    number_of_objects: usize,
    object: [CanOpenObject<'a>; MAX_NUMBER_OF_OBJECTS],
}

impl <'a> ObjectDictionaryBuilder<'a> {
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
                CanOpenObject::const_object(0, 0, TypeVariant::U8(0))
            }),
            number_of_objects: 3,
        };
        odb.object[0] = CanOpenObject::const_object(0x1000, 0x01, TypeVariant::U32(device_type));
        let error_flags = 0_u8;
        odb.object[1] = CanOpenObject::const_object(0x1001, 0x01, TypeVariant::U8(error_flags));
        odb.object[2] = CanOpenObject::const_object(0x1018, 0x01, TypeVariant::U32(vendor_id));

        odb
    }

    /// Register any CANOpen object to the object dictionary via a consuming builder pattern.
    ///
    /// # Panics
    ///
    /// * `Object dictionary too small. -
    ///       in case the compile time array cannot hold any further objects.
    ///       Fix by increasing the constant MAX_NUMBER_OF_OBJECTS
    /// * `Object at index,subindex is already registered` - not implemented yet.
    ///
    pub fn register(mut self, object: CanOpenObject<'a>)-> ObjectDictionaryBuilder<'a> {
        let index = self.number_of_objects;
        self.object[index] = object;
        self.number_of_objects += 1;
        if self.number_of_objects >= MAX_NUMBER_OF_OBJECTS {
            panic!("Object dictionary too small.");
        }
        todo!("panic if object already registered");

        self
    }

    /// Short hand registry of device name object ad 0x1001,0x01
    pub fn device_name(mut self, device_name: &'a str) -> ObjectDictionaryBuilder<'a> {
        self.register(CanOpenObject::const_object(0x1001, 0x01, TypeVariant::S(device_name)))
    }

}

pub struct ObjectDictionary<'a> {
    number_of_objects: usize,
    object: [CanOpenObject<'a>; MAX_NUMBER_OF_OBJECTS],
    node_id: u8,
}

impl ObjectDictionary<'_> {
    /// SDO server interface - invoked only by SDO server
    /// Write an object where the type size is less or equal four bytes
    fn download_expedited(
        &mut self,
        index: u16,
        subindex: u8,
        value: TypeVariant,
    ) -> Result<(), CanOpenError> {
        panic!("Not implemented yet");
    }

    /// SDO server interface - invoked only by SDO server
    /// Write an object where the type size is less or equal four bytes
    fn download(
        &mut self,
        index: u16,
        subindex: u8,
        value: Iter<'_, &[u8]>,
    ) -> Result<(), CanOpenError> {
        panic!("Not implemented yet");
    }

    /// SDO server interface - invoked only by SDO server
    /// Read an object
    /// !TODO review the variance of the signature
    fn upload(&self, index: u16, subindex: u8) -> Result<Iter<'_, [u8; 7]>, CanOpenError> {
        panic!("Not implemented yet");
    }

    /// Register a new object in the object directory
    pub fn register_object(can_object: CanOpenObject) {
        panic!("Not implemented yet");
    }

    /// from local modify the value from local
    pub fn set_object_value(&mut self, index: u16, subindex: u8, value: TypeVariant) {}

    /// from local modify the value from local
    pub fn get_object_value(&self, index: u16, subindex: u8) -> Result<TypeVariant, CanOpenError> {
        Err(CanOpenError::ObjectDoesNotExist { index, subindex })
    }
}
