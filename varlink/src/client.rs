//! Handle network connections for a varlink service

#![allow(dead_code)]

use libc::close;
use libc::dup2;
// FIXME
use libc::getpid;
use std::env;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::Command;
use unix_socket::os::linux::SocketAddrExt;
use unix_socket::UnixListener as AbstractUnixListener;
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::UnixStream as AbstractStream;

pub enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

pub fn varlink_exec(address: String) -> io::Result<String> {
    let executable = &address[5..];
    let listener = AbstractUnixListener::bind("")?;
    let local_addr = listener.local_addr()?;
    let path = local_addr.as_abstract();
    let fd = listener.into_raw_fd();
    Command::new(executable)
        .arg(format!(
            "--varlink=unix:@{}",
            String::from_utf8_lossy(path.unwrap())
        ))
        .before_exec(move || {
            unsafe {
                if fd != 3 {
                    close(3);
                    dup2(fd, 3);
                }
                env::set_var("LISTEN_FDS", "1");
                env::set_var("LISTEN_FDNAMES", "varlink");
                env::set_var("LISTEN_PID", format!("{}", getpid()));
            }
            Ok(())
        })
        .spawn()?;
    Ok(format!("unix:@{}", String::from_utf8_lossy(path.unwrap())))
}

impl<'a> VarlinkStream {
    pub fn connect(address: &str) -> io::Result<Self> {
        let mut address: String = address.into();

        if address.starts_with("exec:") {
            address = varlink_exec(address)?;
        }

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
