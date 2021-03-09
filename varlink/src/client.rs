//! Handle network connections for a varlink service

#![allow(dead_code)]

use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, IntoRawFd};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::process::Child;

#[cfg(unix)]
use libc::{close, dup2, getpid};
use tempfile::TempDir;
#[cfg(windows)]
use uds_windows::UnixStream;

use crate::error::*;
use crate::stream::Stream;

#[allow(clippy::try_err)]
pub fn varlink_connect<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Box<dyn Stream>, String)> {
    let address = address.as_ref();
    let new_address: String = address.into();

    if let Some(addr) = new_address.strip_prefix("tcp:") {
        Ok((
            Box::new(TcpStream::connect(&addr).map_err(map_context!())?),
            new_address,
        ))
    } else if let Some(addr) = new_address.strip_prefix("unix:") {
        let mut addr = String::from(addr.split(';').next().unwrap());
        if addr.starts_with('@') {
            addr = addr.replacen('@', "\0", 1);
            return get_abstract_unixstream(&addr)
                .map(|v| (Box::new(v) as Box<dyn Stream>, new_address));
        }
        Ok((
            Box::new(UnixStream::connect(addr).map_err(map_context!())?),
            new_address,
        ))
    } else {
        Err(context!(ErrorKind::InvalidAddress))?
    }
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
                .map_err(map_context!())?
                .into_raw_fd(),
        ))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn get_abstract_unixstream(_addr: &str) -> Result<UnixStream> {
    Err(context!(ErrorKind::InvalidAddress))
}

#[cfg(windows)]
pub fn varlink_exec<S: ?Sized + AsRef<str>>(
    _address: &S,
) -> Result<(Child, String, Option<TempDir>)> {
    Err(context!(ErrorKind::MethodNotImplemented(
        "varlink_exec".into()
    )))
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

    let dir = tempdir().map_err(map_context!())?;
    let file_path = dir.path().join("varlink-socket");

    let listener = UnixListener::bind(file_path.clone()).map_err(map_context!())?;
    let fd = listener.as_raw_fd();

    let child = unsafe {
        Command::new("sh")
            .arg("-c")
            .arg(executable)
            .pre_exec({
                let file_path = file_path.clone();
                move || {
                    dup2(2, 1);
                    if fd != 3 {
                        dup2(fd, 3);
                        close(fd);
                    }
                    env::set_var("VARLINK_ADDRESS", format!("unix:{}", file_path.display()));
                    env::set_var("LISTEN_FDS", "1");
                    env::set_var("LISTEN_FDNAMES", "varlink");
                    env::set_var("LISTEN_PID", format!("{}", getpid()));
                    Ok(())
                }
            })
            .spawn()
            .map_err(map_context!())?
    };

    Ok((child, format!("unix:{}", file_path.display()), Some(dir)))
}

#[cfg(windows)]
pub fn varlink_bridge<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Child, Box<dyn Stream>)> {
    use std::io::copy;
    use std::process::{Command, Stdio};
    use std::thread;

    let (stream0, stream1) = UnixStream::pair().map_err(map_context!())?;
    let executable = address.as_ref();

    let mut child = Command::new("cmd")
        .arg("/C")
        .arg(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(map_context!())?;

    let mut client_writer = child.stdin.take().unwrap();
    let mut client_reader = child.stdout.take().unwrap();
    let mut service_writer = stream1.try_clone().map_err(map_context!())?;
    let mut service_reader = stream1;

    thread::spawn(move || copy(&mut client_reader, &mut service_writer));
    thread::spawn(move || copy(&mut service_reader, &mut client_writer));

    Ok((child, Box::new(stream0)))
}

#[cfg(unix)]
pub fn varlink_bridge<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Child, Box<dyn Stream>)> {
    use std::os::unix::io::FromRawFd;
    use std::process::Command;

    let executable = address.as_ref();
    // use unix_socket::UnixStream;
    let (stream0, stream1) = UnixStream::pair().map_err(map_context!())?;
    let fd = stream1.into_raw_fd();
    let childin = unsafe { ::std::fs::File::from_raw_fd(fd) };
    let childout = unsafe { ::std::fs::File::from_raw_fd(fd) };

    let child = Command::new("sh")
        .arg("-c")
        .arg(executable)
        .stdin(childin)
        .stdout(childout)
        .spawn()
        .map_err(map_context!())?;
    Ok((child, Box::new(stream0)))
}
