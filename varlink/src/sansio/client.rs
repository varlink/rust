//! Client state machine for the varlink protocol.
//!
//! This module implements the client-side protocol logic without any I/O operations.

use super::protocol::{parse_message, parse_reply, serialize_request};
use super::types::{ClientEvent, ParseResult, Transmit};
use crate::{Request, Result};
use std::collections::VecDeque;

/// Client connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ClientState {
    /// Idle, no pending requests
    Idle,
    /// Awaiting a reply for a method call
    AwaitingReply {
        /// The method that was called
        method: String,
        /// Whether multiple replies are expected (more flag)
        more: bool,
    },
    /// Receiving multiple replies
    Receiving {
        /// The method being called
        method: String,
    },
    /// Connection upgraded to binary protocol
    Upgraded {
        /// The interface handling the upgraded connection
        interface: String,
    },
    /// Error state
    Error {
        /// Error message
        message: String,
    },
}

/// Sans-IO client state machine for varlink protocol.
///
/// This client implementation contains no I/O code. It operates purely
/// through the poll-based API:
///
/// - `send_request()`: Queue a method call
/// - `handle_input()`: Process incoming bytes
/// - `poll_transmit()`: Get outgoing data to send
/// - `poll_event()`: Get protocol events (replies, errors)
///
/// # Example
///
/// ```no_run
/// use varlink::sansio::Client;
/// use varlink::Request;
/// use std::borrow::Cow;
///
/// let mut client = Client::new();
///
/// // Queue a request
/// let request = Request {
///     method: Cow::Borrowed("org.example.Ping"),
///     parameters: None,
///     more: None,
///     oneway: None,
///     upgrade: None,
/// };
/// client.send_request("org.example.Ping".into(), request)?;
///
/// // Get data to send
/// while let Some(transmit) = client.poll_transmit() {
///     // socket.write_all(&transmit.payload)?;
/// }
///
/// // Process incoming data
/// let data = vec![]; // Read from socket
/// client.handle_input(&data)?;
///
/// // Process events
/// while let Some(event) = client.poll_event() {
///     // Handle event
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct Client {
    /// Current state
    state: ClientState,
    /// Outgoing transmit queue
    send_buf: VecDeque<Transmit>,
    /// Incoming receive buffer
    recv_buf: Vec<u8>,
    /// Pending events queue
    pending_events: VecDeque<ClientEvent>,
}

impl Client {
    /// Create a new client state machine.
    pub fn new() -> Self {
        Self {
            state: ClientState::Idle,
            send_buf: VecDeque::new(),
            recv_buf: Vec::new(),
            pending_events: VecDeque::new(),
        }
    }

    /// Queue a method call request.
    ///
    /// This queues the request for transmission. Use `poll_transmit()` to
    /// get the serialized request data to send over the network.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name (e.g., "org.example.Ping")
    /// * `request` - The request structure
    ///
    /// # Returns
    ///
    /// `Ok(())` if the request was queued, or an error if the client is
    /// not in a state to send requests.
    pub fn send_request(&mut self, method: String, request: Request) -> Result<()> {
        // Check if we can send a request in the current state
        match &self.state {
            ClientState::Idle => {
                // Serialize the request
                let payload = serialize_request(&request)?;
                self.send_buf.push_back(Transmit::new(payload));

                // Update state
                let more = request.more.unwrap_or(false);
                self.state = ClientState::AwaitingReply { method, more };

                Ok(())
            }
            _ => Err(crate::context!(crate::ErrorKind::ConnectionBusy)),
        }
    }

