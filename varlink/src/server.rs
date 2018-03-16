use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::thread;
use std::sync::Arc;
use std::net::{TcpListener, TcpStream};
use unix_socket::{UnixListener, UnixStream};

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
}

impl VarlinkListener {
    pub fn new(address: &str) -> io::Result<Self> {
        if address.starts_with("tcp:") {
            Ok(VarlinkListener::TCP(TcpListener::bind(&address[4..])?))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(&address[5..]);
            if addr.starts_with("@") {
                addr = addr.replacen("@", "\0", 1);
            }
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
}

pub fn listen(addr: &str, service: Arc<::VarlinkService>) -> io::Result<()> {
    println!("Listening on {}", addr);
    let listener = VarlinkListener::new(addr)?;

    loop {
        let mut stream = listener.accept()?;
        let service = service.clone();
        let _join = thread::spawn(move || -> io::Result<()> {
            let (mut r, mut w) = stream.split()?;
            if let Err(e) = service.handle(&mut r, &mut w) {
                println!("Handle Error: {}", e);
            }
            Ok(())
        });
    }
}
