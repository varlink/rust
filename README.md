# varlink for rust

[![Varlink Certified](https://img.shields.io/badge/varlink-certified-green.svg)](https://www.varlink.org/Language-Bindings)
[![Build Status](https://travis-ci.org/varlink/rust.svg?branch=master)](https://travis-ci.org/varlink/rust)
[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/varlink/rust/CI)](https://github.com/varlink/rust/actions)
[![Coverage Status](https://coveralls.io/repos/github/varlink/rust/badge.svg?branch=master)](https://coveralls.io/github/varlink/rust?branch=master)
[![Crate](https://img.shields.io/crates/v/varlink.svg)](https://crates.io/crates/varlink)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/varlink/)
[![dependency status](https://deps.rs/repo/github/varlink/rust/status.svg)](https://deps.rs/repo/github/varlink/rust)
![Rust Version 1.70+](https://img.shields.io/badge/rustc-v1.70%2B-blue.svg)


See http://varlink.org for more information about varlink.

## Example usage
Look into the examples directory. ```build.rs``` contains the magic, which will build rust bindings for the varlink interface definition file.
Or use `varlink_derive` to generate the bindings at compile time.

## Sans-IO Architecture

The varlink crate now provides a sans-io implementation that separates protocol logic from I/O operations. This design enables:

- Testing protocol logic without real sockets
- Flexibility in I/O implementation (sync, async, custom transports)
- Runtime-agnostic code
- Better composability

### Using the Sans-IO API

The sans-io module provides pure state machines that operate through a poll-based API:

```rust
use varlink::sansio::{Client, Server};
use varlink::{Request, Reply};
use std::borrow::Cow;

// Client example
let mut client = Client::new();

// Queue a request
let request = Request {
    method: Cow::Borrowed("org.example.Ping"),
    parameters: None,
    more: None,
    oneway: None,
    upgrade: None,
};
client.send_request("org.example.Ping".into(), request)?;

// In your event loop:
// 1. Send outgoing data
while let Some(transmit) = client.poll_transmit() {
    socket.write_all(&transmit.payload)?;
}

// 2. Receive and process incoming data
let n = socket.read(&mut buf)?;
client.handle_input(&buf[..n])?;

// 3. Process protocol events
while let Some(event) = client.poll_event() {
    // Handle replies
}

// Server example
let mut server = Server::new();

// Process incoming data
server.handle_input(&data)?;

// Handle requests
while let Some(event) = server.poll_event() {
    if let ServerEvent::Request { request } = event {
        // Process request and send reply
        let reply = Reply {
            parameters: None,
            continues: None,
            error: None,
        };
        server.send_reply(reply)?;
    }
}

// Send outgoing data
while let Some(transmit) = server.poll_transmit() {
    socket.write_all(&transmit.payload)?;
}
```

The traditional blocking I/O API remains fully supported and uses the sans-io protocol functions internally.

## Async/Await Support with Tokio

The varlink crate provides full async/await support through the `tokio` feature. The async API is architecturally symmetric with the sync API, making it easy to understand and use.

### Enabling Async Support

Add the `tokio` feature to your `Cargo.toml`:

```toml
[dependencies]
varlink = { version = "*", features = ["tokio"] }

[build-dependencies]
varlink_generator = "*"
```

### Generating Async Code

#### Option 1: Using `varlink_async!` Macro (Recommended)

The simplest way to generate async code is using the `varlink_async!` macro:

```rust
use varlink_derive::varlink_async;

varlink_async!(org_example_ping, r#"
# Example async service
interface org.example.ping

# Returns the same string
method Ping(ping: string) -> (pong: string)
"#);
```

This generates all the necessary async traits and types at compile time.

#### Option 2: Using `build.rs`

For larger interface files, use `build.rs`:

```rust
// build.rs
extern crate varlink_generator;
use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let output_path = out_dir.join("org_example_ping.rs");

    let mut input = File::open("src/org.example.ping.varlink").unwrap();
    let mut output = File::create(&output_path).unwrap();

    varlink_generator::generate_with_options(
        &mut input,
        &mut output,
        &varlink_generator::GeneratorOptions {
            generate_async: true,
            ..Default::default()
        },
        false, // tosource: false for include!() usage
    ).unwrap();
}
```

Then include the generated code:

```rust
pub mod org_example_ping {
    include!(concat!(env!("OUT_DIR"), "/org_example_ping.rs"));
}
```

#### Option 3: Command-Line Tool

Generate async code directly from the command line:

```bash
varlink-rust-generator --async src/org.example.ping.varlink > org_example_ping_async.rs
```

### Using Async Interfaces

#### Server Implementation

```rust
use async_trait::async_trait;
use std::sync::Arc;
use varlink::{listen_async, ListenAsyncConfig};

// Include generated code
mod org_example_ping;
use org_example_ping::{VarlinkInterface, Call_Ping};

// Implement the async interface
struct PingService;

#[async_trait]
impl VarlinkInterface for PingService {
    async fn ping(
        &self,
        call: &mut dyn Call_Ping,
        ping: String,
    ) -> varlink::Result<()> {
        // Async business logic here
        call.reply(ping)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Arc::new(PingService);
    let handler = Arc::new(org_example_ping::new(service));

    listen_async(
        handler,
        "tcp:127.0.0.1:12345",
        &ListenAsyncConfig::default(),
    )
    .await?;

    Ok(())
}
```

#### Client Usage

```rust
use org_example_ping::{VarlinkClient, VarlinkClientInterface};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = varlink::AsyncConnection::with_address("tcp:127.0.0.1:12345")
        .await?;

    let client = VarlinkClient::new(connection);

    let reply = client
        .ping("Hello, Async Varlink!".to_string())
        .call()
        .await?;

    println!("Response: {}", reply.pong);
    Ok(())
}
```

### Architecture

The async implementation follows the same architectural patterns as the sync version:

**Sync Architecture:**
- `Interface` trait - provides metadata (`get_name()`, `get_description()`)
- `ConnectionHandler` trait - handles I/O operations
- `VarlinkInterfaceProxy` - adapts user implementation to framework traits

**Async Architecture:**
- `AsyncInterface` trait - provides metadata (`get_name()`, `get_description()`)
- `AsyncConnectionHandler` trait - handles async I/O operations
- `VarlinkInterfaceProxy` - adapts user async implementation to framework traits

Both use the sans-io protocol logic internally, ensuring consistent behavior.

### Key Differences from Sync API

1. **Interface trait methods are async**: Use `async fn` and `.await` syntax
2. **Connection creation**: `AsyncConnection::with_address().await`
3. **Method calls**: `client.method().call().await`
4. **Server function**: `listen_async()` instead of `listen()`

### Examples

See the [`examples/async_example`](https://github.com/varlink/rust/tree/master/examples/async_example) directory for a complete working example demonstrating:
- Async server implementation
- Async client usage
- Integration with Tokio runtime
- Testing async services

## More Info

* [Git Repo](https://github.com/varlink/rust)
* [API Documentation](https://docs.rs/varlink)
* [Crate](https://crates.io/crates/varlink)
* [Example](https://github.com/varlink/rust/tree/master/examples) usage of this crate
