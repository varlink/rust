extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate getopts;
extern crate libc;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use failure::Fail;
use org_example_ping::*;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Error, Read, Write};
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::thread;
use varlink::{
    Call, CallTrait, Connection, ConnectionHandler, Listener, ServerStream, VarlinkService,
};

// Dynamically build the varlink rust code.
mod org_example_ping;

#[cfg(test)]
mod test;

// Main

fn print_usage(program: &str, opts: &getopts::Options) {
    let brief = format!("Usage: {} [--varlink=<address>] [--client]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optopt("", "varlink", "varlink address URL", "<address>");
    opts.optflag("", "client", "run in client mode");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            print_usage(&program, &opts);
            return;
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return;
    }

    let client_mode = matches.opt_present("client");

    let ret = if client_mode {
        let connection = match matches.opt_str("varlink") {
            None => Connection::with_activate(&format!("{} --varlink=$VARLINK_ADDRESS", program))
                .unwrap(),
            Some(address) => Connection::with_address(&address).unwrap(),
        };
        run_client(connection)
    } else {
        if let Some(address) = matches.opt_str("varlink") {
            run_server(&address, 0).map_err(|e| e.into())
        } else {
            print_usage(&program, &opts);
            eprintln!("Need varlink address in server mode.");
            exit(1);
        }
    };
    exit(match ret {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}

// Client

fn run_client(connection: Arc<RwLock<varlink::Connection>>) -> Result<()> {
    {
        let mut iface = VarlinkClient::new(connection.clone());
        let ping = String::from("Test");

        let reply = iface.ping(ping.clone()).call()?;
        assert_eq!(ping, reply.pong);
        println!("Pong: '{}'", reply.pong);

        let reply = iface.ping(ping.clone()).call()?;
        assert_eq!(ping, reply.pong);
        println!("Pong: '{}'", reply.pong);

        let reply = iface.ping(ping.clone()).call()?;
        assert_eq!(ping, reply.pong);
        println!("Pong: '{}'", reply.pong);

        let _reply = iface.upgrade().call()?;
        println!("Client: upgrade()");
    }
    {
        let mut conn = connection.write().unwrap();
        let mut writer = conn.writer.take().unwrap();
        writer.write_all("test test\n".as_bytes())?;
        conn.writer = Some(writer);
        let mut buf = Vec::new();
        let mut reader = conn.reader.take().unwrap();
        if reader.read_until('\n' as u8, &mut buf)? == 0 {
            // incomplete data, in real life, store all bytes for the next call
            // for now just read the rest
            reader.read_to_end(&mut buf)?;
        };
        eprintln!("Client: upgraded got: {}", String::from_utf8_lossy(&buf));
        conn.reader = Some(reader);
    }
    Ok(())
}

// Server

struct MyOrgExamplePing;

impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
    fn ping(&self, call: &mut Call_Ping, ping: String) -> varlink::Result<()> {
        call.reply(ping)
    }

    fn upgrade(&self, call: &mut Call_Upgrade) -> varlink::Result<()> {
        eprintln!("Server: called upgrade");
        call.set_upgraded(true);
        call.reply()
    }

    fn call_upgraded(&self, call: &mut Call, bufreader: &mut BufRead) -> varlink::Result<usize> {
        let mut buf = String::new();
        let len = bufreader.read_line(&mut buf)?;
        if len == 0 {
            // incomplete data, in real life, store all bytes for the next call
            // for now just drop out of upgraded
            call.set_upgraded(false);

            return Ok(len);
            //return Err(varlink::ErrorKind::ConnectionClosed.into());
        }
        eprintln!("Server: upgraded got: {}", buf);

        call.writer.write_all("server reply\n".as_bytes())?;

        call.set_upgraded(true);
        Ok(len)
    }
}

pub fn listen_multiplex<S: ?Sized + AsRef<str>, H: ::ConnectionHandler + Send + Sync + 'static>(
    handler: H,
    address: &S,
    accept_timeout: u64,
) -> varlink::Result<()> {
    let handler = Arc::new(handler);
    let mut fd_to_stream: HashMap<i32, ServerStream> = HashMap::new();
    let mut fd_to_buffer: HashMap<i32, Vec<u8>> = HashMap::new();
    let mut fds = Vec::new();
    let mut threads = Vec::new();
    let listener = Listener::new(address)?;
    listener.set_nonblocking(true)?;

    fds.push(libc::pollfd {
        fd: listener.as_raw_fd(),
        revents: 0,
        events: libc::POLLIN,
    });

    loop {
        // Read activity on listening socket
        if fds[0].revents != 0 {
            let mut client = listener.accept(0)?;

            client.set_nonblocking(true)?;

            let fd = client.as_raw_fd();
            fds.push(libc::pollfd {
                fd,
                revents: 0,
                events: libc::POLLIN,
            });

            fd_to_stream.insert(fd, client);
            fd_to_buffer.insert(fd, Vec::new());
        }

        // Store which indices to remove
        let mut indices_to_remove = vec![];

        // Check client connections ...
        for i in 1..fds.len() {
            if fds[i].revents != 0 {
                let mut upgraded_iface: Option<String> = None;
                let mut client = fd_to_stream.get_mut(&fds[i].fd).unwrap();
                let mut buf = fd_to_buffer.get_mut(&fds[i].fd).unwrap();
                loop {
                    let mut msg_index: Option<usize> = None;
                    let mut byte_buf: [u8; 8192] = [0; 8192];
                    match client.read(&mut byte_buf) {
                        Ok(0) => {
                            let _ = client.shutdown();
                            indices_to_remove.push(i);
                            break;
                        }
                        Ok(len) => {
                            // read() until zero byte
                            let old: usize = buf.len();
                            buf.extend(&byte_buf[0..len]);

                            for (n, b) in byte_buf[0..len].iter().enumerate() {
                                if *b == 0u8 {
                                    msg_index = Some(old + n + 1);
                                    break;
                                }
                            }

                            // if zero byte found, handle message and write output.
                            if let Some(n) = msg_index {
                                let mut out: Vec<u8> = Vec::new();
                                match handler.handle(
                                    &mut BufReader::new(&buf[0..n]),
                                    &mut out,
                                    None,
                                ) {
                                    // TODO: buffer output and write only on POLLOUT
                                    Ok(None) => match client.write(out.as_ref()) {
                                        Err(e) => {
                                            eprintln!("write error: {}", e);
                                            let _ = client.shutdown();
                                            indices_to_remove.push(i);
                                            break;
                                        }
                                        Ok(_) => {}
                                    },
                                    Ok(Some(last_iface)) => match client.write(out.as_ref()) {
                                        Err(e) => {
                                            eprintln!("write error: {}", e);
                                            let _ = client.shutdown();
                                            indices_to_remove.push(i);
                                            break;
                                        }
                                        Ok(_) => {
                                            upgraded_iface = Some(last_iface);
                                        }
                                    },
                                    Err(e) => match e.kind() {
                                        err => {
                                            eprintln!("handler error: {}", err);
                                            for cause in err.causes().skip(1) {
                                                eprintln!("  caused by: {}", cause);
                                            }
                                            let _ = client.shutdown();
                                            indices_to_remove.push(i);
                                            break;
                                        }
                                    },
                                }

                                // Remove the handled message
                                buf.drain(0..n);
                            }
                        }
                        Err(e) => match e.kind() {
                            ::std::io::ErrorKind::WouldBlock => {
                                break;
                            }
                            _ => {
                                let _ = client.shutdown();
                                indices_to_remove.push(i);
                                eprintln!("IO error: {}", e);
                                break;
                            }
                        },
                    }
                }
                if upgraded_iface.is_some() {
                    eprintln!("Upgraded MODE");
                    // upgraded mode... thread away the server
                    // feed it directly with the client stream
                    indices_to_remove.push(i);
                    let mut stream = client.try_clone().unwrap();
                    let j = thread::spawn({
                        eprintln!("upgraded thread");
                        let handler = handler.clone();
                        move || {
                            let _r = stream.set_nonblocking(false);
                            let (reader, mut writer) = stream.split().unwrap();

                            let _r = handler.handle(
                                &mut BufReader::new(reader),
                                &mut writer,
                                upgraded_iface,
                            );
                        }
                    });
                    threads.push(j);
                }
            }
        }

        // We can't modify the vector while we are traversing it, so update now.
        for i in indices_to_remove {
            fd_to_buffer.remove(&fds[i].fd);
            fd_to_stream.remove(&fds[i].fd);
            fds.remove(i);
        }

        let r = unsafe {
            libc::poll(
                fds.as_mut_ptr(),
                (fds.len() as u32).into(),
                (accept_timeout * 1000) as i32,
            )
        };

        if r < 0 {
            return Err(Error::last_os_error().into());
        }

        if r == 0 && fds.len() == 1 {
            for t in threads {
                let _r = t.join();
            }

            return Err(varlink::Error::from(varlink::ErrorKind::Timeout));
        }
    }
}

fn run_server(address: &str, timeout: u64) -> varlink::Result<()> {
    let myorgexampleping = MyOrgExamplePing;
    let myinterface = org_example_ping::new(Box::new(myorgexampleping));
    let service = VarlinkService::new(
        "org.varlink",
        "test ping service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    //varlink::listen(service, &address, 10, timeout)?;

    // Demonstrate a single process, single-threaded service
    listen_multiplex(service, &address, timeout)?;
    Ok(())
}
