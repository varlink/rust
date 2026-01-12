# Async Varlink Example with Tokio

This example demonstrates how to use the varlink async API with Tokio, including full introspection support.

## Overview

The sans-io architecture in varlink separates protocol logic from I/O operations, making it easy to integrate with any async runtime. This example shows:

- **Async Server with Introspection**: Uses `AsyncVarlinkService` to provide `org.varlink.service.GetInfo` and `GetInterfaceDescription`
- **Async Client**: Uses the generated `VarlinkClient` with `AsyncConnection`
- **Concurrent Connections**: Server handles multiple clients concurrently using `tokio::spawn`
- **Service Discovery**: Demonstrates how clients can discover available interfaces

## Key Features

### AsyncVarlinkService with Introspection

The example uses `AsyncVarlinkService` to wrap interface handlers and provide standard introspection:

```rust
use varlink::{listen_async, AsyncVarlinkService, ListenAsyncConfig};

// Create the interface handler
let ping_service = Arc::new(PingService);
let ping_handler = Arc::new(org_example_ping::new(ping_service));

// Wrap with AsyncVarlinkService for introspection support
let service = Arc::new(AsyncVarlinkService::new(
    "org.example",
    "Async Ping Example",
    "1.0",
    "https://github.com/varlink/rust",
    vec![ping_handler],
));

// Start listening
listen_async(service, "tcp:127.0.0.1:9999", &ListenAsyncConfig::default()).await?;
```

### Async Client

```rust
// Connect to the service
let connection = varlink::AsyncConnection::with_address("tcp:127.0.0.1:9999").await?;
let client = org_example_ping::VarlinkClient::new(connection);

// Make a request
let reply = client.ping("Hello!".to_string()).call().await?;
println!("Response: {}", reply.pong);
```

### Service Discovery via Introspection

```rust
use varlink_stdinterfaces::org_varlink_service_async::VarlinkClientInterface;

let connection = varlink::AsyncConnection::with_address("tcp:127.0.0.1:9999").await?;
let client = varlink_stdinterfaces::org_varlink_service_async::VarlinkClient::new(connection);

// Discover service info
let info = client.get_info().call().await?;
println!("Interfaces: {:?}", info.interfaces);

// Get interface description
let desc = client.get_interface_description("org.example.ping".to_string()).call().await?;
println!("IDL: {}", desc.description);
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
Server: Listening on 127.0.0.1:9999 (with introspection)

=== Running Introspection Example ===

Client: Connecting to 127.0.0.1:9999 for introspection
Client: Calling org.varlink.service.GetInfo...
  Vendor: org.example
  Product: Async Ping Example
  Version: 1.0
  URL: https://github.com/varlink/rust
  Interfaces: ["org.varlink.service", "org.example.ping"]

Client: Calling org.varlink.service.GetInterfaceDescription for 'org.example.ping'...
  Description:
# Example async service
interface org.example.ping

# Returns the same string
method Ping(ping: string) -> (pong: string)

error PingError(parameter: int)

=== Running Client Example ===

Client: Connecting to 127.0.0.1:9999
Client: Sending Ping request: 'Hello, Async Varlink!'
Server: Ping request with: 'Hello, Async Varlink!'
Client: Pong response: 'Hello, Async Varlink!'

=== Running Second Client Example ===

Client: Connecting to 127.0.0.1:9999
Client: Sending Ping request: 'Testing sans-io with Tokio'
Server: Ping request with: 'Testing sans-io with Tokio'
Client: Pong response: 'Testing sans-io with Tokio'

=== Example Complete ===
```

## Architecture

The example is structured as follows:

1. **Protocol Definition**: `org.example.ping.varlink` defines the Ping method
2. **Generated Code**: `build.rs` generates async Rust bindings using `varlink_generator`
3. **Async Server**: `AsyncVarlinkService` wraps interface handlers and provides introspection
4. **Async Client**: `VarlinkClient` makes requests using `AsyncConnection`
5. **Main Function**: Demonstrates introspection and regular method calls

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
