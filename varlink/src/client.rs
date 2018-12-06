//! Handle network connections for a varlink service

#![allow(dead_code)]

#[cfg(unix)]
use libc::{close, dup2, getpid};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::process::Child;
use tempfile::TempDir;

#[cfg(windows)]
use uds_windows::UnixStream;
#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use {ErrorKind, Result};

pub enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

#[cfg(windows)]
pub fn varlink_exec<S: ?Sized + AsRef<str>>(
    _address: &S,
) -> Result<(Child, String, Option<TempDir>)> {
    return Err(ErrorKind::MethodNotImplemented("varlink_exec".into()).into());
}

#[cfg(unix)]
pub fn varlink_exec<S: ?Sized + AsRef<str>>(
    address: &S,
) -> Result<(Child, String, Option<TempDir>)> {
    use std::env;
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use tempfile::tempdir;

    let executable = String::from("exec ") + address.as_ref();

    use unix_socket::UnixListener;

    let dir = tempdir()?;
    let file_path = dir.path().join("varlink-socket");

    let listener = UnixListener::bind(file_path.clone())?;
    let fd = listener.into_raw_fd();

    let child = Command::new("sh")
        .arg("-c")
        .arg(executable)
        .before_exec({
            let file_path = file_path.clone();
            move || {
                unsafe {
                    if fd != 3 {
                        close(3);
                        dup2(fd, 3);
                    }
                    env::set_var("VARLINK_ADDRESS", format!("unix:{}", file_path.display()));
                    env::set_var("LISTEN_FDS", "1");
                    env::set_var("LISTEN_FDNAMES", "varlink");
                    env::set_var("LISTEN_PID", format!("{}", getpid()));
                }
                Ok(())
            }
        }).spawn()?;
    Ok((child, format!("unix:{}", file_path.display()), Some(dir)))
}

#[cfg(windows)]
pub fn varlink_bridge<S: ?Sized + AsRef<str>>(_address: &S) -> Result<(Child, VarlinkStream)> {
    return Err(ErrorKind::MethodNotImplemented("varlink_bridge".into()).into());
}

#[cfg(unix)]
pub fn varlink_bridge<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Child, VarlinkStream)> {
    use std::os::unix::io::FromRawFd;
    use std::process::Command;

    let executable = address.as_ref();
    // use unix_socket::UnixStream;
    let (stream0, stream1) = UnixStream::pair()?;
    let fd = stream1.into_raw_fd();
    let childin = unsafe { ::std::fs::File::from_raw_fd(fd) };
    let childout = unsafe { ::std::fs::File::from_raw_fd(fd) };

    let child = Command::new("sh")
        .arg("-c")
        .arg(executable)
        .stdin(childin)
        .stdout(childout)
        .spawn()?;
    Ok((child, VarlinkStream::UNIX(stream0)))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_abstract_unixstream(addr: &str) -> Result<UnixStream> {
    // FIXME: abstract unix domains sockets still not in std
    // FIXME: https://github.com/rust-lang/rust/issues/14194
    use std::os::unix::io::FromRawFd;
    use unix_socket::UnixStream as AbstractStream;

    unsafe {
        Ok(UnixStream::from_raw_fd(
            AbstractStream::connect(addr)?.into_raw_fd(),
        ))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn get_abstract_unixstream(_addr: &str) -> Result<UnixStream> {
    Err(ErrorKind::InvalidAddress.into())
}

impl<'a> VarlinkStream {
    pub fn connect<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Self, String)> {
        let address = address.as_ref();
        let new_address: String = address.into();

        if new_address.starts_with("tcp:") {
            Ok((
                VarlinkStream::TCP(TcpStream::connect(&new_address[4..])?),
                new_address,
            ))
        } else if new_address.starts_with("unix:") {
            let mut addr = String::from(new_address[5..].split(';').next().unwrap());
            if addr.starts_with('@') {
                addr = addr.replacen('@', "\0", 1);
                return get_abstract_unixstream(&addr)
                    .and_then(|v| Ok((VarlinkStream::UNIX(v), new_address)));
            }
            Ok((VarlinkStream::UNIX(UnixStream::connect(addr)?), new_address))
        } else {
            Err(ErrorKind::InvalidAddress)?
        }
    }

    pub fn split(&mut self) -> Result<(Box<Read + Send + Sync>, Box<Write + Send + Sync>)> {
        match *self {
            VarlinkStream::TCP(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
            VarlinkStream::UNIX(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        match *self {
            VarlinkStream::TCP(ref mut s) => s.shutdown(Shutdown::Both)?,
            VarlinkStream::UNIX(ref mut s) => s.shutdown(Shutdown::Both)?,
        }
        Ok(())
    }

    pub fn set_nonblocking(&self, b: bool) -> Result<()> {
        match *self {
            VarlinkStream::TCP(ref l) => l.set_nonblocking(b)?,
            VarlinkStream::UNIX(ref l) => l.set_nonblocking(b)?,
        }
        Ok(())
    }
}

impl Drop for VarlinkStream {
    fn drop(&mut self) {
        let _r = self.shutdown();
    }
}
