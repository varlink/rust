use std::io::{self, copy, BufRead, Write};
use std::thread;

use chainerror::*;
use serde_json::{from_slice, from_value, to_string};

use varlink::{Call, Connection, GetInterfaceDescriptionArgs, Reply, Request, VarlinkStream};
use varlink_stdinterfaces::org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};

use crate::Result;

pub fn handle<R, W>(mut client_reader: R, mut client_writer: W) -> Result<bool>
where
    R: BufRead + Send + Sync + 'static,
    W: Write + Send + Sync + 'static,
{
    let conn = Connection::new("unix:/run/org.varlink.resolver").map_err(mstrerr!(
        "Failed to connect to resolver '{}'",
        "unix:/run/org.varlink.resolver"
    ))?;
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

            let (service_reader, mut service_writer) = stream.split()?;
            last_service_stream = Some(stream);
            let mut service_bufreader = ::std::io::BufReader::new(service_reader);

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
            let (mut service_reader, mut service_writer) = service_stream.split()?;
            {
                let copy1 = thread::spawn(move || copy(&mut client_reader, &mut service_writer));
                let copy2 = thread::spawn(move || copy(&mut service_reader, &mut client_writer));
                let r = copy1.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
                let r = copy2.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
            }
            return Ok(true);
        }
    }
    Ok(upgraded)
}

pub fn handle_connect<R, W>(
    address: &str,
    mut client_reader: R,
    mut client_writer: W,
) -> Result<bool>
where
    R: BufRead + Send + Sync + 'static,
    W: Write + Send + Sync + 'static,
{
    let mut upgraded = false;
    let mut last_iface = String::new();
    let (mut stream, _) = VarlinkStream::connect(&address)?;

    loop {
        if !upgraded {
            let (service_reader, mut service_writer) = stream.split()?;

            let mut buf = Vec::new();
            match client_reader.read_until(b'\0', &mut buf) {
                Ok(0) => break,
                Err(_e) => break,
                _ => {}
            }

            // pop the last zero byte
            buf.pop();

            let req: Request = from_slice(&buf)?;

            let n: usize = match req.method.rfind('.') {
                None => {
                    let method: String = String::from(req.method.as_ref());
                    let mut call = Call::new(&mut client_writer, &req);
                    call.reply_interface_not_found(Some(method))?;
                    return Ok(false);
                }
                Some(x) => x,
            };

            let iface = String::from(&req.method[..n]);

            if iface != last_iface {
                last_iface = iface.clone();
            }

            {
                let b = to_string(&req)? + "\0";

                service_writer.write_all(b.as_bytes())?;
                service_writer.flush()?;
            }

            if req.oneway.unwrap_or(false) {
                continue;
            }

            upgraded = req.upgrade.unwrap_or(false);

            let mut service_bufreader = ::std::io::BufReader::new(service_reader);

            loop {
                let mut buf = Vec::new();

                if service_bufreader.read_until(0, &mut buf)? == 0 {
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
            let (mut service_reader, mut service_writer) = stream.split()?;

            // Should copy back and forth, until someone disconnects.
            {
                let copy1 = thread::spawn(move || copy(&mut client_reader, &mut service_writer));
                let copy2 = thread::spawn(move || copy(&mut service_reader, &mut client_writer));
                let r = copy1.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
                let r = copy2.join();
                r.unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::ConnectionAborted)))?;
            }
            return Ok(true);
        }
    }
    Ok(upgraded)
}
