//! Handle network connections for a varlink service

#![allow(dead_code)]

use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Child;

#[cfg(unix)]
use libc::{close, dup2, getpid};
use tempfile::TempDir;
#[cfg(windows)]
use uds_windows::UnixStream;

use crate::error::*;
use crate::stream::Stream;

pub fn varlink_connect<S: ?Sized + AsRef<str>>(address: &S) -> Result<(Box<dyn Stream>, String)> {
    let address = address.as_ref();
    let new_address: String = address.into();

    if new_address.starts_with("tcp:") {
        Ok((
            Box::new(TcpStream::connect(&new_address[4..]).map_err(map_context!())?),
            new_address,
        ))
    } else if new_address.starts_with("unix:") {
        let mut addr = String::from(new_address[5..].split(';').next().unwrap());
        if addr.starts_with('@') {
            addr = addr.replacen('@', "\0", 1);
            return get_abstract_unixstream(&addr)
                .and_then(|v| Ok((Box::new(v) as Box<dyn Stream>, new_address)));
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
    use std::os::unix::io::FromRawFd;

    unsafe {
        Ok(UnixStream::from_raw_fd(
            UnixStream::connect(addr)
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
    return Err(context!(ErrorKind::MethodNotImplemented(
        "varlink_exec".into()
    )));
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
    let dir = tempdir().map_err(map_context!())?;
    let file_path = dir.path().join("varlink-socket");

    let listener = UnixListener::bind(file_path.clone()).map_err(map_context!())?;
    let fd = listener.into_raw_fd();

    let child = unsafe {
        Command::new("sh")
            .arg("-c")
            .arg(executable)
            .pre_exec({
                let file_path = file_path.clone();
                move || {
                    dup2(2, 1);
                    if fd != 3 {
                        close(3);
                        dup2(fd, 3);
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
