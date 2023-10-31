//! # Implementation of a CANOpen node state machine
//!
//! States are modeled as data types.
//! State transitions are date type conversations.
//! Entry and exit functions are implemented as part those transistions.
//!
//!     --> The compiler finds supports a consistent
//!     state machine specification
//!
//! The StateMachine is a datatype as well. It contains data that is common to all states.
//!
//! The state machine processes state events. Processing can take any actions and/or
//! perform state transitions.
//!
//! # Example
//!
//! ```ignore
//! let od = ObjectDictionary::new(123, 456, 789);
//! let nmt_slave = NodeStateMachine(od);
//! let nmt_slave_task = async {
//!    ;
//! }
//! ```
use crate::{CANOpenFrame, ObjectDictionary};
use tokio_socketcan::{CANFrame, CANSocket};
///
/// ``` mermaid

/// The CANOpen state
struct Initialization {}
/// The CANOpen state
struct Operational {}
/// The CANOpen state
struct Stopped {}
/// The CANOpen state
struct PreOperational {}

///
/// ``` mermaid
#[derive(Debug, Copy, Clone)]
pub enum NmtSlaveEvent {
    BootUp,
    Operational,
    Stop,
    PreOperational,
    ResetApplication,
    ResetCommunication,
}

/// CANOpen node state machine implements  NMT - Slave
///
/// - Manages the state of a CANOpen Node
/// - Is controlled by the CANOpen Network Manager
///
/// ## Specification
///
/// ```mermaid
///
/// stateDiagram-v2
///
/// [*] --> Initialization
/// Initialization --> PreOperational
/// PreOperational --> Operational
/// PreOperational --> Initialization
/// PreOperational --> Stopped
/// Operational --> PreOperational
/// Operational --> Initilization
/// Operational --> Stopped
/// Stopped --> PreOperational
/// Stopped --> Operational
/// Stopped --> Initialization:w
///
/// ```
pub struct NodeStateMachine<'a, S> {
    state: S,
    // shared values
    can_socket: CANSocket,
    od: ObjectDictionary<'a>,
}

impl<'a> NodeStateMachine<'a, Initialization> {
    /// New state machine - aka move into the 1st state
    ///
    /// ``` mermaid
    /// [*] --> Initialization
    /// ```
    ///
    pub fn new(
        od: ObjectDictionary<'a>,
        can_socket: CANSocket,
    ) -> NodeStateMachine<'a, Initialization> {
        NodeStateMachine {
            state: Initialization {},
            can_socket,
            od,
        }
    }
}

impl<'a> From<NodeStateMachine<'a, Initialization>> for NodeStateMachine<'a, PreOperational> {
    fn from(state: NodeStateMachine<'a, Initialization>) -> NodeStateMachine<'a, PreOperational> {
        NodeStateMachine {
            state: PreOperational {},
            can_socket: state.can_socket,
            od: state.od,
        }
    }
}

impl<'a, S> NodeStateMachine<'a, S> {
    // process an state machine event
    // pub async fn handle_nmt_slave_frames(&mut self) {
    //     while let Some(Ok(frame)) = self.can_socket.next().await {
    //         let frame = CANOpenFrame::try_from(frame)?;
    //         if let Ok(nmt_slave_event) = frame.try_from(frame) {
    //             let new_state = self.process(nmt_slave_event);
    //             self = new_state;
    //         }
    //     }
    // }

    // fn process_events(self, event: NmtSlaveEvent) -> Self {
    //     match event {
    //         NmtSlaveEvent::BootUp => NodeStateMachine::<PreOperational>::from(self),
    //         NmtSlaveEvent::ResetCommunication => NodeStateMachine::<Initialization>::from(self),
    //         _ => NodeStateMachine::<Initialization>::new(self.od, self.can_socket),
    //     }
    // }
}

// see also https://hoverbear.org/blog/rust-state-machine-pattern
