//! Core types for the sans-io varlink implementation.

use crate::{Reply, Request};
use std::net::SocketAddr;

/// Represents data to be transmitted over the network.
///
/// The sans-io state machines produce `Transmit` objects via `poll_transmit()`.
/// The application is responsible for actually sending this data over the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Transmit {
    /// Destination address (None for stream-based protocols like TCP/Unix sockets)
    pub dst: Option<SocketAddr>,
    /// Payload to send (null-terminated JSON for varlink)
    pub payload: Vec<u8>,
}

impl Transmit {
    /// Create a new transmit with payload for a stream protocol
    pub fn new(payload: Vec<u8>) -> Self {
        Self { dst: None, payload }
    }

    /// Create a new transmit with destination and payload for a datagram protocol
    pub fn new_with_dst(dst: SocketAddr, payload: Vec<u8>) -> Self {
        Self {
            dst: Some(dst),
            payload,
        }
    }
}

/// Result of parsing a varlink message from a byte buffer.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult {
    /// A complete message was parsed successfully
    Complete {
        /// The parsed message bytes (without null terminator)
        message: Vec<u8>,
        /// Number of bytes consumed from input (including null terminator)
        consumed: usize,
    },
    /// More data is needed to complete the message
    Incomplete {
        /// Minimum number of additional bytes needed (0 if unknown)
        needed: usize,
    },
    /// The buffer contains invalid data
    Invalid {
        /// Description of the parse error
        error: String,
    },
}

/// Events emitted by the client state machine.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// A reply was received for a method call
    Reply {
        /// The method that was called
        method: String,
        /// The reply from the server
        reply: Reply,
        /// Whether more replies are expected (continues flag)
        continues: bool,
    },
    /// An error occurred
    Error {
        /// The method that was called
        method: String,
        /// The error message
        error: String,
    },
    /// The connection was upgraded to a binary protocol
    Upgraded {
        /// The interface that handles the upgraded connection
        interface: String,
    },
}

/// Events emitted by the server state machine.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// A method call request was received
    Request {
        /// The request from the client
        request: Request<'static>,
    },
    /// The client requested a protocol upgrade
    Upgrade {
        /// The interface to upgrade to
        interface: String,
    },
}
