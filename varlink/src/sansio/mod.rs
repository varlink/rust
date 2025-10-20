//! Sans-IO implementation of the varlink protocol.
//!
//! This module provides a pure state machine implementation of the varlink protocol
//! that is independent of any I/O operations. This design allows:
//!
//! - Testing protocol logic without real sockets
//! - Flexibility in I/O implementation (sync, async, custom transports)
//! - Better composability and reusability
//! - Runtime-agnostic code
//!
//! # Architecture
//!
//! The sans-io implementation follows the poll-based pattern:
//!
//! - `handle_input()`: Feed incoming bytes to the state machine
//! - `poll_transmit()`: Get outgoing data to send
//! - `poll_event()`: Get protocol events (replies, requests, etc.)
//!
//! # Example
//!
//! ```no_run
//! use varlink::sansio::Client;
//! use varlink::Request;
//! use std::borrow::Cow;
//! use std::io::{Read, Write};
//!
//! let mut client = Client::new();
//!
//! // Create a request
//! let request = Request {
//!     method: Cow::Borrowed("org.example.Method"),
//!     parameters: None,
//!     more: None,
//!     oneway: None,
//!     upgrade: None,
//! };
//!
//! // Queue the request
//! client.send_request("org.example.Method".into(), request)?;
//!
//! // In your event loop (pseudo-code):
//! # let mut socket = std::io::empty(); // Placeholder for example
//! # let mut buf = [0u8; 8192];
//! // 1. Send data
//! while let Some(transmit) = client.poll_transmit() {
//!     socket.write_all(&transmit.payload)?;
//! }
//!
//! // 2. Receive data
//! let n = socket.read(&mut buf)?;
//! client.handle_input(&buf[..n])?;
//!
//! // 3. Process events
//! while let Some(event) = client.poll_event() {
//!     // Handle event
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod client;
pub mod protocol;
pub mod server;
pub mod types;

pub use self::client::Client;
pub use self::protocol::{parse_message, serialize_reply, serialize_request};
pub use self::server::Server;
pub use self::types::{ClientEvent, ParseResult, ServerEvent, Transmit};
