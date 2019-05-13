use std::io::{self, copy, BufRead, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::{Arc, RwLock};
use std::thread;

use chainerror::*;
use serde_json::{from_slice, from_value, to_string};

use varlink::{
    Call, Connection, ErrorKind, GetInterfaceDescriptionArgs, Reply, Request, VarlinkStream,
};
use varlink_stdinterfaces::org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};

#[cfg(unix)]
use crate::watchclose_epoll::WatchClose;
#[cfg(windows)]
use crate::watchclose_windows::WatchClose;
use crate::Result;

pub fn handle<R, W>(resolver: &str, client_reader_o: R, mut client_writer: W) -> Result<bool>
where
    R: Read + AsRawFd + Send + 'static,
    W: Write + AsRawFd + Send + 'static,
{
    let conn = Connection::new(resolver)
        .map_err(mstrerr!("Failed to connect to resolver '{}'", resolver))?;

    let mut client_reader = unsafe {
        ::std::io::BufReader::new(::std::fs::File::from_raw_fd(client_reader_o.as_raw_fd()))
    };

    let mut resolver = VarlinkClient::new(conn);

    let mut upgraded = false;
    let mut last_iface = String::new();
    let mut last_service_stream: Option<VarlinkStream> = None;
    let mut address = String::new();

    loop {
        if !upgraded {
            let mut buf = Vec::new();
            match client_reader.read_until(b'\0', &mut buf) {
                Ok(0) => break,
                Err(_e) => break,
                _ => {}
            }

            // pop the last zero byte
            buf.pop();

            let mut req: Request = from_slice(&buf).map_err(mstrerr!("Error from slice"))?;

            if req.method == "org.varlink.service.GetInfo" {
                req.method = "org.varlink.resolver.GetInfo".into();
            }

            let n: usize = match req.method.rfind('.') {
                None => {
                    let method: String = String::from(req.method.as_ref());
                    let mut call = Call::new(&mut client_writer, &req);
                    call.reply_interface_not_found(Some(method))?;
                    return Ok(false);
                }
                Some(x) => x,
            };

            let iface = {
                if req.method == "org.varlink.service.GetInterfaceDescription" {
                    let val = req.parameters.clone().unwrap_or_default();
                    let args: GetInterfaceDescriptionArgs = from_value(val)?;
                    args.interface.into()
                } else {
                    String::from(&req.method[..n])
                }
            };

            if iface != last_iface {
                if iface.eq("org.varlink.resolver") {
                    address = String::from("unix:/run/org.varlink.resolver");
                } else {
                    address = match resolver.resolve(iface.clone()).call() {
                        Ok(r) => r.address,
                        _ => {
                            let mut call = Call::new(&mut client_writer, &req);
                            call.reply_interface_not_found(Some(iface))?;
                            return Ok(false);
                        }
                    };
                }
                last_iface = iface.clone();
            }

            let mut stream = match VarlinkStream::connect(&address) {
                Ok((a, _)) => a,
                _ => {
                    let mut call = Call::new(&mut client_writer, &req);
                    call.reply_interface_not_found(Some(iface))?;
                    return Ok(false);
                }
            };

            let (_, mut service_writer) = stream.split()?;
            let service_reader = WatchClose::new_read(&stream, &client_writer)?;
            let mut service_bufreader = ::std::io::BufReader::new(service_reader);

            last_service_stream = Some(stream);

            {
                let b = to_string(&req)? + "\0";

                service_writer.write_all(b.as_bytes())?;
                service_writer.flush()?;
            }

            if req.oneway.unwrap_or(false) {
                continue;
            }

            upgraded = req.upgrade.unwrap_or(false);

            loop {
                let mut buf = Vec::new();

                if service_bufreader.read_until(0, &mut buf)? == 0 {
                    break;
                }
                if buf.is_empty() {
                    return Err(strerr!("Connection Closed").into());
                }

                client_writer.write_all(&buf)?;
                client_writer.flush()?;

                buf.pop();

                let reply: Reply = from_slice(&buf)?;

                if upgraded || (!reply.continues.unwrap_or(false)) {
                    break;
                }
            }
        } else if let Some(ref mut service_stream) = last_service_stream {
            // Should copy back and forth, until someone disconnects.
            let (_, mut service_writer) = service_stream.split()?;
            let mut service_reader = WatchClose::new_read(service_stream, &client_writer)?;
            let mut client_reader = WatchClose::new_read(&client_reader_o, service_stream)?;

            {
                let copy1 = thread::spawn(move || copy(&mut client_reader, &mut service_writer));
                let copy2 = thread::spawn(move || copy(&mut service_reader, &mut client_writer));
                let r = copy2.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
                let r = copy1.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
            }
            return Ok(true);
        } else {
            unreachable!();
        }
    }
    Ok(upgraded)
}

pub fn handle_connect<R, W>(
    connection: Arc<RwLock<Connection>>,
    client_reader: R,
    mut client_writer: W,
) -> Result<bool>
where
    R: Read + AsRawFd + Send + 'static,
    W: Write + AsRawFd + Send + 'static,
{
    let mut upgraded = false;

    let mut conn = connection.write().unwrap();

    if conn.stream.is_none() {
        return Err(varlink::context!(ErrorKind::ConnectionBusy).into());
    }

    let stream = conn.stream.take().unwrap();
    let mut service_writer = unsafe { ::std::fs::File::from_raw_fd(stream.as_raw_fd()) };

    let mut service_reader =
        ::std::io::BufReader::new(WatchClose::new_read(&stream, &client_writer)?);

    let mut client_reader =
        ::std::io::BufReader::new(WatchClose::new_read(&client_reader, &stream)?);

    loop {
        if !upgraded {
            let mut buf = Vec::new();
            // FIXME: check if service_writer was closed, while reading
            match client_reader.read_until(b'\0', &mut buf) {
                Ok(0) => break,
                Err(_e) => break,
                _ => {}
            }

            // pop the last zero byte
            buf.pop();

            let req: Request = from_slice(&buf)?;

            {
                buf.push(0);
                // FIXME: check if client_reader was closed, while writing
                service_writer.write_all(&buf)?;
                service_writer.flush()?;
            }

            if req.oneway.unwrap_or(false) {
                continue;
            }

            upgraded = req.upgrade.unwrap_or(false);

            loop {
                let mut buf = Vec::new();

                if service_reader.read_until(0, &mut buf)? == 0 {
                    break;
                }
                if buf.is_empty() {
                    return Err(strerr!("Connection Closed!").into());
                }

                client_writer.write_all(&buf)?;
                client_writer.flush()?;

                buf.pop();

                let reply: Reply = from_slice(&buf)?;

                if upgraded || !reply.continues.unwrap_or(false) {
                    break;
                }
            }
        } else {
            // Should copy back and forth, until someone disconnects.
            {
                let copy1 = thread::spawn(move || copy(&mut client_reader, &mut service_writer));
                let copy2 = thread::spawn(move || copy(&mut service_reader, &mut client_writer));
                let r = copy2.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
                let r = copy1.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
            }
            return Ok(true);
        }
    }
    Ok(upgraded)
}
