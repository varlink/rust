//! Pure protocol parsing and serialization functions.
//!
//! This module contains side-effect-free functions for parsing and serializing
//! varlink messages. All functions operate on byte buffers and never perform I/O.

use super::types::ParseResult;
use crate::{Reply, Request, Result};

/// Parse a null-terminated varlink message from a byte buffer.
///
/// The varlink protocol uses null-terminated JSON messages. This function
/// scans the buffer for a null terminator and returns the message if complete.
///
/// # Arguments
///
/// * `buf` - The input buffer to parse
///
/// # Returns
///
/// * `ParseResult::Complete` - A complete message was found
/// * `ParseResult::Incomplete` - More data is needed
/// * `ParseResult::Invalid` - The buffer contains invalid data
///
/// # Example
///
/// ```
/// use varlink::sansio::protocol::parse_message;
/// use varlink::sansio::types::ParseResult;
///
/// let buf = b"{\"method\":\"org.example.Ping\"}\0";
/// match parse_message(buf) {
///     ParseResult::Complete { message, consumed } => {
///         assert_eq!(consumed, buf.len());
///         // Parse JSON from message...
///     }
///     _ => panic!("Expected complete message"),
/// }
/// ```
pub fn parse_message(buf: &[u8]) -> ParseResult {
    // Scan for null terminator
    match buf.iter().position(|&b| b == 0) {
        Some(pos) => {
            // Found null terminator
            let message = buf[..pos].to_vec();
            let consumed = pos + 1; // Include the null terminator

            // Basic validation: message should be valid UTF-8 and look like JSON
            if let Ok(s) = std::str::from_utf8(&message) {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return ParseResult::Invalid {
                        error: "Empty message".to_string(),
                    };
                }
                // Check if it looks like JSON (starts with { or [)
                if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
                    return ParseResult::Invalid {
                        error: format!("Message does not look like JSON: {}", trimmed),
                    };
                }
            } else {
                return ParseResult::Invalid {
                    error: "Message is not valid UTF-8".to_string(),
                };
            }

            ParseResult::Complete { message, consumed }
        }
        None => {
            // No null terminator found, need more data
            ParseResult::Incomplete { needed: 0 }
        }
    }
}

/// Serialize a varlink request to bytes with null terminator.
///
/// # Arguments
///
/// * `request` - The request to serialize
///
/// # Returns
///
/// A `Result` containing the serialized bytes (including null terminator)
///
/// # Example
///
/// ```
/// use varlink::{Request, sansio::protocol::serialize_request};
/// use std::borrow::Cow;
///
/// let request = Request {
///     method: Cow::Borrowed("org.example.Ping"),
///     parameters: None,
///     more: None,
///     oneway: None,
///     upgrade: None,
/// };
///
/// let bytes = serialize_request(&request).unwrap();
/// assert!(bytes.ends_with(&[0])); // Null-terminated
/// ```
pub fn serialize_request(request: &Request) -> Result<Vec<u8>> {
    let json = serde_json::to_string(request).map_err(crate::map_context!())?;
    let mut bytes = json.into_bytes();
    bytes.push(0); // Add null terminator
    Ok(bytes)
}

/// Serialize a varlink reply to bytes with null terminator.
///
/// # Arguments
///
/// * `reply` - The reply to serialize
///
/// # Returns
///
/// A `Result` containing the serialized bytes (including null terminator)
///
/// # Example
///
/// ```
/// use varlink::{Reply, sansio::protocol::serialize_reply};
/// use std::borrow::Cow;
///
/// let reply = Reply {
///     parameters: None,
///     continues: None,
///     error: None,
/// };
///
/// let bytes = serialize_reply(&reply).unwrap();
/// assert!(bytes.ends_with(&[0])); // Null-terminated
/// ```
pub fn serialize_reply(reply: &Reply) -> Result<Vec<u8>> {
    let json = serde_json::to_string(reply).map_err(crate::map_context!())?;
    let mut bytes = json.into_bytes();
    bytes.push(0); // Add null terminator
    Ok(bytes)
}

