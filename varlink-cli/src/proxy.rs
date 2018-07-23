use failure::ResultExt;
use org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};
use serde_json::{from_slice, to_string};
use std::io::{BufRead, Write};
use varlink::{
    Call, Connection, ConnectionHandler, ErrorKind, Reply, Request, Result, VarlinkStream,
};

pub struct Proxy {}

impl ConnectionHandler for Proxy {
    fn handle(&self, bufreader: &mut BufRead, writer: &mut Write) -> Result<bool> {
        let conn = Connection::new("unix:/run/org.varlink.resolver")?;
        let mut resolver = VarlinkClient::new(conn);

        let mut upgraded = false;
        let mut last_iface = String::new();
        let mut address = String::new();

        loop {
            if !upgraded {
                let mut buf = Vec::new();
                match bufreader.read_until(b'\0', &mut buf) {
                    Ok(0) => break,
                    Err(_e) => break,
                    _ => {}
                }

                // pop the last zero byte
                buf.pop();

                let mut req: Request = from_slice(&buf).context(ErrorKind::SerdeJsonDe(
                    String::from_utf8_lossy(&buf).to_string(),
                ))?;

                if req.method == "org.varlink.service.GetInfo" {
                    req.method = "org.varlink.resolver.GetInfo".into();
                }

                let n: usize = match req.method.rfind('.') {
                    None => {
                        let method: String = String::from(req.method.as_ref());
                        let mut call = Call::new(writer, &req);
                        call.reply_interface_not_found(Some(method))?;
                        return Ok(false);
                    }
                    Some(x) => x,
                };

                let iface = String::from(&req.method[..n]);

                if iface != last_iface {
                    if iface.eq("org.varlink.resolver") {
                        address = String::from("unix:/run/org.varlink.resolver");
                    } else {
                        address = match resolver.resolve(iface.clone()).call() {
                            Ok(r) => r.address,
                            _ => {
                                let mut call = Call::new(writer, &req);
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
                        let mut call = Call::new(writer, &req);
                        call.reply_interface_not_found(Some(iface))?;
                        return Ok(false);
                    }
                };

                let (r, mut w) = stream.split()?;
                let mut bufreader = ::std::io::BufReader::new(r);

                {
                    let b = to_string(&req)? + "\0";

                    w.write_all(b.as_bytes())?;
                    w.flush()?;
                }

                loop {
                    let mut buf = Vec::new();

                    if bufreader.read_until(0, &mut buf)? == 0 {
                        break;
                    }
                    if buf.is_empty() {
                        return Err(ErrorKind::ConnectionClosed)?;
                    }

                    writer.write_all(&buf)?;
                    writer.flush()?;

                    buf.pop();

                    let reply: Reply = from_slice(&buf).context(ErrorKind::SerdeJsonDe(
                        String::from_utf8_lossy(&buf).to_string(),
                    ))?;

                    upgraded = reply.upgraded.unwrap_or(false);

                    if upgraded || !reply.continues.unwrap_or(false) {
                        break;
                    }
                }
            } else {
                // TODO: also read() from writer and send back the output
                let mut buf = vec![0; 2048];
                let mut size = bufreader.read(&mut buf)?;
                loop {
                    size -= writer.write(&buf[..size])?;
                    if size == 0 {
                        break;
                    }
                }
            }
        }
        Ok(upgraded)
    }
}
