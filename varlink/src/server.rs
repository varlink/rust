//! Handle network connections for a varlink service

//#![feature(getpid)]
//use std::process;
// FIXME
use libc::getpid;
use std::env;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::UnixListener as AbstractUnixListener;

enum VarlinkListener {
    TCP(TcpListener),
    UNIX(UnixListener),
}

enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

impl<'a> VarlinkStream {
    pub fn split(&mut self) -> io::Result<(Box<Read + Send + Sync>, Box<Write + Send + Sync>)> {
        match *self {
            VarlinkStream::TCP(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
            VarlinkStream::UNIX(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
        }
    }
    pub fn shutdown(&mut self) -> io::Result<()> {
        match *self {
            VarlinkStream::TCP(ref mut s) => s.shutdown(Shutdown::Both),
            VarlinkStream::UNIX(ref mut s) => s.shutdown(Shutdown::Both),
        }
    }
}

fn activation_listener() -> io::Result<Option<i32>> {
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
            Ok(ref pid) if pid.parse::<i32>() == Ok(getpid()) => {}
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

    for (i, v) in fdnames.split(":").enumerate() {
        if v == "varlink" {
            return Ok(Some(3 + i as i32));
        }
    }

    Ok(None)
}

//FIXME: add Drop with shutdown() and unix file removal
impl VarlinkListener {
    pub fn new(address: &str) -> io::Result<Self> {
        if let Some(l) = activation_listener()? {
            if address.starts_with("tcp:") {
                unsafe {
                    return Ok(VarlinkListener::TCP(TcpListener::from_raw_fd(l)));
                }
            } else if address.starts_with("unix:") {
                unsafe {
                    return Ok(VarlinkListener::UNIX(UnixListener::from_raw_fd(l)));
                }
            } else {
                return Err(Error::new(ErrorKind::Other, "unknown varlink address"));
            }
        }

        if address.starts_with("tcp:") {
            Ok(VarlinkListener::TCP(TcpListener::bind(&address[4..])?))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(address[5..].split(";").next().unwrap());
            if addr.starts_with("@") {
                addr = addr.replacen("@", "\0", 1);
                let l = AbstractUnixListener::bind(addr)?;
                unsafe {
                    return Ok(VarlinkListener::UNIX(UnixListener::from_raw_fd(
                        l.into_raw_fd(),
                    )));
                }
            }
            // ignore error on non-existant file
            let _ = fs::remove_file(&*addr);
            Ok(VarlinkListener::UNIX(UnixListener::bind(addr)?))
        } else {
            Err(Error::new(ErrorKind::Other, "unknown varlink address"))
        }
    }

    pub fn accept(&self) -> io::Result<VarlinkStream> {
        match self {
            &VarlinkListener::TCP(ref l) => {
                let (mut s, _addr) = l.accept()?;
                Ok(VarlinkStream::TCP(s))
            }
            &VarlinkListener::UNIX(ref l) => {
                let (mut s, _addr) = l.accept()?;
                Ok(VarlinkStream::UNIX(s))
            }
        }
    }
    pub fn set_nonblocking(&self, b: bool) -> io::Result<()> {
        match self {
            &VarlinkListener::TCP(ref l) => l.set_nonblocking(b),
            &VarlinkListener::UNIX(ref l) => l.set_nonblocking(b),
        }
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
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

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).unwrap();
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
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    job.call_box();
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

pub enum ServerError {
    IoError(Error),
    AcceptTimeout,
}

impl From<Error> for ServerError {
    fn from(e: Error) -> Self {
        ServerError::IoError(e)
    }
}

pub fn do_listen(
    service: ::VarlinkService,
    addr: &str,
    workers: usize,
    accept_timeout: u64,
) -> Result<(), ServerError> {
    let service = Arc::new(service);
    let listener = Arc::new(VarlinkListener::new(addr)?);
    listener.set_nonblocking(false)?;
    let pool = ThreadPool::new(workers);

    loop {
        let mut stream: VarlinkStream;
        if accept_timeout != 0 {
            let listener = listener.clone();
            let (sender, receiver) = mpsc::channel();

            let _t = thread::spawn(move || {
                match sender.send(listener.accept()) {
                    Ok(()) => {} // everything good
                    Err(_) => {} // we have been released, don't panic
                }
            });

            stream = match receiver.recv_timeout(Duration::from_secs(accept_timeout)) {
                Ok(s) => s?,
                Err(_) => return Err(ServerError::AcceptTimeout),
            };
        } else {
            stream = listener.accept()?;
        }

        let service = service.clone();

        pool.execute(move || {
            let (mut r, mut w) = stream.split().expect("Could not split stream");

            if let Err(_e) = service.handle(&mut r, &mut w) {
                //println!("Handle Error: {}", e);
                let _ = stream.shutdown();
            }
        });
    }
}
