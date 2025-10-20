//! Async server support for varlink using Tokio
//!
//! This module provides async versions of the varlink server functionality,
//! using the sans-io state machines for protocol handling and Tokio for I/O.

use crate::error::*;
use crate::sansio::Server;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

/// Async listener for varlink connections
#[derive(Debug)]
pub enum AsyncListener {
    /// TCP listener
    TCP(TcpListener),
    /// Unix domain socket listener
    #[cfg(unix)]
    UNIX(UnixListener),
}

impl AsyncListener {
    /// Create a new async listener from an address string
    ///
    /// Supported formats:
    /// - `tcp:host:port` - TCP listener
    /// - `unix:/path/to/socket` - Unix domain socket
    /// - `unix:@abstract` - Abstract Unix socket (Linux only)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// # use varlink::server_async::AsyncListener;
    /// # #[tokio::main]
    /// # async fn main() -> varlink::Result<()> {
    /// let listener = AsyncListener::new("tcp:127.0.0.1:9999").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new<S: AsRef<str>>(address: S) -> Result<Self> {
        let address = address.as_ref();

        if let Some(addr) = address.strip_prefix("tcp:") {
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            Ok(AsyncListener::TCP(listener))
        } else if let Some(addr) = address.strip_prefix("unix:") {
            #[cfg(unix)]
            {
                Self::create_unix_listener(addr).await
            }
            #[cfg(not(unix))]
            {
                let _ = addr;
                Err(context!(ErrorKind::InvalidAddress))
            }
        } else {
            Err(context!(ErrorKind::InvalidAddress))
        }
    }

    #[cfg(unix)]
    async fn create_unix_listener(addr: &str) -> Result<Self> {
        use std::fs;

        if let Some(abstract_addr) = addr.strip_prefix('@') {
            // Abstract socket (Linux only)
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                let addr = abstract_addr.split(';').next().unwrap_or(abstract_addr);
                // On Linux, we can bind to abstract sockets by prefixing with null byte
                let socket_path = format!("\0{}", addr);
                let listener = UnixListener::bind(socket_path)
                    .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
                Ok(AsyncListener::UNIX(listener))
            }
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            {
                let _ = abstract_addr;
                Err(context!(ErrorKind::InvalidAddress))
            }
        } else {
            // File-based Unix socket
            let addr = addr.split(';').next().unwrap_or(addr);
            // Remove existing socket file if it exists
            let _ = fs::remove_file(addr);
            let listener =
                UnixListener::bind(addr).map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            Ok(AsyncListener::UNIX(listener))
        }
    }

    /// Accept a new connection
    ///
    /// Returns a boxed async stream (either TcpStream or UnixStream)
    pub async fn accept(&self) -> Result<AsyncStream> {
        match self {
            AsyncListener::TCP(listener) => {
                let (stream, _) = listener
                    .accept()
                    .await
                    .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
                Ok(AsyncStream::TCP(stream))
            }
            #[cfg(unix)]
            AsyncListener::UNIX(listener) => {
                let (stream, _) = listener
                    .accept()
                    .await
                    .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
                Ok(AsyncStream::UNIX(stream))
            }
        }
    }
}

/// Async stream wrapper for TCP and Unix domain sockets
pub enum AsyncStream {
    /// TCP stream
    TCP(TcpStream),
    /// Unix domain socket stream
    #[cfg(unix)]
    UNIX(UnixStream),
}

impl AsyncStream {
    /// Read data from the stream
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            AsyncStream::TCP(stream) => stream.read(buf).await,
            #[cfg(unix)]
            AsyncStream::UNIX(stream) => stream.read(buf).await,
        }
    }

    /// Write data to the stream
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            AsyncStream::TCP(stream) => stream.write_all(buf).await,
            #[cfg(unix)]
            AsyncStream::UNIX(stream) => stream.write_all(buf).await,
        }
    }

    /// Flush the stream
    async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            AsyncStream::TCP(stream) => stream.flush().await,
            #[cfg(unix)]
            AsyncStream::UNIX(stream) => stream.flush().await,
        }
    }
}

