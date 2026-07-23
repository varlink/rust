//! Async server support for varlink using Tokio
//!
//! This module provides async versions of the varlink server functionality,
//! using the sans-io state machines for protocol handling and Tokio for I/O.

use crate::error::*;
use crate::sansio::Server;
use async_trait::async_trait;
use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

/// Check for systemd socket activation environment variables
///
/// Returns the file descriptor number if socket activation is detected.
/// Checks LISTEN_FDS, LISTEN_PID, and optionally LISTEN_FDNAMES.
fn activation_listener() -> Option<usize> {
    let nfds: usize = match env::var("LISTEN_FDS") {
        Ok(ref n) => match n.parse::<usize>() {
            Ok(n) if n >= 1 => n,
            _ => return None,
        },
        _ => return None,
    };

    match env::var("LISTEN_PID") {
        Ok(ref pid) if pid.parse::<usize>() == Ok(process::id() as usize) => {}
        _ => return None,
    }

    if nfds == 1 {
        return Some(3);
    }

    let fdnames = env::var("LISTEN_FDNAMES").ok()?;

    for (i, v) in fdnames.split(':').enumerate() {
        if v == "varlink" {
            return Some(3 + i);
        }
    }

    None
}

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

        // Check for socket activation first
        if let Some(fd) = activation_listener() {
            return Self::from_activation_fd(fd, address);
        }

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

    /// Create an async listener from a socket activation file descriptor
    #[cfg(unix)]
    fn from_activation_fd(fd: usize, address: &str) -> Result<Self> {
        use std::os::unix::io::FromRawFd;

        let fd = fd as std::os::unix::io::RawFd;

        // Validate that the fd is actually open before taking ownership
        // This prevents IO Safety violations if the fd wasn't properly inherited
        unsafe {
            if libc::fcntl(fd, libc::F_GETFD) == -1 {
                return Err(context!(ErrorKind::Io(std::io::ErrorKind::InvalidInput)));
            }
        }

        if address.starts_with("tcp:") {
            let std_listener = unsafe { std::net::TcpListener::from_raw_fd(fd) };
            std_listener
                .set_nonblocking(true)
                .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            let listener = TcpListener::from_std(std_listener)
                .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            Ok(AsyncListener::TCP(listener))
        } else if address.starts_with("unix:") {
            let std_listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(fd) };
            std_listener
                .set_nonblocking(true)
                .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            let listener = UnixListener::from_std(std_listener)
                .map_err(|e| context!(ErrorKind::Io(e.kind())))?;
            Ok(AsyncListener::UNIX(listener))
        } else {
            Err(context!(ErrorKind::InvalidAddress))
        }
    }

    #[cfg(not(unix))]
    fn from_activation_fd(_fd: usize, _address: &str) -> Result<Self> {
        Err(context!(ErrorKind::MethodNotImplemented(
            "socket activation".into()
        )))
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
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            AsyncStream::TCP(stream) => stream.read(buf).await,
            #[cfg(unix)]
            AsyncStream::UNIX(stream) => stream.read(buf).await,
        }
    }

    /// Write data to the stream
    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            AsyncStream::TCP(stream) => stream.write_all(buf).await,
            #[cfg(unix)]
            AsyncStream::UNIX(stream) => stream.write_all(buf).await,
        }
    }

    /// Flush the stream
    pub async fn flush(&mut self) -> std::io::Result<()> {
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

/// Trait for async varlink interfaces that provide introspection metadata.
///
/// This trait extends `AsyncConnectionHandler` with methods to get the interface
/// name and description, enabling introspection support when used with
/// `AsyncVarlinkService`.
///
/// # Examples
///
/// ```ignore
/// // Generated code will implement this trait automatically
/// impl AsyncInterface for VarlinkInterfaceHandler {
///     fn get_name(&self) -> &'static str {
///         "org.example.ping"
///     }
///
///     fn get_description(&self) -> &'static str {
///         r#"interface org.example.ping
/// method Ping(ping: string) -> (pong: string)
/// "#
///     }
/// }
/// ```
pub trait AsyncInterface: AsyncConnectionHandler {
    /// Returns the interface name (e.g., "org.example.ping")
    fn get_name(&self) -> &'static str;

    /// Returns the interface description (varlink IDL text)
    fn get_description(&self) -> &'static str;
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
pub async fn handle_connection<H: AsyncConnectionHandler>(
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

use std::borrow::Cow;
use std::collections::HashMap;

/// Async varlink service that wraps multiple async interfaces and provides introspection.
///
/// This is the async equivalent of `VarlinkService`. It manages multiple async interfaces
/// and automatically provides the `org.varlink.service` introspection methods:
/// - `GetInfo` - Returns service metadata and list of interfaces
/// - `GetInterfaceDescription` - Returns the varlink IDL for a given interface
///
/// # Examples
///
/// ```ignore
/// use std::sync::Arc;
/// use varlink::{listen_async, ListenAsyncConfig, AsyncVarlinkService};
///
/// // Create async interfaces (generated code)
/// let ping_handler = Arc::new(org_example_ping::new(Arc::new(MyPingService)));
/// let other_handler = Arc::new(org_example_other::new(Arc::new(MyOtherService)));
///
/// // Create the async service with introspection support
/// let service = Arc::new(AsyncVarlinkService::new(
///     "org.example",
///     "Example Service",
///     "1.0",
///     "http://example.org",
///     vec![ping_handler, other_handler],
/// ));
///
/// // Listen for connections
/// listen_async(service, "tcp:127.0.0.1:9999", &ListenAsyncConfig::default()).await?;
/// ```
pub struct AsyncVarlinkService {
    info: crate::ServiceInfo,
    ifaces: HashMap<Cow<'static, str>, Arc<dyn AsyncInterface>>,
}

impl AsyncVarlinkService {
    /// Create a new async varlink service with introspection support.
    ///
    /// # Arguments
    ///
    /// * `vendor` - The vendor name (e.g., "org.example")
    /// * `product` - The product name (e.g., "Example Service")
    /// * `version` - The version string (e.g., "1.0")
    /// * `url` - The URL for documentation (e.g., "http://example.org")
    /// * `interfaces` - A vector of async interface handlers
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let service = AsyncVarlinkService::new(
    ///     "org.example",
    ///     "Example Service",
    ///     "1.0",
    ///     "http://example.org",
    ///     vec![Arc::new(my_interface_handler)],
    /// );
    /// ```
    pub fn new<S: Into<Cow<'static, str>>>(
        vendor: S,
        product: S,
        version: S,
        url: S,
        interfaces: Vec<Arc<dyn AsyncInterface>>,
    ) -> Self {
        let mut ifhashmap = HashMap::<Cow<'static, str>, Arc<dyn AsyncInterface>>::new();
        for iface in interfaces {
            ifhashmap.insert(iface.get_name().into(), iface);
        }
        let mut ifnames: Vec<Cow<'static, str>> = vec!["org.varlink.service".into()];
        ifnames.extend(ifhashmap.keys().cloned());
        AsyncVarlinkService {
            info: crate::ServiceInfo {
                vendor: vendor.into(),
                product: product.into(),
                version: version.into(),
                url: url.into(),
                interfaces: ifnames,
            },
            ifaces: ifhashmap,
        }
    }

    /// Returns the varlink service interface description
    fn get_description(&self) -> &'static str {
        r#"# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

# Get a list of all the interfaces a service provides and information
# about the implementation.
method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: []string
)

# Get the description of an interface that is implemented by this service.
method GetInterfaceDescription(interface: string) -> (description: string)

# The requested interface was not found.
error InterfaceNotFound (interface: string)

# The requested method was not found
error MethodNotFound (method: string)

# The interface defines the requested method, but the service does not
# implement it.
error MethodNotImplemented (method: string)

# One of the passed parameters is invalid.
error InvalidParameter (parameter: string)
"#
    }
}

#[async_trait]
impl AsyncConnectionHandler for AsyncVarlinkService {
    async fn handle(
        &self,
        server: &mut Server,
        upgraded_iface: Option<String>,
    ) -> Result<Option<String>> {
        use crate::sansio::ServerEvent;
        use crate::Reply;

        // If already upgraded, delegate to the upgraded interface
        if let Some(ref iface_name) = upgraded_iface {
            if let Some(handler) = self.ifaces.get(iface_name.as_str()) {
                return handler.handle(server, upgraded_iface).await;
            }
            return Ok(upgraded_iface);
        }

        // Process events from the sans-io server
        while let Some(event) = server.poll_event() {
            match event {
                ServerEvent::Request { request } => {
                    // Extract the interface name from the method
                    let n: usize = match request.method.rfind('.') {
                        None => {
                            // No interface specified, send error
                            let reply = Reply {
                                parameters: None,
                                continues: None,
                                error: Some(
                                    format!(
                                        "org.varlink.service.InterfaceNotFound: {}",
                                        request.method
                                    )
                                    .into(),
                                ),
                            };
                            server.send_reply(reply)?;
                            continue;
                        }
                        Some(x) => x,
                    };

                    let iface = &request.method[..n];

                    // Handle the request based on the interface
                    if iface == "org.varlink.service" {
                        // Handle built-in service methods
                        match request.method.as_ref() {
                            "org.varlink.service.GetInfo" => {
                                let reply = Reply {
                                    parameters: Some(
                                        serde_json::to_value(&self.info)
                                            .map_err(crate::map_context!())?,
                                    ),
                                    continues: None,
                                    error: None,
                                };
                                server.send_reply(reply)?;
                            }
                            "org.varlink.service.GetInterfaceDescription" => {
                                if let Some(params) = request.parameters {
                                    let args: crate::GetInterfaceDescriptionArgs =
                                        serde_json::from_value(params)
                                            .map_err(crate::map_context!())?;
                                    let description = if args.interface == "org.varlink.service" {
                                        Some(self.get_description().to_string())
                                    } else if let Some(handler) =
                                        self.ifaces.get(args.interface.as_ref())
                                    {
                                        Some(handler.get_description().to_string())
                                    } else {
                                        let reply = Reply {
                                            parameters: None,
                                            continues: None,
                                            error: Some(
                                                "org.varlink.service.InvalidParameter: interface"
                                                    .to_string()
                                                    .into(),
                                            ),
                                        };
                                        server.send_reply(reply)?;
                                        continue;
                                    };
                                    let reply = Reply {
                                        parameters: Some(
                                            serde_json::json!({"description": description}),
                                        ),
                                        continues: None,
                                        error: None,
                                    };
                                    server.send_reply(reply)?;
                                } else {
                                    let reply = Reply {
                                        parameters: None,
                                        continues: None,
                                        error: Some(
                                            "org.varlink.service.InvalidParameter: parameters"
                                                .into(),
                                        ),
                                    };
                                    server.send_reply(reply)?;
                                }
                            }
                            _ => {
                                let reply = Reply {
                                    parameters: None,
                                    continues: None,
                                    error: Some(
                                        format!(
                                            "org.varlink.service.MethodNotFound: {}",
                                            request.method
                                        )
                                        .into(),
                                    ),
                                };
                                server.send_reply(reply)?;
                            }
                        }
                    } else if let Some(handler) = self.ifaces.get(iface) {
                        // Delegate to the interface handler
                        // We need to push this request back so the handler can process it
                        server.push_request(request);
                        return handler.handle(server, None).await;
                    } else {
                        // Interface not found
                        let reply = Reply {
                            parameters: None,
                            continues: None,
                            error: Some(
                                format!("org.varlink.service.InterfaceNotFound: {}", iface).into(),
                            ),
                        };
                        server.send_reply(reply)?;
                    }
                }
                ServerEvent::Upgrade { interface } => {
                    // Upgrade requested
                    return Ok(Some(interface));
                }
            }
        }

        Ok(None)
    }
}
