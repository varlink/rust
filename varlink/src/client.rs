//! Handle network connections for a varlink service

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::process::Child;

use chainerror::*;
#[cfg(unix)]
use libc::{close, dup2, getpid};
use tempfile::TempDir;
#[cfg(windows)]
use uds_windows::UnixStream;

use crate::error::*;

#[derive(Debug)]
pub enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

#[cfg(windows)]
pub(crate) fn varlink_exec<S: ?Sized + AsRef<str>>(
    _address: &S,
) -> Result<(Child, String, Option<TempDir>)> {
    return Err(into_cherr!(ErrorKind::MethodNotImplemented(
        "varlink_exec".into()
    )));
}

#[cfg(unix)]
pub(crate) fn varlink_exec<S: ?Sized + AsRef<str>>(
    address: &S,
) -> Result<(Child, String, Option<TempDir>)> {
    use std::env;
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use tempfile::tempdir;

    let executable = String::from("exec ") + address.as_ref();

    use unix_socket::UnixListener;

    let dir = tempdir().map_err(minto_cherr!())?;
    let file_path = dir.path().join("varlink-socket");

    let listener = UnixListener::bind(file_path.clone()).map_err(minto_cherr!())?;
    let fd = listener.into_raw_fd();

    let child = Command::new("sh")
        .arg("-c")
        .arg(executable)
        .before_exec({
            let file_path = file_path.clone();
            move || {
                unsafe {
                    if fd != 3 {
                        let _ = close(3);
                        let _ = dup2(fd, 3);
                    }
                    env::set_var("VARLINK_ADDRESS", format!("unix:{}", file_path.display()));
                    env::set_var("LISTEN_FDS", "1");
                    env::set_var("LISTEN_FDNAMES", "varlink");
                    env::set_var("LISTEN_PID", format!("{}", getpid()));
                }
                Ok(())
            }
        })
        .spawn()
        .map_err(minto_cherr!())?;
    Ok((child, format!("unix:{}", file_path.display()), Some(dir)))
}

#[cfg(windows)]
pub(crate) fn varlink_bridge<S: ?Sized + AsRef<str>>(
    address: &S,
) -> Result<(Child, VarlinkStream)> {
    use std::io::copy;
    use std::process::{Command, Stdio};
    use std::thread;

    let (stream0, stream1) = UnixStream::pair().map_err(minto_cherr!())?;
    let executable = address.as_ref();

    let mut child = Command::new("cmd")
        .arg("/C")
        .arg(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(minto_cherr!())?;

    let mut client_writer = child.stdin.take().unwrap();
    let mut client_reader = child.stdout.take().unwrap();
    let mut service_writer = stream1.try_clone().map_err(minto_cherr!())?;
    let mut service_reader = stream1;

    thread::spawn(move || copy(&mut client_reader, &mut service_writer));
    thread::spawn(move || copy(&mut service_reader, &mut client_writer));

    Ok((child, VarlinkStream::UNIX(stream0)))
}

#[cfg(unix)]
pub(crate) fn varlink_bridge<S: ?Sized + AsRef<str>>(
    address: &S,
) -> Result<(Child, VarlinkStream)> {
    use std::os::unix::io::FromRawFd;
    use std::process::Command;

    let executable = address.as_ref();
    // use unix_socket::UnixStream;
    let (stream0, stream1) = UnixStream::pair().map_err(minto_cherr!())?;
    let fd = stream1.into_raw_fd();
    let childin = unsafe { ::std::fs::File::from_raw_fd(fd) };
    let childout = unsafe { ::std::fs::File::from_raw_fd(fd) };

    let child = Command::new("sh")
        .arg("-c")
        .arg(executable)
        .stdin(childin)
        .stdout(childout)
        .spawn()
        .map_err(minto_cherr!())?;
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
            AbstractStream::connect(addr)
                .map_err(minto_cherr!())?
                .into_raw_fd(),
        ))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn get_abstract_unixstream(_addr: &str) -> Result<UnixStream> {
    Err(into_cherr!(ErrorKind::InvalidAddress))
}

impl VarlinkStream {
    pub fn connect<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Self, String)> {
        let address = address.as_ref();
        let new_address: String = address.into();

        if new_address.starts_with("tcp:") {
            Ok((
                VarlinkStream::TCP(TcpStream::connect(&new_address[4..]).map_err(minto_cherr!())?),
                new_address,
            ))
        } else if new_address.starts_with("unix:") {
            let mut addr = String::from(new_address[5..].split(';').next().unwrap());
            if addr.starts_with('@') {
                addr = addr.replacen('@', "\0", 1);
                return get_abstract_unixstream(&addr)
                    .and_then(|v| Ok((VarlinkStream::UNIX(v), new_address)));
            }
            Ok((
                VarlinkStream::UNIX(UnixStream::connect(addr).map_err(minto_cherr!())?),
                new_address,
            ))
        } else {
            Err(into_cherr!(ErrorKind::InvalidAddress))?
        }
    }

    pub fn split(&mut self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        match *self {
            VarlinkStream::TCP(ref mut s) => Ok((
                Box::new(s.try_clone().map_err(minto_cherr!())?),
                Box::new(s.try_clone().map_err(minto_cherr!())?),
            )),
            VarlinkStream::UNIX(ref mut s) => Ok((
                Box::new(s.try_clone().map_err(minto_cherr!())?),
                Box::new(s.try_clone().map_err(minto_cherr!())?),
            )),
        }
    }

    pub fn shutdown(&mut self) -> Result<()> {
        match *self {
            VarlinkStream::TCP(ref mut s) => s.shutdown(Shutdown::Both).map_err(minto_cherr!())?,
            VarlinkStream::UNIX(ref mut s) => s.shutdown(Shutdown::Both).map_err(minto_cherr!())?,
        }
        Ok(())
    }

    pub fn set_nonblocking(&self, b: bool) -> Result<()> {
        match *self {
            VarlinkStream::TCP(ref l) => l.set_nonblocking(b).map_err(minto_cherr!())?,
            VarlinkStream::UNIX(ref l) => l.set_nonblocking(b).map_err(minto_cherr!())?,
        }
        Ok(())
    }
}

impl Drop for VarlinkStream {
    fn drop(&mut self) {
        let _r = self.shutdown();
    }
}
