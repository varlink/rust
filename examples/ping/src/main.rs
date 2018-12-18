use std::env;
use std::io::{BufRead, Read, Write};
use std::process::exit;
use std::sync::{Arc, RwLock};

use varlink::{Call, Connection, VarlinkService};

use crate::org_example_ping::*;
use chainerror::*;

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
    opts.optflag("m", "multiplex", "run in multiplex mode");
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

    let ret: std::result::Result<(), Box<std::error::Error>> = if client_mode {
        let connection = match matches.opt_str("varlink") {
            None => Connection::with_activate(&format!("{} --varlink=$VARLINK_ADDRESS", program))
                .unwrap(),
            Some(address) => Connection::with_address(&address).unwrap(),
        };
        run_client(&connection).map_err(|e| e.into())
    } else if let Some(address) = matches.opt_str("varlink") {
        run_server(&address, 0, matches.opt_present("m")).map_err(mstrerr!("running server with \
        address {}", address)).map_err(|e| e.into())
    } else {
        print_usage(&program, &opts);
        eprintln!("Need varlink address in server mode.");
        exit(1);
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

fn run_client(connection: &Arc<RwLock<varlink::Connection>>) -> Result<()> {
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

        let _reply = iface.upgrade().upgrade()?;
        println!("Client: upgrade()");
    }
    {
        // serve upgraded connection
        let mut conn = connection.write().unwrap();
        let mut writer = conn.writer.take().unwrap();
        writer.write_all(b"test test\nEnd\n").map_err(minto_cherr!())?;
        conn.writer = Some(writer);
        let mut buf = Vec::new();
        let mut reader = conn.reader.take().unwrap();
        if reader.read_until(b'\n', &mut buf).map_err(minto_cherr!())? == 0 {
            // incomplete data, in real life, store all bytes for the next call
            // for now just read the rest
            reader.read_to_end(&mut buf).map_err(minto_cherr!())?;
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
        call.to_upgraded();
        call.reply()
    }

    // An upgraded connection has its own application specific protocol.
    // Normally, there is no way back to the varlink protocol with this connection.
    fn call_upgraded(&self, call: &mut Call, bufreader: &mut BufRead) -> varlink::Result<Vec<u8>> {
        loop {
            let mut buf = String::new();
            let len = bufreader
                .read_line(&mut buf)
                .map_err(minto_cherr!())?;
            if len == 0 {
                eprintln!("Server: upgraded got: none");
                // incomplete data, in real life, store all bytes for the next call
                // return Ok(buf.as_bytes().to_vec());
                return Err(
                    into_cherr!(varlink::ErrorKind::ConnectionClosed)
                );
            }
            eprintln!("Server: upgraded got: {}", buf);

            call.writer
                .write_all(b"server reply: ")
                .map_err(minto_cherr!())?;
            call.writer
                .write_all(buf.as_bytes())
                .map_err(minto_cherr!())?;
            if buf.eq("End\n") {
                break;
            }
        }
        eprintln!("Server: upgraded ending");
        Ok(Vec::new())
    }
}

#[cfg(unix)]
mod multiplex {
    use std::collections::HashMap;
    use std::io::{self, BufRead, BufReader, Error, Read, Write};
    use std::sync::{Arc, RwLock};
    use std::thread;

        use chainerror::*;
    use varlink::{ConnectionHandler, Listener, ServerStream};

    struct FdTracker {
        stream: Option<ServerStream>,
        buffer: Option<Vec<u8>>,
    }

    impl FdTracker {
        fn shutdown(&mut self) -> varlink::Result<()> {
            self.stream.as_mut().unwrap().shutdown()
        }
        fn chain_buffer(&mut self, buf: &mut Vec<u8>) {
            self.buffer.as_mut().unwrap().append(buf);
        }
        #[allow(clippy::ptr_arg)]
        fn fill_buffer(&mut self, buf: &Vec<u8>) {
            self.buffer.as_mut().unwrap().clone_from(buf);
        }
        fn buf_as_slice(&mut self) -> &[u8] {
            self.buffer.as_mut().unwrap().as_slice()
        }
        fn write(&mut self, out: &[u8]) -> io::Result<usize> {
            self.stream.as_mut().unwrap().write(out)
        }
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.stream.as_mut().unwrap().read(buf)
        }
    }

    // listen_multiplex is just an example, if you don't want to use varlink::listen()
    // and how to build your own main-loop and use the low-level varlink::handle() instead
    pub fn listen_multiplex<
        S: ?Sized + AsRef<str>,
        H: ConnectionHandler + Send + Sync + 'static,
    >(
        handler: H,
        address: &S,
        idle_timeout: u64,
    ) -> varlink::Result<()> {
        let timeout: i32 = match idle_timeout {
            0 => -1,
            n => (n * 1000) as i32,
        };

        let handler = Arc::new(handler);
        let mut fdmap: HashMap<i32, FdTracker> = HashMap::new();
        let mut fds = Vec::new();
        let mut threads = Vec::new();
        let listener = Listener::new(address)?;
        let upgraded_in_use = Arc::new(RwLock::new(0));

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

                fdmap.insert(
                    fd,
                    FdTracker {
                        stream: Some(client),
                        buffer: Some(Vec::new()),
                    },
                );
            }

            // Store which indices to remove
            let mut indices_to_remove = vec![];

            // Check client connections ...
            for (i, fds_item) in fds.iter().enumerate().skip(1) {
                if fds_item.revents != 0 {
                    let mut upgraded_iface: Option<String> = None;
                    let tracker = fdmap.get_mut(&fds_item.fd).unwrap();
                    loop {
                        let mut readbuf: [u8; 8192] = [0; 8192];

                        match tracker.read(&mut readbuf) {
                            Ok(0) => {
                                let _ = tracker.shutdown();
                                indices_to_remove.push(i);
                                break;
                            }
                            Ok(len) => {
                                let mut out: Vec<u8> = Vec::new();
                                tracker.chain_buffer(&mut readbuf[0..len].to_vec());
                                eprintln!(
                                    "Handling: {}",
                                    String::from_utf8_lossy(&tracker.buf_as_slice())
                                );

                                match handler.handle(&mut tracker.buf_as_slice(), &mut out, None) {
                                    // TODO: buffer output and write only on POLLOUT
                                    Ok((unprocessed_bytes, last_iface)) => {
                                        upgraded_iface = last_iface;
                                        if !unprocessed_bytes.is_empty() {
                                            eprintln!(
                                                "Unprocessed bytes: {}",
                                                String::from_utf8_lossy(&unprocessed_bytes)
                                            );
                                        }
                                        tracker.fill_buffer(&unprocessed_bytes);

                                        if let Err(err) = tracker.write(out.as_ref()) {
                                            eprintln!("write error: {}", err);
                                            let _ = tracker.shutdown();
                                            indices_to_remove.push(i);
                                            break;
                                        }
                                    }
                                    Err(e) => match e.kind() {
                                        err => {
                                            eprintln!("handler error: {}", err);
                                            let _ = tracker.shutdown();
                                            indices_to_remove.push(i);
                                            break;
                                        }
                                    },
                                }
                            }
                            Err(e) => match e.kind() {
                                io::ErrorKind::WouldBlock => {
                                    break;
                                }
                                _ => {
                                    let _ = tracker.shutdown();
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
                        // If you have a better idea, open an Issue or PR on github
                        indices_to_remove.push(i);

                        let j = thread::spawn({
                            eprintln!("upgraded thread");
                            let handler = handler.clone();
                            let mut stream = tracker.stream.take().unwrap();
                            let buffer = tracker.buffer.take().unwrap();
                            let upgraded_in_use = upgraded_in_use.clone();
                            move || {
                                let _r = stream.set_nonblocking(false);
                                let (reader, mut writer) = stream.split().unwrap();
                                let br = BufReader::new(reader);
                                let mut bufreader = Box::new(buffer.chain(br));
                                let mut upgraded_iface = upgraded_iface.take();

                                {
                                    let mut ctr = upgraded_in_use.write().unwrap();
                                    *ctr += 1;
                                }
                                loop {
                                    match handler.handle(
                                        &mut bufreader,
                                        &mut writer,
                                        upgraded_iface,
                                    ) {
                                        Ok((unread, iface)) => {
                                            upgraded_iface = iface;
                                            match bufreader.fill_buf() {
                                                Err(_) => {
                                                    eprintln!("Upgraded end");
                                                    break;
                                                }
                                                Ok(buf) => {
                                                    if buf.is_empty() && unread.is_empty() {
                                                        eprintln!("Upgraded end");
                                                        break;
                                                    }

                                                    if !unread.is_empty() {
                                                        eprintln!(
                                                            "Not handled bytes: {}",
                                                            String::from_utf8_lossy(&unread)
                                                        );
                                                        break;
                                                    }

                                                    if !buf.is_empty() {
                                                        eprintln!(
                                                            "fill_buf(): {}",
                                                            String::from_utf8_lossy(&buf)
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Err(err) => match err.kind() {
                                            varlink::ErrorKind::ConnectionClosed => {
                                                eprintln!("Upgraded end");
                                                break;
                                            }
                                            _ => {
                                                eprintln!("Upgraded end: {}", err);
                                                break;
                                            }
                                        },
                                    }
                                }
                                {
                                    let mut ctr = upgraded_in_use.write().unwrap();
                                    *ctr -= 1;
                                }
                            }
                        });
                        threads.push(j);
                    }
                }
            }

            // We can't modify the vector while we are traversing it, so update now.
            for i in indices_to_remove {
                fdmap.remove(&fds[i].fd);
                fds.remove(i);
            }

            let r = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };

            if r < 0 {
                for t in threads {
                    let _r = t.join();
                }
                return Err(Error::last_os_error()).map_err(minto_cherr!());
            }

            if r == 0 && fds.len() == 1 && *upgraded_in_use.read().unwrap() == 0 {
                eprintln!("listen_multiplex: Waiting for threads to end.");
                for t in threads {
                    let _r = t.join();
                }

                return Err(into_cherr!(varlink::ErrorKind::Timeout));
            }
        }
    }
}

fn run_server(address: &str, timeout: u64, multiplex: bool) -> varlink::Result<()> {
    let myorgexampleping = MyOrgExamplePing;
    let myinterface = org_example_ping::new(Box::new(myorgexampleping));
    let service = VarlinkService::new(
        "org.varlink",
        "test ping service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    #[cfg(windows)]
    {
        let _ = multiplex;
        varlink::listen(service, &address, 1, 10, timeout)?;
    }
    #[cfg(unix)]
    {
        if multiplex {
            // Demonstrate a single process, single-threaded service
            multiplex::listen_multiplex(service, &address, timeout)?;
        } else {
            varlink::listen(service, &address, 1, 10, timeout)?;
        }
    }
    Ok(())
}