/// Parse a request from message bytes (without null terminator).
///
/// This is a convenience function that combines message parsing with JSON deserialization.
///
/// # Arguments
///
/// * `message` - The message bytes (from `ParseResult::Complete`)
///
/// # Returns
///
/// A `Result` containing the parsed request
pub fn parse_request(message: &[u8]) -> Result<Request<'static>> {
    let request: Request = serde_json::from_slice(message).map_err(crate::map_context!())?;
    // Convert to 'static by cloning any borrowed data
    Ok(Request {
        method: std::borrow::Cow::Owned(request.method.into_owned()),
        parameters: request.parameters,
        more: request.more,
        oneway: request.oneway,
        upgrade: request.upgrade,
    })
}

/// Parse a reply from message bytes (without null terminator).
///
/// This is a convenience function that combines message parsing with JSON deserialization.
///
/// # Arguments
///
/// * `message` - The message bytes (from `ParseResult::Complete`)
///
/// # Returns
///
/// A `Result` containing the parsed reply
pub fn parse_reply(message: &[u8]) -> Result<Reply> {
    serde_json::from_slice(message).map_err(crate::map_context!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_parse_complete_message() {
        let buf = b"{\"method\":\"org.example.Ping\"}\0";
        match parse_message(buf) {
            ParseResult::Complete { message, consumed } => {
                assert_eq!(consumed, buf.len());
                assert_eq!(message, b"{\"method\":\"org.example.Ping\"}");
            }
            _ => panic!("Expected complete message"),
        }
    }

    #[test]
    fn test_parse_incomplete_message() {
        let buf = b"{\"method\":\"org.example.Ping\"}";
        match parse_message(buf) {
            ParseResult::Incomplete { .. } => {
                // Expected
            }
            _ => panic!("Expected incomplete message"),
        }
    }

    #[test]
    fn test_parse_empty_message() {
        let buf = b"\0";
        match parse_message(buf) {
            ParseResult::Invalid { .. } => {
                // Expected
            }
            _ => panic!("Expected invalid message"),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let buf = b"not json\0";
        match parse_message(buf) {
            ParseResult::Invalid { .. } => {
                // Expected
            }
            _ => panic!("Expected invalid message"),
        }
    }

    #[test]
    fn test_parse_with_extra_data() {
        let buf = b"{\"method\":\"org.example.Ping\"}\0{\"extra\":\"data\"}";
        match parse_message(buf) {
            ParseResult::Complete { message, consumed } => {
                assert_eq!(consumed, 30); // Up to and including first null (29 bytes + null)
                assert_eq!(message, b"{\"method\":\"org.example.Ping\"}");
            }
            _ => panic!("Expected complete message"),
        }
    }

    #[test]
    fn test_serialize_request() {
        let request = Request {
            method: Cow::Borrowed("org.example.Ping"),
            parameters: None,
            more: None,
            oneway: None,
            upgrade: None,
        };

        let bytes = serialize_request(&request).unwrap();
        assert!(bytes.ends_with(&[0]));

        // Verify it can be parsed back
        match parse_message(&bytes) {
            ParseResult::Complete { message, .. } => {
                let parsed: Request = serde_json::from_slice(&message).unwrap();
                assert_eq!(parsed.method, "org.example.Ping");
            }
            _ => panic!("Failed to parse serialized request"),
        }
    }

    #[test]
    fn test_serialize_reply() {
        let reply = Reply {
            parameters: None,
            continues: None,
            error: None,
        };

        let bytes = serialize_reply(&reply).unwrap();
        assert!(bytes.ends_with(&[0]));

        // Verify it can be parsed back
        match parse_message(&bytes) {
            ParseResult::Complete { message, .. } => {
                let parsed: Reply = serde_json::from_slice(&message).unwrap();
                assert!(parsed.parameters.is_none());
            }
            _ => panic!("Failed to parse serialized reply"),
        }
    }

    #[test]
    fn test_parse_request_helper() {
        let json = b"{\"method\":\"org.example.Ping\"}";
        let request = parse_request(json).unwrap();
        assert_eq!(request.method, "org.example.Ping");
    }

    #[test]
    fn test_parse_reply_helper() {
        let json = b"{}";
        let reply = parse_reply(json).unwrap();
        assert!(reply.parameters.is_none());
    }
}
