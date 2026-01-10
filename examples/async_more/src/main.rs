//! Async Varlink "More" Example using Tokio
//!
//! This example demonstrates multi-reply functionality in async mode,
//! using the `more` flag in requests and `continues` flag in responses.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use varlink::{listen_async, AsyncVarlinkService, ListenAsyncConfig};

// Include the generated code
mod org_example_more;
use org_example_more::{State, VarlinkClientInterface, VarlinkInterface};

/// More service implementation using the generated async VarlinkInterface trait
struct MoreService {
    sleep_duration: Duration,
}

#[async_trait]
impl VarlinkInterface for MoreService {
    async fn ping(
        &self,
        call: &mut dyn org_example_more::Call_Ping,
        ping: String,
    ) -> varlink::Result<()> {
        println!("Server: Ping request with: '{}'", ping);
        call.reply(ping)
    }

    async fn test_more(
        &self,
        call: &mut dyn org_example_more::Call_TestMore,
        n: i64,
    ) -> varlink::Result<()> {
        println!("Server: TestMore request with n={}", n);

        // Check if the client requested multiple replies
        if !call.wants_more() {
            println!("Server: Error - client did not request 'more'");
            return call.reply_test_more_error("called without more".into());
        }

        if n == 0 {
            return call.reply_test_more_error("n == 0".into());
        }

        // Indicate that more replies are coming
        call.set_continues(true);

        // Send start state
        call.reply(State {
            start: Some(true),
            end: None,
            progress: None,
        })?;
        println!("Server: Sent start state");

        // Send progress states
        for i in 0..n {
            tokio::time::sleep(self.sleep_duration).await;
            let progress = i * 100 / n;
            call.reply(State {
                progress: Some(progress),
                start: None,
                end: None,
            })?;
            println!("Server: Sent progress {}%", progress);
        }

        // Send 100% progress
        call.reply(State {
            progress: Some(100),
            start: None,
            end: None,
        })?;
        println!("Server: Sent progress 100%");

        // Final reply - no more continues
        call.set_continues(false);

        call.reply(State {
            end: Some(true),
            progress: None,
            start: None,
        })?;
        println!("Server: Sent end state");

        Ok(())
    }

    async fn stop_serving(
        &self,
        call: &mut dyn org_example_more::Call_StopServing,
    ) -> varlink::Result<()> {
        call.reply()?;
        Err(varlink::ErrorKind::ConnectionClosed.into())
    }
}

/// Run a server with AsyncVarlinkService for introspection support
async fn run_server(addr: &str, sleep_ms: u64) -> Result<()> {
    println!("Server: Listening on {} (with introspection)", addr);

    let more_service = Arc::new(MoreService {
        sleep_duration: Duration::from_millis(sleep_ms),
    });
    let more_handler = Arc::new(org_example_more::new(more_service));

    // Wrap with AsyncVarlinkService for introspection support
    let service = Arc::new(AsyncVarlinkService::new(
        "org.example",
        "Async More Example",
        "1.0",
        "https://github.com/varlink/rust",
        vec![more_handler],
    ));

    listen_async(
        service,
        format!("tcp:{}", addr),
        &ListenAsyncConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {:?}", e))
}

/// Async client that makes a TestMore request and receives multiple replies
async fn run_client(addr: &str, n: i64) -> Result<()> {
    println!("Client: Connecting to {}", addr);

    let connection = varlink::AsyncConnection::with_address(format!("tcp:{}", addr))
        .await
        .map_err(|e| anyhow::anyhow!("Connection error: {:?}", e))?;

    let client = org_example_more::VarlinkClient::new(connection);

    println!("Client: Sending TestMore request with n={}", n);

    // Use .more() to indicate we want multiple replies
    let mut method_call = client.test_more(n);
    method_call
        .more()
        .await
        .map_err(|e| anyhow::anyhow!("More error: {:?}", e))?;

    // Receive all replies
    loop {
        let reply = method_call
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Recv error: {:?}", e))?;

        let state = reply.state;
        match state {
            State {
                start: Some(true),
                end: None,
                progress: None,
            } => {
                println!("Client: --- Start ---");
            }
            State {
                start: None,
                end: Some(true),
                progress: None,
            } => {
                println!("Client: --- End ---");
                break; // This is the last reply
            }
            State {
                start: None,
                end: None,
                progress: Some(progress),
            } => {
                println!("Client: Progress: {}%", progress);
            }
            _ => {
                println!("Client: Got unknown state: {:?}", state);
            }
        }

        // Check if there are more replies coming
        if !method_call.continues() {
            println!("Client: No more replies expected");
            break;
        }
    }

    println!("Client: Done receiving replies");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:9997";
    let n = 5; // Number of progress steps

    // Spawn the server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = run_server(addr, 100).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run the client
    println!("\n=== Running Async More Example ===\n");
    run_client(addr, n).await?;

    // Clean shutdown
    server_handle.abort();

    println!("\n=== Example Complete ===");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_async_more() {
        let addr = "127.0.0.1:9996";

        // Create a stop flag for graceful shutdown
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        // Spawn server
        let server_handle = tokio::spawn(async move {
            let more_service = Arc::new(MoreService {
                sleep_duration: Duration::from_millis(10),
            });
            let more_handler = Arc::new(org_example_more::new(more_service));

            let service = Arc::new(AsyncVarlinkService::new(
                "org.example",
                "Async More Test",
                "1.0",
                "https://github.com/varlink/rust",
                vec![more_handler],
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

        // Test multi-reply client
        let result = run_client(addr, 3).await;
        assert!(
            result.is_ok(),
            "Multi-reply client failed: {:?}",
            result.err()
        );

        // Signal server to stop
        stop.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(200)).await;
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_wants_more_check() {
        let addr = "127.0.0.1:9995";

        // Create a stop flag for graceful shutdown
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        // Spawn server
        let server_handle = tokio::spawn(async move {
            let more_service = Arc::new(MoreService {
                sleep_duration: Duration::from_millis(10),
            });
            let more_handler = Arc::new(org_example_more::new(more_service));

            let service = Arc::new(AsyncVarlinkService::new(
                "org.example",
                "Async More Test",
                "1.0",
                "https://github.com/varlink/rust",
                vec![more_handler],
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

        // Try to call TestMore without .more() - should get an error
        let connection = varlink::AsyncConnection::with_address(format!("tcp:{}", addr))
            .await
            .expect("Connection failed");

        let client = org_example_more::VarlinkClient::new(connection);

        // Call without .more() - should return TestMoreError
        let result = client.test_more(5).call().await;
        assert!(result.is_err(), "Expected error when calling without more");

        // Signal server to stop
        stop.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(200)).await;
        server_handle.abort();
    }
}
