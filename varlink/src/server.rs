//! Handle network connections for a varlink service
#![allow(dead_code)]

use std::{env, fs, thread};
//#![feature(getpid)]
//use std::process;
use std::io::{BufRead, BufReader};
use std::mem;
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket};
use std::process;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex, RwLock,
};

#[cfg(windows)]
use uds_windows::UnixListener;

use crate::error::*;
use crate::stream::Stream;

#[derive(Debug)]
pub enum Listener {
    TCP(Option<TcpListener>, bool),
    UNIX(Option<UnixListener>, bool),
}

fn activation_listener() -> Option<usize> {
    let nfds: usize;

    match env::var("LISTEN_FDS") {
        Ok(ref n) => match n.parse::<usize>() {
            Ok(n) if n >= 1 => nfds = n,
            _ => return None,
        },
        _ => return None,
    }

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

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_abstract_unixlistener(addr: &str) -> Result<UnixListener> {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::SocketAddr;

    let addr = SocketAddr::from_abstract_name(addr).map_err(map_context!())?;
    UnixListener::bind_addr(&addr).map_err(map_context!())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn get_abstract_unixlistener(_addr: &str) -> Result<UnixListener> {
    Err(context!(ErrorKind::InvalidAddress))
}

impl Listener {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: ?Sized + AsRef<str>>(address: &S) -> Result<Self> {
        let address = address.as_ref();
        if let Some(l) = activation_listener() {
            #[cfg(windows)]
            {
                if address.starts_with("tcp:") {
                    unsafe {
                        return Ok(Listener::TCP(
                            Some(TcpListener::from_raw_socket(l as RawSocket)),
                            true,
                        ));
                    }
                } else if address.starts_with("unix:") {
                    unsafe {
                        return Ok(Listener::UNIX(
                            Some(UnixListener::from_raw_socket(l as RawSocket)),
                            true,
                        ));
                    }
                } else {
                    return Err(context!(ErrorKind::InvalidAddress));
                }
            }
            #[cfg(unix)]
            {
                if address.starts_with("tcp:") {
                    unsafe {
                        return Ok(Listener::TCP(
                            Some(TcpListener::from_raw_fd(l as RawFd)),
                            true,
                        ));
                    }
                } else if address.starts_with("unix:") {
                    unsafe {
                        return Ok(Listener::UNIX(
                            Some(UnixListener::from_raw_fd(l as RawFd)),
                            true,
                        ));
                    }
                } else {
                    return Err(context!(ErrorKind::InvalidAddress));
                }
            }
        }

        if let Some(addr) = address.strip_prefix("tcp:") {
            Ok(Listener::TCP(
                Some(TcpListener::bind(addr).map_err(map_context!())?),
                false,
            ))
        } else if let Some(addr) = address.strip_prefix("unix:@") {
            get_abstract_unixlistener(addr.split(';').next().unwrap_or(addr))
                .map(|v| Listener::UNIX(Some(v), false))
        } else if let Some(addr) = address.strip_prefix("unix:") {
            let addr = addr.split(';').next().unwrap_or(addr);
            // ignore error on non-existent file
            _ = fs::remove_file(addr);
            Ok(Listener::UNIX(
                Some(UnixListener::bind(addr).map_err(map_context!())?),
                false,
            ))
        } else {
            Err(context!(ErrorKind::InvalidAddress))
        }
    }

    pub const fn is_already_accepted(&self) -> bool {
        match *self {
            Self::TCP(_, value) => value,
            Self::UNIX(_, value) => value,
        }
    }

    #[cfg(windows)]
    pub fn accept(&self, timeout: u64) -> Result<Box<dyn Stream>> {
        use winapi::um::winsock2::WSAEINTR as EINTR;
        use winapi::um::winsock2::{fd_set, select, timeval};

        if timeout > 0 {
            let socket: usize =
                self.as_raw_socket()
                    .ok_or(context!(ErrorKind::ConnectionClosed))? as usize;

            let timeout = timeval {
                tv_sec: (timeout / 1000u64) as _,
                tv_usec: ((timeout % 1000u64) * 1000u64) as _,
            };

            unsafe {
                let mut readfs: fd_set = mem::MaybeUninit::zeroed().assume_init();
                loop {
                    readfs.fd_count = 1;
                    readfs.fd_array[0] = socket;

                    let mut writefds = mem::MaybeUninit::zeroed();
                    let mut errorfds = mem::MaybeUninit::zeroed();

                    let ret = select(
                        0,
                        &mut readfs,
                        writefds.as_mut_ptr(),
                        errorfds.as_mut_ptr(),
                        &timeout,
                    );
                    if ret != EINTR {
                        break;
                    }
                }

                if readfs.fd_count == 0 || readfs.fd_array[0] != socket {
                    return Err(context!(ErrorKind::Timeout));
                }
            }
        }

        match self {
            &Listener::TCP(Some(ref l), accepted) => {
                if accepted {
                    unsafe { Ok(Box::new(TcpStream::from_raw_fd(l.as_raw_fd()))) }
                } else {
                    let (s, _addr) = l.accept().map_err(map_context!())?;
                    Ok(Box::new(s))
                }
            }
            Listener::UNIX(Some(ref l), accepted) => {
                if *accepted {
                    unsafe { Ok(Box::new(UnixStream::from_raw_fd(l.as_raw_fd()))) }
                } else {
                    let (s, _addr) = l.accept().map_err(map_context!())?;
                    Ok(Box::new(s))
                }
            }
            _ => Err(context!(ErrorKind::ConnectionClosed)),
        }
    }

    #[cfg(unix)]
    pub fn accept(&self, timeout: u64) -> Result<Box<dyn Stream>> {
        use libc::{fd_set, select, timeval, EAGAIN, EINTR, FD_ISSET, FD_SET, FD_ZERO};

        if timeout > 0 {
            let fd = self
                .as_raw_fd()
                .ok_or(context!(ErrorKind::ConnectionClosed))?;

            let mut timeout = timeval {
                tv_sec: (timeout / 1000u64) as _,
                tv_usec: ((timeout % 1000u64) * 1000u64) as _,
            };

            unsafe {
                let mut readfs = mem::MaybeUninit::<fd_set>::uninit();
                loop {
                    FD_ZERO(readfs.as_mut_ptr());
                    readfs.assume_init();
                    let mut writefds = mem::MaybeUninit::<fd_set>::uninit();
                    FD_ZERO(writefds.as_mut_ptr());
                    writefds.assume_init();
                    let mut errorfds = mem::MaybeUninit::<fd_set>::uninit();
                    FD_ZERO(errorfds.as_mut_ptr());
                    errorfds.assume_init();

                    FD_SET(fd, readfs.as_mut_ptr());
                    let ret = select(
                        fd + 1,
                        readfs.as_mut_ptr(),
                        writefds.as_mut_ptr(),
                        errorfds.as_mut_ptr(),
                        &mut timeout,
                    );
                    if ret != EINTR && ret != EAGAIN {
                        break;
                    }
                }
                if !FD_ISSET(fd, readfs.as_mut_ptr()) {
                    return Err(context!(ErrorKind::Timeout));
                }
            }
        }
        match self {
            &Listener::TCP(Some(ref l), accepted) => {
                if accepted {
                    unsafe { Ok(Box::new(TcpStream::from_raw_fd(l.as_raw_fd()))) }
                } else {
                    let (s, _addr) = l.accept().map_err(map_context!())?;
                    Ok(Box::new(s))
                }
            }
            Listener::UNIX(Some(ref l), accepted) => {
                if *accepted {
                    unsafe { Ok(Box::new(UnixStream::from_raw_fd(l.as_raw_fd()))) }
                } else {
                    let (s, _addr) = l.accept().map_err(map_context!())?;
                    Ok(Box::new(s))
                }
            }
            _ => Err(context!(ErrorKind::ConnectionClosed)),
        }
    }

    pub fn set_nonblocking(&self, b: bool) -> Result<()> {
        match *self {
            Listener::TCP(Some(ref l), _) => l.set_nonblocking(b).map_err(map_context!())?,
            Listener::UNIX(Some(ref l), _) => l.set_nonblocking(b).map_err(map_context!())?,
            _ => return Err(context!(ErrorKind::ConnectionClosed)),
        }
        Ok(())
    }

    #[cfg(unix)]
    pub fn as_raw_fd(&self) -> Option<RawFd> {
        match *self {
            Listener::TCP(Some(ref l), _) => Some(l.as_raw_fd()),
            Listener::UNIX(Some(ref l), _) => Some(l.as_raw_fd()),
            _ => None,
        }
    }

    #[cfg(windows)]
    pub fn as_raw_socket(&self) -> Option<RawSocket> {
        match *self {
            Listener::TCP(Some(ref l), _) => Some(l.as_raw_socket()),
            Listener::UNIX(Some(ref l), _) => Some(l.as_raw_socket()),
            _ => None,
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        match *self {
            Listener::UNIX(Some(ref listener), false) => {
                if let Ok(local_addr) = listener.local_addr() {
                    if let Some(path) = local_addr.as_pathname() {
                        let _ = fs::remove_file(path);
                    }
                }
            }
            Listener::UNIX(ref mut listener, true) => {
                if let Some(l) = listener.take() {
                    #[cfg(unix)]
                    unsafe {
                        let s = UnixStream::from_raw_fd(l.into_raw_fd());
                        let _ = s.set_read_timeout(None);
                    }
                    #[cfg(windows)]
                    let _ = l;
                }
            }
            Listener::TCP(ref mut listener, true) => {
                if let Some(l) = listener.take() {
                    #[cfg(unix)]
                    unsafe {
                        let s = TcpStream::from_raw_fd(l.into_raw_fd());
                        let _ = s.set_read_timeout(None);
                    }
                    #[cfg(windows)]
                    unsafe {
                        let s = TcpStream::from_raw_socket(l.into_raw_socket());
                        let _ = s.set_read_timeout(None);
                    }
                }
            }
            _ => {}
        }
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

struct ThreadPool {
    max_workers: usize,
    workers: Vec<Worker>,
    num_busy: Arc<RwLock<usize>>,
    sender: mpsc::Sender<Message>,
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Job = Box<dyn FnBox + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The initial_worker is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the initial_worker is zero.
    pub fn new(initial_worker: usize, max_workers: usize) -> ThreadPool {
        assert!(initial_worker > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(initial_worker);

        let num_busy = Arc::new(RwLock::new(0_usize));

        for _ in 0..initial_worker {
            workers.push(Worker::new(Arc::clone(&receiver), Arc::clone(&num_busy)));
        }

        ThreadPool {
            max_workers,
            workers,
            sender,
            receiver,
            num_busy,
        }
    }

    pub fn execute<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
        if ((self.num_busy() + 1) >= self.workers.len()) && (self.workers.len() <= self.max_workers)
        {
            self.workers.push(Worker::new(
                Arc::clone(&self.receiver),
                Arc::clone(&self.num_busy),
            ));
        }
    }

    pub fn num_busy(&self) -> usize {
        let num_busy = self.num_busy.read().unwrap();
        *num_busy
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>, num_busy: Arc<RwLock<usize>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    {
                        let mut num_busy = num_busy.write().unwrap();
                        *num_busy += 1;
                    }
                    job.call_box();
                    {
                        let mut num_busy = num_busy.write().unwrap();
                        *num_busy -= 1;
                    }
                }
                Message::Terminate => {
                    break;
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

/// `ListenConfig` specifies the configuration parameters for [`varlink::listen`]
///
/// Examples:
///
/// ```rust
/// let l = varlink::ListenConfig::default();
/// assert_eq!(l.initial_worker_threads, 1);
/// assert_eq!(l.max_worker_threads, 100);
/// assert_eq!(l.idle_timeout, 0);
/// assert!(l.stop_listening.is_none());
/// ```
///
/// [`varlink::listen`]: fn.listen.html
pub struct ListenConfig {
    /// The amount of initial worker threads
    pub initial_worker_threads: usize,
    /// The maximum amount of worker threads
    pub max_worker_threads: usize,
    /// Time in seconds for the server to quit, when it is idle
    pub idle_timeout: u64,
    /// An optional AtomicBool as a global flag, which lets the server stop accepting new connections, when set to `true`
    pub stop_listening: Option<Arc<AtomicBool>>,
}

impl Default for ListenConfig {
    fn default() -> Self {
        ListenConfig {
            initial_worker_threads: 1,
            max_worker_threads: 100,
            idle_timeout: 0,
            stop_listening: None,
        }
    }
}

/// `listen` creates a server, with `num_worker` threads listening on `varlink_uri`.
///
/// If an `listen_config.idle_timeout` != 0 is specified, this function returns after the specified
/// amount of seconds, if no new connection is made in that time frame. It still waits for
/// all pending connections to finish.
///
///# Examples
///
///```
/// extern crate varlink;
///
/// let service = varlink::VarlinkService::new(
///     "org.varlink",
///     "test service",
///     "0.1",
///     "http://varlink.org",
///     vec![/* Your varlink interfaces go here */],
/// );
///
/// if let Err(e) = varlink::listen(
///         service,
///         "unix:test_listen_timeout",
///         &varlink::ListenConfig {
///             idle_timeout: 1,
///             ..Default::default()
///         },
///     ) {
///     if *e.kind() != varlink::ErrorKind::Timeout {
///         panic!("Error listen: {:?}", e);
///     }
/// }
///```
///# Note
/// You don't have to use this simple server. With the `VarlinkService::handle()` method you
/// can implement your own server model using whatever framework you prefer.
pub fn listen<S: ?Sized + AsRef<str>, H: crate::ConnectionHandler + Send + Sync + 'static>(
    handler: H,
    address: &S,
    listen_config: &ListenConfig,
) -> Result<()> {
    let handler = Arc::new(handler);
    let listener = Listener::new(address)?;

    listener.set_nonblocking(false)?;

    let mut pool = ThreadPool::new(
        listen_config.initial_worker_threads,
        listen_config.max_worker_threads,
    );

    loop {
        let mut to_wait = listen_config.idle_timeout * 1000;
        let wait_time = listen_config
            .stop_listening
            .as_ref()
            .map(|_| 100)
            .unwrap_or(to_wait);
        let mut stream = loop {
            match listener.accept(wait_time) {
                Err(e) => match e.kind() {
                    ErrorKind::Timeout => {
                        if let Some(stop) = listen_config.stop_listening.as_ref() {
                            if stop.load(Ordering::SeqCst) {
                                return Ok(());
                            }
                            if listen_config.idle_timeout == 0 {
                                continue;
                            }
                        }

                        if to_wait <= wait_time {
                            if pool.num_busy() == 0 {
                                return Err(e);
                            }
                            to_wait = listen_config.idle_timeout * 1000;
                        } else {
                            to_wait -= wait_time;
                        }

                        continue;
                    }
                    _ => {
                        return Err(e);
                    }
                },
                r => break r?,
            }
        };
        let handler = handler.clone();

        pool.execute(move || {
            let (r, mut w) = stream.split().unwrap();
            let mut br = BufReader::new(r);
            let mut iface: Option<String> = None;
            loop {
                match handler.handle(&mut br, &mut w, iface.clone()) {
                    Ok((_, i)) => {
                        iface = i;
                        match br.fill_buf() {
                            Err(_) => break,
                            Ok([]) => break,
                            _ => {}
                        }
                    }
                    Err(err) => {
                        match err.kind() {
                            ErrorKind::ConnectionClosed | ErrorKind::SerdeJsonDe(_) => {}
                            _ => {
                                eprintln!("Worker error: {:?}", err);
                            }
                        }
                        let _ = stream.shutdown();
                        break;
                    }
                }
            }
        });

        if listener.is_already_accepted() {
            return Ok(());
        }
    }
}
