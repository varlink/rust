use std::io::{self, BufRead, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::{Arc, RwLock};
use std::thread;

use chainerror::prelude::v1::*;
use serde_json::{from_slice, from_value, to_string};

use varlink::{
    varlink_connect, Call, Connection, ErrorKind, GetInterfaceDescriptionArgs, Reply, Request,
    VarlinkStream,
};
use varlink_stdinterfaces::org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};

use crate::watchclose_epoll::WatchClose;
use crate::Result;

pub fn handle<R, W>(resolver: &str, client_reader: R, mut client_writer: W) -> Result<bool>
where
    R: Read + AsRawFd + Send + 'static,
    W: Write + AsRawFd + Send + 'static,
{
    let conn = Connection::new(resolver)
        .context(format!("Failed to connect to resolver '{}'", resolver))?;

    let mut client_bufreader = unsafe {
        ::std::io::BufReader::new(::std::fs::File::from_raw_fd(client_reader.as_raw_fd()))
    };

    let mut resolver = VarlinkClient::new(conn);

    let mut upgraded = false;
    let mut last_iface = String::new();
    let mut last_service_stream: Option<VarlinkStream> = None;
    let mut address = String::new();

    loop {
        if !upgraded {
            let mut buf = Vec::new();
            match client_bufreader.read_until(b'\0', &mut buf) {
                Ok(0) => break,
                Err(_e) => break,
                _ => {}
            }

            // pop the last zero byte
            buf.pop();

            let mut req: Request = from_slice(&buf).context("Error from slice".to_string())?;

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

            let mut stream = match varlink_connect(&address) {
                Ok((a, _)) => a,
                _ => {
                    let mut call = Call::new(&mut client_writer, &req);
                    call.reply_interface_not_found(Some(iface))?;
                    return Ok(false);
                }
            };

            let service_writer = stream.try_clone()?;
            let mut service_writer = service_writer;
            let service_reader = WatchClose::new_read(stream.as_ref(), &client_writer)?;
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
                    return Err("Connection Closed".to_string().into());
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
            // flush buffer
            client_writer.write_all(client_bufreader.buffer())?;
            let service_writer = service_stream.try_clone()?;
            let service_reader = WatchClose::new_read(service_stream.as_ref(), &client_writer)?;
            let client_reader = WatchClose::new_read(&client_reader, service_stream.as_ref())?;

            use std::sync::mpsc::channel;
            let (tx_end, rx_end) = channel();

            // Should copy back and forth, until someone disconnects.
            {
                let copy1 = thread::spawn({
                    let tx_end = tx_end.clone();
                    let mut client_reader = client_reader;
                    let mut service_writer = service_writer;

                    move || {
                        let r = copy(&mut client_reader, &mut service_writer);
                        tx_end.send(1).expect("channel should be open");
                        r
                    }
                });

                let copy2 = thread::spawn({
                    let tx_end = tx_end;
                    let mut service_reader = service_reader;

                    move || {
                        let r = copy(&mut service_reader, &mut client_writer);
                        tx_end.send(2).expect("channel should be open");
                        r
                    }
                });

                let end_tid = rx_end.recv()?;

                match end_tid {
                    1 => {
                        let r1 = copy1
                            .join()
                            .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

                        let _ = service_stream.shutdown();
                        unsafe { libc::close(service_stream.as_raw_fd()) };
                        r1?;

                        let _ = rx_end.recv()?;
                        let r2 = copy2
                            .join()
                            .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

                        r2?;
                    }

                    2 => {
                        let r2 = copy2
                            .join()
                            .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

                        let _ = service_stream.shutdown();
                        unsafe { libc::close(service_stream.as_raw_fd()) };
                        r2?;

                        let _ = rx_end.recv()?;
                        let r1 = copy1
                            .join()
                            .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

                        r1?;
                    }
                    _ => panic!("Unknown"),
                };
            }
            return Ok(true);
        } else {
            unreachable!();
        }
    }
    Ok(upgraded)
}

pub fn copy<R: ?Sized, W: ?Sized>(reader: &mut R, writer: &mut W) -> io::Result<u64>
where
    R: Read,
    W: Write,
{
    use std::io::ErrorKind;

    let mut buf = vec![0u8; 8192];

    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        writer.flush()?;
        written += len as u64;
    }
}

pub fn handle_connect<R, W>(
    connection: Arc<RwLock<Connection>>,
    client_reader: R,
    mut client_writer: W,
) -> Result<()>
where
    R: Read + AsRawFd + Send + 'static,
    W: Write + AsRawFd + Send + 'static,
{
    let mut conn = connection.write().unwrap();

    if conn.stream.is_none() {
        return Err(varlink::context!(ErrorKind::ConnectionBusy).into());
    }

    let mut stream = conn.stream.take().unwrap();

    {
        let _ = conn.reader.take();
        let _ = conn.writer.take();
    }

    let service_writer = stream.try_clone()?;

    let mut service_reader = WatchClose::new_read(stream.as_ref(), &client_writer)?;
    let mut client_reader = WatchClose::new_read(&client_reader, service_writer.as_ref())?;

    use std::sync::mpsc::channel;
    let (tx_end, rx_end) = channel();

    let copy1 = thread::spawn({
        let tx_end = tx_end.clone();
        let mut service_writer = service_writer;
        move || {
            let r = copy(&mut client_reader, &mut service_writer);
            tx_end.send(1).expect("channel should be open");

            r
        }
    });

    let copy2 = thread::spawn({
        let tx_end = tx_end.clone();

        move || {
            let r = copy(&mut service_reader, &mut client_writer);
            tx_end.send(2).expect("channel should be open");
            r
        }
    });

    let mut child = conn.child.take().unwrap();

    let child_watch = thread::spawn({
        let tx_end = tx_end;

        move || {
            let r = child.wait();
            tx_end.send(3).expect("channel should be open");
            r
        }
    });

    let end_tid = rx_end.recv()?;

    match end_tid {
        1 => {
            let r1 = copy1
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            let _ = stream.shutdown();

            let _ = rx_end.recv()?;
            let r2 = copy2
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            r1?;
            r2?;
            Ok(())
        }

        2 => {
            let r2 = copy2
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            let _ = stream.shutdown();

            let _ = rx_end.recv()?;
            let r1 = copy1
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            r2?;
            r1?;
            Ok(())
        }
        3 => {
            let cr = child_watch
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            let _ = stream.shutdown();

            let _ = rx_end.recv()?;
            let _ = copy1
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            let _ = rx_end.recv()?;
            let _ = copy2
                .join()
                .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::BrokenPipe)));

            cr?;
            Ok(())
        }
        _ => panic!("Unknown"),
    }
}
