//! Handle network connections for a varlink service

#![allow(dead_code)]

use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::net::UnixStream;
use std::process::Child;
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::UnixStream as AbstractStream;

pub enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream, Option<Child>),
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn varlink_exec(_address: String) -> io::Result<(Child, String)> {
    unimplemented!();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn varlink_exec(address: String) -> io::Result<(Child, String)> {
    use unix_socket::os::linux::SocketAddrExt;
    use libc::close;
    use libc::dup2;
    use libc::getpid;
    use std::env;
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use unix_socket::UnixListener as AbstractUnixListener;

    let executable = &address[5..];
    let listener = AbstractUnixListener::bind("")?;
    let local_addr = listener.local_addr()?;
    let path = local_addr.as_abstract();
    let fd = listener.into_raw_fd();
    let child = Command::new(executable)
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
    Ok((
        child,
        format!("unix:@{}", String::from_utf8_lossy(path.unwrap())),
    ))
}

impl<'a> VarlinkStream {
    pub fn connect(address: &str) -> io::Result<(Self, String)> {
        let mut address: String = address.into();
        let mut my_child: Option<Child> = None;

        if address.starts_with("exec:") {
            let (c, a) = varlink_exec(address)?;
            address = a;
            my_child = Some(c);
        }

        if address.starts_with("tcp:") {
            Ok((
                VarlinkStream::TCP(TcpStream::connect(&address[4..])?),
                address,
            ))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(address[5..].split(";").next().unwrap());
            if addr.starts_with("@") {
                addr = addr.replacen("@", "\0", 1);
                let l = AbstractStream::connect(addr)?;
                unsafe {
                    return Ok((
                        VarlinkStream::UNIX(UnixStream::from_raw_fd(l.into_raw_fd()), my_child),
                        address,
                    ));
                }
            }
            Ok((
                VarlinkStream::UNIX(UnixStream::connect(addr)?, my_child),
                address,
            ))
        } else {
            Err(Error::new(ErrorKind::Other, "unknown varlink address"))
        }
    }

    pub fn split(&mut self) -> io::Result<(Box<Read + Send + Sync>, Box<Write + Send + Sync>)> {
        match *self {
            VarlinkStream::TCP(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
            VarlinkStream::UNIX(ref mut s, _) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
        }
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        match *self {
            VarlinkStream::TCP(ref mut s) => s.shutdown(Shutdown::Both),
            VarlinkStream::UNIX(ref mut s, _) => s.shutdown(Shutdown::Both),
        }
    }

    pub fn set_nonblocking(&self, b: bool) -> io::Result<()> {
        match self {
            &VarlinkStream::TCP(ref l) => l.set_nonblocking(b),
            &VarlinkStream::UNIX(ref l, _) => l.set_nonblocking(b),
        }
    }
}

impl Drop for VarlinkStream {
    fn drop(&mut self) {
        let _r = self.shutdown();
        match *self {
            VarlinkStream::UNIX(_, Some(ref mut child)) => {
                let _res = child.kill();
                let _res = child.wait();
            }
            _ => {}
        }
    }
}
