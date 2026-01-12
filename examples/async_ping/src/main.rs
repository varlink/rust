//! Async Varlink Client Example using Tokio
//!
//! This example demonstrates how to use the generated async client and server API,
//! including introspection support via AsyncVarlinkService.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use varlink::{listen_async, AsyncVarlinkService, ListenAsyncConfig};
use varlink_stdinterfaces::org_varlink_service_async::VarlinkClientInterface as ServiceClientInterface;

// Include the generated code
mod org_example_ping;
use org_example_ping::{VarlinkClientInterface, VarlinkInterface};

/// Ping service implementation using the generated VarlinkInterface trait
struct PingService;

#[async_trait]
impl VarlinkInterface for PingService {
    async fn ping(
        &self,
        call: &mut dyn org_example_ping::Call_Ping,
        ping: String,
    ) -> varlink::Result<()> {
        println!("Server: Ping request with: '{}'", ping);
        call.reply(ping)
    }
}

/// Run a server with AsyncVarlinkService for introspection support
async fn run_server_with_introspection(addr: &str) -> Result<()> {
    println!("Server: Listening on {} (with introspection)", addr);

    // Create the ping service handler
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

    listen_async(
        service,
        format!("tcp:{}", addr),
        &ListenAsyncConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {:?}", e))
}

/// Async client that makes a Ping request using the generated async client API
async fn run_client(addr: &str, ping_message: &str) -> Result<()> {
    println!("Client: Connecting to {}", addr);

    let connection = varlink::AsyncConnection::with_address(format!("tcp:{}", addr))
        .await
        .map_err(|e| anyhow::anyhow!("Connection error: {:?}", e))?;

    let client = org_example_ping::VarlinkClient::new(connection);

    println!("Client: Sending Ping request: '{}'", ping_message);

    let reply = client
        .ping(ping_message.to_string())
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("Call error: {:?}", e))?;

    println!("Client: Pong response: '{}'", reply.pong);

    Ok(())
}

/// Demonstrate introspection by calling GetInfo and GetInterfaceDescription
async fn run_introspection_client(addr: &str) -> Result<()> {
    println!("Client: Connecting to {} for introspection", addr);

    let connection = varlink::AsyncConnection::with_address(format!("tcp:{}", addr))
        .await
        .map_err(|e| anyhow::anyhow!("Connection error: {:?}", e))?;

    let client = varlink_stdinterfaces::org_varlink_service_async::VarlinkClient::new(connection);

    // Call GetInfo to discover service metadata
    println!("Client: Calling org.varlink.service.GetInfo...");
    let info = client
        .get_info()
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("GetInfo error: {:?}", e))?;

    println!("  Vendor: {}", info.vendor);
    println!("  Product: {}", info.product);
    println!("  Version: {}", info.version);
    println!("  URL: {}", info.url);
    println!("  Interfaces: {:?}", info.interfaces);

    // Call GetInterfaceDescription for org.example.ping
    println!(
        "\nClient: Calling org.varlink.service.GetInterfaceDescription for 'org.example.ping'..."
    );
    let connection = varlink::AsyncConnection::with_address(format!("tcp:{}", addr))
        .await
        .map_err(|e| anyhow::anyhow!("Connection error: {:?}", e))?;
    let client = varlink_stdinterfaces::org_varlink_service_async::VarlinkClient::new(connection);

    let desc = client
        .get_interface_description("org.example.ping".to_string())
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("GetInterfaceDescription error: {:?}", e))?;

    println!("  Description:\n{}", desc.description);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:9999";

    // Spawn the server with introspection support
    let server_handle = tokio::spawn(async move {
        if let Err(e) = run_server_with_introspection(addr).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give the server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Demonstrate introspection
    println!("\n=== Running Introspection Example ===\n");
    run_introspection_client(addr).await?;

    // Run the regular client
    println!("\n=== Running Client Example ===\n");
    run_client(addr, "Hello, Async Varlink!").await?;

    // Run another client request
    println!("\n=== Running Second Client Example ===\n");
    run_client(addr, "Testing sans-io with Tokio").await?;

    // Keep the server running for a bit to handle requests
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Clean shutdown
    server_handle.abort();

    println!("\n=== Example Complete ===");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_async_ping() {
        let addr = "127.0.0.1:9998";

        // Create a stop flag for graceful shutdown
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        // Spawn server with graceful shutdown support and introspection
        let server_handle = tokio::spawn(async move {
            let ping_service = Arc::new(PingService);
            let ping_handler = Arc::new(org_example_ping::new(ping_service));

            // Use AsyncVarlinkService for introspection support
            let service = Arc::new(AsyncVarlinkService::new(
                "org.example",
                "Async Ping Test",
                "1.0",
                "https://github.com/varlink/rust",
                vec![ping_handler],
            ));

            let config = ListenAsyncConfig {
                idle_timeout: Duration::ZERO,
                stop_listening: Some(stop_clone),
            };
            listen_async(service, format!("tcp:{}", addr), &config)
                .await
                .ok();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test introspection
        let introspection_result = run_introspection_client(addr).await;
        assert!(
            introspection_result.is_ok(),
            "Introspection failed: {:?}",
            introspection_result.err()
        );

        // Test ping client
        let result = run_client(addr, "test").await;
        assert!(result.is_ok());

        // Signal server to stop
        stop.store(true, Ordering::SeqCst);

        // Wait for server to shut down
        tokio::time::sleep(Duration::from_millis(200)).await;

        server_handle.abort();
    }
}
