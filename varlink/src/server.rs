//! Handle network connections for a varlink service

use failure::Fail;
use {ErrorKind, Result};
//#![feature(getpid)]
//use std::process;
// FIXME
use libc;
use std::{self, env, fs, mem, thread};
use std::collections::HashMap;
use std::io::{BufReader, Error, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{mpsc, Arc, Mutex, RwLock};
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::UnixListener as AbstractUnixListener;

#[derive(Debug)]
enum Listener {
    TCP(Option<TcpListener>, bool),
    UNIX(Option<UnixListener>, bool),
}

#[derive(Debug)]
enum Stream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

impl<'a> Stream {
    #[allow(dead_code)]
    pub fn split(&mut self) -> Result<(Box<Read + Send + Sync>, Box<Write + Send + Sync>)> {
        match *self {
            Stream::TCP(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
            Stream::UNIX(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
        }
    }
    pub fn shutdown(&mut self) -> Result<()> {
        match *self {
            Stream::TCP(ref mut s) => s.shutdown(Shutdown::Both)?,
            Stream::UNIX(ref mut s) => s.shutdown(Shutdown::Both)?,
        }
        Ok(())
    }

    pub fn try_clone(&mut self) -> ::std::io::Result<Stream> {
        match *self {
            Stream::TCP(ref mut s) => Ok(Stream::TCP(s.try_clone()?)),
            Stream::UNIX(ref mut s) => Ok(Stream::UNIX(s.try_clone()?)),
        }
    }

    pub fn set_nonblocking(&mut self, b: bool) -> Result<()> {
        match *self {
            Stream::TCP(ref mut s) => Ok(s.set_nonblocking(b)?),
            Stream::UNIX(ref mut s) => Ok(s.set_nonblocking(b)?),
        }
    }

    pub fn as_raw_fd(&mut self) -> RawFd {
        match *self {
            Stream::TCP(ref mut s) => s.as_raw_fd(),
            Stream::UNIX(ref mut s) => s.as_raw_fd(),
        }
    }
}

impl ::std::io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize> {
        match *self {
            Stream::TCP(ref mut s) => s.write(buf),
            Stream::UNIX(ref mut s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> ::std::io::Result<()> {
        match *self {
            Stream::TCP(ref mut s) => s.flush(),
            Stream::UNIX(ref mut s) => s.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> ::std::io::Result<()> {
        match *self {
            Stream::TCP(ref mut s) => s.write_all(buf),
            Stream::UNIX(ref mut s) => s.write_all(buf),
        }
    }

    fn write_fmt(&mut self, fmt: ::std::fmt::Arguments) -> ::std::io::Result<()> {
        match *self {
            Stream::TCP(ref mut s) => s.write_fmt(fmt),
            Stream::UNIX(ref mut s) => s.write_fmt(fmt),
        }
    }
}

impl ::std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
        match *self {
            Stream::TCP(ref mut s) => s.read(buf),
            Stream::UNIX(ref mut s) => s.read(buf),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> ::std::io::Result<usize> {
        match *self {
            Stream::TCP(ref mut s) => s.read_to_end(buf),
            Stream::UNIX(ref mut s) => s.read_to_end(buf),
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> ::std::io::Result<usize> {
        match *self {
            Stream::TCP(ref mut s) => s.read_to_string(buf),
            Stream::UNIX(ref mut s) => s.read_to_string(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> ::std::io::Result<()> {
        match *self {
            Stream::TCP(ref mut s) => s.read_exact(buf),
            Stream::UNIX(ref mut s) => s.read_exact(buf),
        }
    }
}

fn activation_listener() -> Result<Option<i32>> {
    let nfds: u32;

    match env::var("LISTEN_FDS") {
        Ok(ref n) => match n.parse::<u32>() {
            Ok(n) if n >= 1 => nfds = n,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    }

    unsafe {
        match env::var("LISTEN_PID") {
            //FIXME: replace Ok(getpid()) with Ok(process::id())
            Ok(ref pid) if pid.parse::<i32>() == Ok(libc::getpid()) => {}
            _ => return Ok(None),
        }
    }

    if nfds == 1 {
        return Ok(Some(3));
    }

    let fdnames: String;

    match env::var("LISTEN_FDNAMES") {
        Ok(n) => {
            fdnames = n;
        }
        _ => return Ok(None),
    }

    for (i, v) in fdnames.split(':').enumerate() {
        if v == "varlink" {
            return Ok(Some(3 + i as i32));
        }
    }

    Ok(None)
}

impl Listener {
    pub fn new<S: ?Sized + AsRef<str>>(address: &S) -> Result<Self> {
        let address = address.as_ref();
        if let Some(l) = activation_listener()? {
            if address.starts_with("tcp:") {
                unsafe {
                    return Ok(Listener::TCP(
                        Some(TcpListener::from_raw_fd(l)),
                        true,
                    ));
                }
            } else if address.starts_with("unix:") {
                unsafe {
                    return Ok(Listener::UNIX(
                        Some(UnixListener::from_raw_fd(l)),
                        true,
                    ));
                }
            } else {
                return Err(ErrorKind::InvalidAddress.into());
            }
        }

        if address.starts_with("tcp:") {
            Ok(Listener::TCP(
                Some(TcpListener::bind(&address[4..])?),
                false,
            ))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(address[5..].split(';').next().unwrap());
            if addr.starts_with('@') {
                addr = addr.replacen('@', "\0", 1);
                let l = AbstractUnixListener::bind(addr)?;
                unsafe {
                    return Ok(Listener::UNIX(
                        Some(UnixListener::from_raw_fd(l.into_raw_fd())),
                        false,
                    ));
                }
            }
            // ignore error on non-existant file
            let _ = fs::remove_file(&*addr);
            let l = UnixListener::bind(addr)?;
            unsafe {
                Ok(Listener::UNIX(
                    Some(UnixListener::from_raw_fd(l.into_raw_fd())),
                    false,
                ))
            }
        } else {
            Err(ErrorKind::InvalidAddress.into())
        }
    }

    pub fn accept(&self, timeout: u64) -> Result<Stream> {
        if timeout > 0 {
            let fd = match self {
                Listener::TCP(Some(l), _) => l.as_raw_fd(),
                Listener::UNIX(Some(l), _) => l.as_raw_fd(),
                _ => return Err(ErrorKind::ConnectionClosed.into()),
            };

            unsafe {
                let mut readfs: libc::fd_set = mem::uninitialized();
                loop {
                    libc::FD_ZERO(&mut readfs);
                    let mut writefds: libc::fd_set = mem::uninitialized();
                    libc::FD_ZERO(&mut writefds);
                    let mut errorfds: libc::fd_set = mem::uninitialized();
                    libc::FD_ZERO(&mut errorfds);
                    let mut timeout = libc::timeval {
                        tv_sec: timeout as libc::time_t,
                        tv_usec: 0,
                    };

                    libc::FD_SET(fd, &mut readfs);
                    let ret = libc::select(
                        fd + 1,
                        &mut readfs,
                        &mut writefds,
                        &mut errorfds,
                        &mut timeout,
                    );
                    if ret != libc::EINTR && ret != libc::EAGAIN {
                        break;
                    }
                }
                if !libc::FD_ISSET(fd, &mut readfs) {
                    return Err(ErrorKind::Timeout.into());
                }
            }
        }
        match self {
            &Listener::TCP(Some(ref l), _) => {
                let (mut s, _addr) = l.accept()?;
                Ok(Stream::TCP(s))
            }
            Listener::UNIX(Some(ref l), _) => {
                let (mut s, _addr) = l.accept()?;
                Ok(Stream::UNIX(s))
            }
            _ => Err(ErrorKind::ConnectionClosed.into()),
        }
    }
    pub fn set_nonblocking(&self, b: bool) -> Result<()> {
        match *self {
            Listener::TCP(Some(ref l), _) => l.set_nonblocking(b)?,
            Listener::UNIX(Some(ref l), _) => l.set_nonblocking(b)?,
            _ => Err(ErrorKind::ConnectionClosed)?,
        }
        Ok(())
    }

    pub fn as_raw_fd(&self) -> RawFd {
        match *self {
            Listener::TCP(Some(ref l), _) => l.as_raw_fd(),
            Listener::UNIX(Some(ref l), _) => l.as_raw_fd(),
            _ => panic!("pattern `TCP(None, _)` not covered"),
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
                    unsafe {
                        let s = UnixStream::from_raw_fd(l.into_raw_fd());
                        let _ = s.set_read_timeout(None);
                    }
                }
            }
            Listener::TCP(ref mut listener, true) => {
                if let Some(l) = listener.take() {
                    unsafe {
                        let s = TcpStream::from_raw_fd(l.into_raw_fd());
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

type Job = Box<FnBox + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        let num_busy = Arc::new(RwLock::new(0 as usize));

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver), Arc::clone(&num_busy)));
        }

        ThreadPool {
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
        if (self.num_busy() + 1) >= self.workers.len() {
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

/// `listen` creates a server, with `num_worker` threads listening on `varlink_uri`.
///
/// If an `accept_timeout` != 0 is specified, this function returns after the specified
/// amount of seconds, if no new connection is made in that time frame. It still waits for
/// all pending connections to finish.
///
///# Examples
///
///```
/// extern crate failure;
/// extern crate varlink;
/// use failure::Fail;
///
/// let service = varlink::VarlinkService::new(
///     "org.varlink",
///     "test service",
///     "0.1",
///     "http://varlink.org",
///     vec![/* Your varlink interfaces go here */],
/// );
///
/// if let Err(e) = varlink::listen(service, "unix:/tmp/test_listen_timeout", 10, 1) {
///     if e.kind() != varlink::ErrorKind::Timeout {
///         panic!("Error listen: {:?}", e.cause());
///     }
/// }
///```
///# Note
/// You don't have to use this simple server. With the `VarlinkService::handle()` method you
/// can implement your own server model using whatever framework you prefer.
pub fn listen<S: ?Sized + AsRef<str>, H: ::ConnectionHandler + Send + Sync + 'static>(
    handler: H,
    address: &S,
    initial_worker_threads: usize,
    accept_timeout: u64,
) -> Result<()> {
    let handler = Arc::new(handler);
    let listener = Listener::new(address)?;
    listener.set_nonblocking(false)?;
    let mut pool = ThreadPool::new(initial_worker_threads);

    loop {
        let mut stream = match listener.accept(accept_timeout) {
            Err(e) => {
                if e.kind() == ErrorKind::Timeout {
                    if pool.num_busy() == 0 {
                        return Err(e);
                    }
                    continue;
                } else {
                    return Err(e);
                }
            }
            r => r?,
        };
        let handler = handler.clone();

        pool.execute(move || {
            if let Err(err) = handler.handle(
                &mut BufReader::new(stream.try_clone().expect("Could not split stream")),
                &mut stream,
            ) {
                if err.kind() != ErrorKind::ConnectionClosed {
                    eprintln!("Worker error: {}", err);
                    for cause in err.causes().skip(1) {
                        eprintln!("  caused by: {}", cause);
                    }
                }
                let _ = stream.shutdown();
            }
        });
    }
}

pub fn listen_multiplex<S: ?Sized + AsRef<str>, H: ::ConnectionHandler + Send + Sync + 'static>(
    handler: H,
    address: &S,
    accept_timeout: u64,
) -> Result<()> {
    let handler = Arc::new(handler);
    let mut fd_to_stream: HashMap<i32, Stream> = HashMap::new();
    let mut fds = Vec::new();

    let listener = Listener::new(address)?;
    listener.set_nonblocking(true)?;

    fds.push(libc::pollfd {
        fd: listener.as_raw_fd(),
        revents:0,
        events: libc::POLLIN,
    });

    loop {
        // Read activity on listening socket
        if fds[0].revents != 0 {
            let mut client = match listener.accept(accept_timeout) {
                Err(e) => {
                    if e.kind() == ErrorKind::Timeout {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(s) => s,
            };

            client.set_nonblocking(true)?;

            let fd = client.as_raw_fd();
            fds.push(libc::pollfd {
                fd,
                revents: 0,
                events: libc::POLLIN,
            });

            fd_to_stream.insert(fd, client);
        }

        // Store which indices to remove
        let mut indices_to_remove = vec!();

        // Check client connections ...
        for i in 1..fds.len() {
            if fds[i].revents != 0 {

                // It appears we are unable to leave the stream in the hashmap and have it
                // mutable for the call into handle, thus we remove and re-insert if we need too.
                // Perhaps there is a better way to handle this.
                let mut client = fd_to_stream.remove(&fds[i].fd).unwrap();
                let mut reader = BufReader::new(client.try_clone().expect("Could not split stream"));

                match handler.handle(&mut reader, &mut client) {
                    Ok(_) => {
                        // When we get an Ok result the socket went EOF, lets shut it down.
                        let _ = client.shutdown();
                        indices_to_remove.push(i);
                    },
                    Err(e) => {
                        match e.kind() {
                            ErrorKind::ConnectionClosed => {
                                let _ = client.shutdown();
                                indices_to_remove.push(i);
                            },
                            ErrorKind::Io(n) => {
                                if n == std::io::ErrorKind::WouldBlock {
                                    // Hmmmm, this is our indication that we can continue, stuff
                                    // the stream and associated fd back into mapping.
                                    fd_to_stream.insert(fds[i].fd, client);
                                } else {
                                    // Other IO errors, we will shut down.
                                    let _ = client.shutdown();
                                    indices_to_remove.push(i);
                                }
                            }
                            err => {
                                eprintln!("handler error: {}", err);
                                for cause in err.causes().skip(1) {
                                    eprintln!("  caused by: {}", cause);
                                }
                                let _ = client.shutdown();
                                indices_to_remove.push(i);
                            }
                        }
                    }
                }
            }
        }

        // We can't modify the vector while we are traversing it, so update now.
        for i in indices_to_remove {
            fds.remove(i);
        }

        let r = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::c_ulong, -1) };

        if r < 0 {
            return Err(Error::last_os_error().into());
        }
    }
}
