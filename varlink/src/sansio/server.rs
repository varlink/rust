//! Server state machine for the varlink protocol.
//!
//! This module implements the server-side protocol logic without any I/O operations.

use super::protocol::{parse_message, parse_request, serialize_reply};
use super::types::{ParseResult, ServerEvent, Transmit};
use crate::{Reply, Result};
use std::collections::VecDeque;

/// Server connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    /// Receiving requests
    Receiving,
    /// Processing a request
    Processing,
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

/// Sans-IO server state machine for varlink protocol.
///
/// This server implementation contains no I/O code. It operates purely
/// through the poll-based API:
///
/// - `handle_input()`: Process incoming bytes (requests)
/// - `send_reply()`: Queue a reply to send
/// - `poll_transmit()`: Get outgoing data to send
/// - `poll_event()`: Get protocol events (requests, upgrades)
///
/// # Example
///
/// ```no_run
/// use varlink::sansio::Server;
/// use varlink::Reply;
///
/// let mut server = Server::new();
///
/// // Process incoming data
/// let data = vec![]; // Read from socket
/// server.handle_input(&data)?;
///
/// // Process events
/// while let Some(event) = server.poll_event() {
///     // Handle request and send reply
///     let reply = Reply {
///         parameters: None,
///         continues: None,
///         error: None,
///     };
///     server.send_reply(reply)?;
/// }
///
/// // Get data to send
/// while let Some(transmit) = server.poll_transmit() {
///     // socket.write_all(&transmit.payload)?;
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct Server {
    /// Current state
    state: ServerState,
    /// Outgoing transmit queue
    send_buf: VecDeque<Transmit>,
    /// Incoming receive buffer
    recv_buf: Vec<u8>,
    /// Pending events queue
    pending_events: VecDeque<ServerEvent>,
}

impl Server {
    /// Create a new server state machine.
    pub fn new() -> Self {
        Self {
            state: ServerState::Receiving,
            send_buf: VecDeque::new(),
            recv_buf: Vec::new(),
            pending_events: VecDeque::new(),
        }
    }

    /// Process incoming data from the network.
    ///
    /// This feeds data into the state machine for parsing. After calling this,
    /// use `poll_event()` to retrieve any generated events (requests).
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

                    // Parse as request
                    let request = parse_request(&message)?;

                    // Check for upgrade request
                    if request.upgrade.unwrap_or(false) {
                        // Extract interface name from method
                        let interface = request
                            .method
                            .rsplit_once('.')
                            .map(|x| x.0)
                            .unwrap_or(&request.method)
                            .to_string();

                        self.pending_events.push_back(ServerEvent::Upgrade {
                            interface: interface.clone(),
                        });

                        self.state = ServerState::Upgraded { interface };
                    } else {
                        // Normal request
                        self.pending_events
                            .push_back(ServerEvent::Request { request });
                        self.state = ServerState::Processing;
                    }
                }
                ParseResult::Incomplete { .. } => {
                    // Need more data
                    break;
                }
                ParseResult::Invalid { error } => {
                    self.state = ServerState::Error {
                        message: error.clone(),
                    };
                    return Err(crate::context!(crate::ErrorKind::InvalidParameter(error)));
                }
            }
        }

        Ok(())
    }

    /// Queue a reply to send.
    ///
    /// This queues the reply for transmission. Use `poll_transmit()` to
    /// get the serialized reply data to send over the network.
    ///
    /// # Arguments
    ///
    /// * `reply` - The reply structure
    ///
    /// # Returns
    ///
    /// `Ok(())` if the reply was queued, or an error if serialization fails.
    pub fn send_reply(&mut self, reply: Reply) -> Result<()> {
        // Serialize the reply
        let payload = serialize_reply(&reply)?;
        self.send_buf.push_back(Transmit::new(payload));

        // If this is the last reply (continues = false or None), go back to receiving
        if !reply.continues.unwrap_or(false) && self.state == ServerState::Processing {
            self.state = ServerState::Receiving;
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
    /// requests or upgrade notifications.
    ///
    /// # Returns
    ///
    /// `Some(ServerEvent)` if there is an event available, or `None` if
    /// there are no pending events.
    pub fn poll_event(&mut self) -> Option<ServerEvent> {
        self.pending_events.pop_front()
    }

    /// Get the current state of the server.
    pub fn state(&self) -> &ServerState {
        &self.state
    }

    /// Check if the server is in receiving state.
    pub fn is_receiving(&self) -> bool {
        matches!(self.state, ServerState::Receiving)
    }

    /// Check if the server is upgraded.
    pub fn is_upgraded(&self) -> bool {
        matches!(self.state, ServerState::Upgraded { .. })
    }

    /// Push a request back to the front of the event queue.
    ///
    /// This is useful when routing requests between handlers - a dispatcher
    /// can pull a request, examine it, then push it back for another handler
    /// to process.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to push back to the front of the queue
    pub fn push_request(&mut self, request: crate::Request<'static>) {
        self.pending_events
            .push_front(ServerEvent::Request { request });
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_new() {
        let server = Server::new();
        assert!(server.is_receiving());
    }

    #[test]
    fn test_receive_request() {
        let mut server = Server::new();

        // Simulate receiving a request
        let request_data = b"{\"method\":\"org.example.Ping\"}\0";
        server.handle_input(request_data).unwrap();

        // Should have an event
        let event = server.poll_event();
        assert!(event.is_some());

        if let Some(ServerEvent::Request { request }) = event {
            assert_eq!(request.method, "org.example.Ping");
        } else {
            panic!("Expected request event");
        }
    }

    #[test]
    fn test_send_reply() {
        let mut server = Server::new();

        let reply = Reply {
            parameters: None,
            continues: None,
            error: None,
        };

        server.send_reply(reply).unwrap();

        // Should have something to transmit
        assert!(server.poll_transmit().is_some());
    }

    #[test]
    fn test_request_reply_cycle() {
        let mut server = Server::new();

        // Receive request
        let request_data = b"{\"method\":\"org.example.Ping\"}\0";
        server.handle_input(request_data).unwrap();

        // Get event
        let event = server.poll_event();
        assert!(event.is_some());

        // Send reply
        let reply = Reply {
            parameters: None,
            continues: None,
            error: None,
        };
        server.send_reply(reply).unwrap();

        // Should have data to transmit
        let transmit = server.poll_transmit();
        assert!(transmit.is_some());

        // Should be back to receiving
        assert!(server.is_receiving());
    }
}