/// Async connection handler trait
///
/// Implement this trait to handle varlink requests asynchronously.
/// The handler is called for each request event from the sans-io server.
#[async_trait]
pub trait AsyncConnectionHandler: Send + Sync {
    /// Handle a server event
    ///
    /// This method is called for each `ServerEvent::Request` received.
    /// The implementation should process the request and send replies
    /// using `server.send_reply()`.
    ///
    /// # Arguments
    ///
    /// * `server` - The sans-io server state machine
    /// * `upgraded_iface` - Optional interface name if connection was upgraded
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(interface))` if the connection was upgraded,
    /// `Ok(None)` if the request was handled normally, or an error.
    async fn handle(
        &self,
        server: &mut Server,
        upgraded_iface: Option<String>,
    ) -> Result<Option<String>>;
}

/// Configuration for async listener
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "tokio")]
/// use varlink::ListenAsyncConfig;
/// use std::time::Duration;
///
/// let config = ListenAsyncConfig {
///     idle_timeout: Duration::from_secs(60),
///     ..Default::default()
/// };
/// ```
pub struct ListenAsyncConfig {
    /// Time to wait for new connections before shutting down if idle
    ///
    /// If set to zero (default), the server will not timeout.
    pub idle_timeout: Duration,

    /// Optional flag to stop accepting new connections
    ///
    /// When set to `true`, the server will gracefully shut down.
    pub stop_listening: Option<Arc<AtomicBool>>,
}

impl Default for ListenAsyncConfig {
    fn default() -> Self {
        ListenAsyncConfig {
            idle_timeout: Duration::ZERO,
            stop_listening: None,
        }
    }
}

/// Async varlink server
///
/// Creates an async server that listens for varlink connections and handles
/// them using the provided handler. This function uses the sans-io state machines
/// for protocol handling and Tokio for async I/O.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "tokio")]
/// # use varlink::{listen_async, ListenAsyncConfig, VarlinkService};
/// # use std::sync::Arc;
/// # #[tokio::main]
/// # async fn main() -> varlink::Result<()> {
/// let service = VarlinkService::new(
///     "org.example",
///     "Example Service",
///     "1.0",
///     "http://example.org",
///     vec![],
/// );
///
/// listen_async(
///     Arc::new(service),
///     "tcp:127.0.0.1:9999",
///     &ListenAsyncConfig::default(),
/// ).await
/// # }
/// ```
pub async fn listen_async<S: AsRef<str>, H: AsyncConnectionHandler + 'static>(
    handler: Arc<H>,
    address: S,
    config: &ListenAsyncConfig,
) -> Result<()> {
    let listener = AsyncListener::new(address).await?;
    let mut active_connections = 0usize;

    loop {
        // Wait for new connection with timeout if configured
        let stream = if config.idle_timeout.as_secs() > 0 || config.stop_listening.is_some() {
            let timeout_duration = if config.stop_listening.is_some() {
                Duration::from_millis(100)
            } else {
                config.idle_timeout
            };

            match tokio::time::timeout(timeout_duration, listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Timeout occurred
                    if let Some(stop) = &config.stop_listening {
                        if stop.load(Ordering::SeqCst) {
                            // Graceful shutdown requested
                            return Ok(());
                        }
                    }

                    if config.idle_timeout.as_secs() > 0 && active_connections == 0 {
                        // Idle timeout with no active connections
                        return Err(context!(ErrorKind::Timeout));
                    }

                    continue;
                }
            }
        } else {
            listener.accept().await?
        };

        // Spawn a task to handle the connection
        let handler = Arc::clone(&handler);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, handler).await {
                match e.kind() {
                    ErrorKind::ConnectionClosed => {}
                    _ => eprintln!("Connection error: {:?}", e),
                }
            }
        });

        active_connections += 1;
    }
}

/// Handle a single async connection
async fn handle_connection<H: AsyncConnectionHandler>(
    mut stream: AsyncStream,
    handler: Arc<H>,
) -> Result<()> {
    let mut server = Server::new();
    let mut buf = vec![0u8; 8192];
    let mut upgraded_iface: Option<String> = None;

    loop {
        // Read data from the stream
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|_| context!(ErrorKind::ConnectionClosed))?;

        if n == 0 {
            // Connection closed
            return Ok(());
        }

        // Feed data into the sans-io server
        server.handle_input(&buf[..n])?;

        // Let the handler process all pending events
        upgraded_iface = handler.handle(&mut server, upgraded_iface.clone()).await?;

        // Send all pending transmits
        while let Some(transmit) = server.poll_transmit() {
            stream
                .write_all(&transmit.payload)
                .await
                .map_err(|_| context!(ErrorKind::ConnectionClosed))?;
            stream
                .flush()
                .await
                .map_err(|_| context!(ErrorKind::ConnectionClosed))?;
        }
    }
}