    /// Process incoming data from the network.
    ///
    /// This feeds data into the state machine for parsing. After calling this,
    /// use `poll_event()` to retrieve any generated events.
    ///
    /// # Arguments
    ///
    /// * `data` - Incoming bytes from the network
    ///
    /// # Returns
    ///
    /// `Ok(())` if the data was processed successfully, or an error if
    /// the data is invalid.
    pub fn handle_input(&mut self, data: &[u8]) -> Result<()> {
        // Append to receive buffer
        self.recv_buf.extend_from_slice(data);

        // Try to parse messages
        loop {
            match parse_message(&self.recv_buf) {
                ParseResult::Complete { message, consumed } => {
                    // Remove consumed bytes
                    self.recv_buf.drain(..consumed);

                    // Parse as reply
                    let reply = parse_reply(&message)?;

                    // Handle based on current state
                    match &self.state {
                        ClientState::AwaitingReply { method, more } => {
                            let continues = reply.continues.unwrap_or(false);

                            // Queue event
                            self.pending_events.push_back(ClientEvent::Reply {
                                method: method.clone(),
                                reply: reply.clone(),
                                continues,
                            });

                            // Update state
                            if continues && *more {
                                self.state = ClientState::Receiving {
                                    method: method.clone(),
                                };
                            } else {
                                self.state = ClientState::Idle;
                            }
                        }
                        ClientState::Receiving { method } => {
                            let continues = reply.continues.unwrap_or(false);

                            // Queue event
                            self.pending_events.push_back(ClientEvent::Reply {
                                method: method.clone(),
                                reply: reply.clone(),
                                continues,
                            });

                            // Update state
                            if !continues {
                                self.state = ClientState::Idle;
                            }
                        }
                        _ => {
                            return Err(crate::context!(crate::ErrorKind::InvalidParameter(
                                "Unexpected reply".into()
                            )));
                        }
                    }
                }
                ParseResult::Incomplete { .. } => {
                    // Need more data
                    break;
                }
                ParseResult::Invalid { error } => {
                    self.state = ClientState::Error {
                        message: error.clone(),
                    };
                    return Err(crate::context!(crate::ErrorKind::InvalidParameter(error)));
                }
            }
        }

        Ok(())
    }

    /// Poll for outgoing data to transmit.
    ///
    /// Returns the next chunk of data that should be sent over the network,
    /// or `None` if there is no data to send.
    ///
    /// # Returns
    ///
    /// `Some(Transmit)` if there is data to send, or `None` if the send
    /// buffer is empty.
    pub fn poll_transmit(&mut self) -> Option<Transmit> {
        self.send_buf.pop_front()
    }

    /// Poll for protocol events.
    ///
    /// Returns the next event generated by the state machine, such as
    /// replies, errors, or upgrade notifications.
    ///
    /// # Returns
    ///
    /// `Some(ClientEvent)` if there is an event available, or `None` if
    /// there are no pending events.
    pub fn poll_event(&mut self) -> Option<ClientEvent> {
        self.pending_events.pop_front()
    }

    /// Get the current state of the client.
    pub fn state(&self) -> &ClientState {
        &self.state
    }

    /// Check if the client is idle (can send a new request).
    pub fn is_idle(&self) -> bool {
        matches!(self.state, ClientState::Idle)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_client_new() {
        let client = Client::new();
        assert!(client.is_idle());
    }

    #[test]
    fn test_send_request() {
        let mut client = Client::new();

        let request = Request {
            method: Cow::Borrowed("org.example.Ping"),
            parameters: None,
            more: None,
            oneway: None,
            upgrade: None,
        };

        client
            .send_request("org.example.Ping".into(), request)
            .unwrap();

        // Should have something to transmit
        assert!(client.poll_transmit().is_some());
    }

    #[test]
    fn test_receive_reply() {
        let mut client = Client::new();

        let request = Request {
            method: Cow::Borrowed("org.example.Ping"),
            parameters: None,
            more: None,
            oneway: None,
            upgrade: None,
        };

        client
            .send_request("org.example.Ping".into(), request)
            .unwrap();
        let _ = client.poll_transmit();

        // Simulate receiving a reply
        let reply_data = b"{}\0";
        client.handle_input(reply_data).unwrap();

        // Should have an event
        let event = client.poll_event();
        assert!(event.is_some());

        // Should be back to idle
        assert!(client.is_idle());
    }
}
