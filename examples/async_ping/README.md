# Async Varlink Example with Tokio

This example demonstrates how to use the varlink sans-io state machines with Tokio's async I/O.

## Overview

The sans-io architecture in varlink separates protocol logic from I/O operations, making it easy to integrate with any async runtime. This example shows:

- **Async Server**: Uses `varlink::sansio::Server` with `tokio::net::TcpListener`
- **Async Client**: Uses `varlink::sansio::Client` with `tokio::net::TcpStream`
- **Concurrent Connections**: Server handles multiple clients concurrently using `tokio::spawn`
- **Poll-based API**: Demonstrates the complete event loop pattern

## Key Features

### Sans-IO State Machines

The example uses the pure protocol state machines that operate independently of I/O:

```rust
// Server side
let mut server = Server::new();

// Feed data from the network
server.handle_input(&buf[..n])?;

// Process protocol events
while let Some(event) = server.poll_event() {
    // Handle requests and send replies
}

// Get data to send over the network
while let Some(transmit) = server.poll_transmit() {
    stream.write_all(&transmit.payload).await?;
}
```

### Async Client

```rust
// Client side
let mut client = Client::new();

// Send a request
client.send_request(method, request)?;

// Transmit outgoing data
while let Some(transmit) = client.poll_transmit() {
    stream.write_all(&transmit.payload).await?;
}

// Receive response
let n = stream.read(&mut buf).await?;
client.handle_input(&buf[..n])?;

// Process events
while let Some(event) = client.poll_event() {
    // Handle replies
}
```

## Running the Example

```bash
# Run the example
cargo run --package async_ping

# Run tests
cargo test --package async_ping
```

## Expected Output

```
Server: Listening on 127.0.0.1:9999

=== Running Client Example ===

Client: Connecting to 127.0.0.1:9999
Server: New client connected
Client: Sending Ping request: 'Hello, Async Varlink!'
Server: Received request: "org.example.ping.Ping"
Server: Ping request with: 'Hello, Async Varlink!'
Client: Received reply for method: org.example.ping.Ping
Client: Pong response: 'Hello, Async Varlink!'

=== Running Second Client Example ===

Client: Connecting to 127.0.0.1:9999
Client: Sending Ping request: 'Testing sans-io with Tokio'
Server: Received request: "org.example.ping.Ping"
Server: Ping request with: 'Testing sans-io with Tokio'
Client: Received reply for method: org.example.ping.Ping
Client: Pong response: 'Testing sans-io with Tokio'

=== Example Complete ===
```

## Architecture

The example is structured as follows:

1. **Protocol Definition**: `org.example.ping.varlink` defines the Ping method
2. **Generated Code**: `build.rs` generates Rust bindings using `varlink_generator`
3. **Async Server**: `handle_client()` processes connections using the sans-io Server
4. **Async Client**: `run_client()` makes requests using the sans-io Client
5. **Main Function**: Coordinates server startup and client requests

## Benefits of Sans-IO with Async

- **Testability**: Protocol logic can be tested without actual sockets
- **Flexibility**: Works with any async runtime (tokio, async-std, smol, etc.)
- **Composability**: Easy to integrate into existing async applications
- **Control**: Full control over buffering, timeouts, and error handling
- **Efficiency**: No unnecessary allocations or copies

## Comparison with Blocking I/O

The traditional varlink API uses blocking I/O with `Arc<RwLock<Connection>>`. This async example shows how the sans-io approach enables:

- Non-blocking I/O with async/await
- Concurrent request handling
- Better resource utilization
- Integration with async ecosystems

## Further Reading

- [Sans-IO Pattern](https://sans-io.readthedocs.io/)
- [Firezone Blog: Sans-IO](https://www.firezone.dev/blog/sans-io)
- [Tokio Documentation](https://tokio.rs/)
- [Varlink Protocol Specification](https://varlink.org/)
