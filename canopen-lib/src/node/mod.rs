//! Implementation of a CANOpen node
//!
//! A minimal node contains of
//! - an SDO server
//! - an object dictionary with entries ...
//! - a state machine
//! - an (empty) application

pub mod node_sm;
pub mod object;
pub mod object_dictionary;
pub mod sdo_server;

pub use node_sm::*;
pub use object_dictionary::*;
pub use sdo_server::*;
