# Async Example

This example demonstrates how to build an async varlink service and client using Tokio.

## Features

- **Async Server**: Uses `listen_async` and `AsyncConnectionHandler` trait
- **Async Client**: Uses generated `VarlinkClient` with `call()` methods (await internally)
- **Code Generation**: Uses `varlink_generator` in `build.rs` with `generate_async: true` option
- **Error Handling**: Demonstrates proper error handling for both service and custom errors
- **State Management**: Shows how to use `Arc<tokio::sync::RwLock<T>>` for shared state in async context

## Interface

The example implements the `org.example.network` interface which provides:
- `List()` - Returns a list of network devices
- `Info(ifindex: int)` - Returns information about a specific network device
- Custom errors: `UnknownNetworkIfIndex`

## Building

```bash
cargo build
```

The `build.rs` script generates Rust code from `src/org.example.network.varlink` with async client support enabled.

## Running

Start the server:
```bash
cargo run -- --varlink="tcp:127.0.0.1:12345"
```

In another terminal, run the client:
```bash
cargo run -- --varlink="tcp:127.0.0.1:12345" --client
```

## Code Structure

### Server Implementation

The server uses `AsyncConnectionHandler` trait:

```rust
#[async_trait]
impl AsyncConnectionHandler for MyOrgExampleNetwork {
    async fn handle(
        &self,
        server: &mut Server,
        _upgraded_iface: Option<String>,
    ) -> varlink::Result<Option<String>> {
        // Handle requests using server.poll_event() and server.send_reply()
    }
}
```

### Client Implementation

The client uses the generated async client:

```rust
let connection = varlink::AsyncConnection::with_address(address).await?;
let client = org_example_network::VarlinkClient::new(connection);

// Make async calls
let reply = client.list().call().await?;
```

## Comparison with Sync Example

| Feature | Sync Example | Async Example |
|---------|-------------|---------------|
| Connection | `Connection::with_address()` | `AsyncConnection::with_address().await` |
| Client | `VarlinkClient` | `VarlinkClient` |
| Method calls | `.call()` | `.call().await` |
| Server | `listen()` + `VarlinkInterface` trait | `listen_async()` + `AsyncConnectionHandler` trait |
| State | `Arc<RwLock<T>>` | `Arc<tokio::sync::RwLock<T>>` |
| Runtime | Synchronous I/O | Tokio async runtime |
