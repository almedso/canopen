//! Implementation of a CANOpen node
//!
//! A minimal node contains of
//! - an SDO server
//! - an object dictionary with entries ...
//! - A state machine

#[derive(Debug, Copy, Clone)]
pub enum State {
    BootUp,
    Operational,
    Stopped,
    PreOperational,
    UnknownState,
}

#[derive(Debug, Copy, Clone)]
pub enum Mode {
    Operational,
    Stop,
    PreOperational,
    ResetApplication,
    ResetCommunication,
}
