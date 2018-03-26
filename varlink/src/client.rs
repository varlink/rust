//! Handle network connections for a varlink service

use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::UnixStream as AbstractStream;

pub enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

impl<'a> VarlinkStream {
    pub fn connect(address: &str) -> io::Result<Self> {
        if address.starts_with("tcp:") {
            Ok(VarlinkStream::TCP(TcpStream::connect(&address[4..])?))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(address[5..].split(";").next().unwrap());
            if addr.starts_with("@") {
                addr = addr.replacen("@", "\0", 1);
                let l = AbstractStream::connect(addr)?;
                unsafe {
                    return Ok(VarlinkStream::UNIX(UnixStream::from_raw_fd(
                        l.into_raw_fd(),
                    )));
                }
            }
            Ok(VarlinkStream::UNIX(UnixStream::connect(addr)?))
        } else {
            Err(Error::new(ErrorKind::Other, "unknown varlink address"))
        }
    }

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

    pub fn set_nonblocking(&self, b: bool) -> io::Result<()> {
        match self {
            &VarlinkStream::TCP(ref l) => l.set_nonblocking(b),
            &VarlinkStream::UNIX(ref l) => l.set_nonblocking(b),
        }
    }
}
