extern crate getopts;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_more::*;
use std::{thread, time};
use std::env;
use std::io;
use std::io::{Error, ErrorKind};
use std::process::exit;
use varlink::VarlinkService;

// Dynamically build the varlink rust code.
mod org_example_more;

// Main

fn print_usage(program: &str, opts: getopts::Options) {
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
            print_usage(&program, opts);
            return;
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let client_mode = matches.opt_present("client");

    let address = match matches.opt_str("varlink") {
        None => {
            if !client_mode {
                eprintln!("Need varlink address in server mode.");
                print_usage(&program, opts);
                return;
            }
            format!("exec:{}", program)
        }
        Some(a) => a,
    };

    let ret = match client_mode {
        true => run_client(address),
        false => run_server(address, 0, 1000),
    };

    exit(match ret {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

// Client

fn run_client(address: String) -> io::Result<()> {
    let con1 = varlink::Connection::new(&address)?;
    let new_addr;
    {
        let conn = con1.read().unwrap();
        new_addr = conn.address();
    }
    let call = org_example_more::VarlinkClient::new(con1);

    let con2 = varlink::Connection::new(&new_addr)?;
    let mut pingcall = org_example_more::VarlinkClient::new(con2);

    for reply in call.more().test_more(10)? {
        let reply = reply?;
        //assert!(reply.state.is_some());
        let state = reply.state;
        match state {
            State {
                start: Some(true),
                end: None,
                progress: None,
                ..
            } => {
                eprintln!("--- Start ---");
            }
            State {
                start: None,
                end: Some(true),
                progress: None,
                ..
            } => {
                eprintln!("--- End ---");
            }
            State {
                start: None,
                end: None,
                progress: Some(progress),
                ..
            } => {
                eprintln!("Progress: {}", progress);
                if progress > 50 {
                    let reply = pingcall.ping("Test".into())?.recv()?;
                    println!("Pong: '{}'", reply.pong);
                }
            }
            _ => eprintln!("Got unknown state: {:?}", state),
        }
    }

    Ok(())
}

// Server

struct MyOrgExampleMore {
    sleep_duration: u64,
}

impl VarlinkInterface for MyOrgExampleMore {
    fn ping(&self, call: &mut _CallPing, ping: String) -> io::Result<()> {
        return call.reply(ping);
    }

    fn stop_serving(&self, call: &mut _CallStopServing) -> io::Result<()> {
        call.reply()?;
        Err(Error::new(ErrorKind::ConnectionRefused, "Disconnect"))
    }
    fn test_more(&self, call: &mut _CallTestMore, n: i64) -> io::Result<()> {
        if !call.wants_more() {
            return call.reply_test_more_error("called without more".into());
        }

        if n == 0 {
            return call.reply_test_more_error("n == 0".into());
        }

        call.set_continues(true);

        call.reply(State {
            start: Some(true),
            end: None,
            progress: None,
        })?;

        for i in 0..n {
            thread::sleep(time::Duration::from_millis(self.sleep_duration));
            call.reply(State {
                progress: Some(i * 100 / n),
                start: None,
                end: None,
            })?;
        }

        call.reply(State {
            progress: Some(100),
            start: None,
            end: None,
        })?;

        call.set_continues(false);

        call.reply(State {
            end: Some(true),
            progress: None,
            start: None,
        })
    }
}

fn run_server(address: String, timeout: u64, sleep_duration: u64) -> io::Result<()> {
    let myexamplemore = MyOrgExampleMore { sleep_duration };
    let myinterface = new(Box::new(myexamplemore));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );
    varlink::listen(service, &address, 10, timeout)
}

#[cfg(test)]
mod test {
    use std::io;
    use std::{thread, time};

    fn run_self_test(address: String) -> io::Result<()> {
        let client_address = address.clone();

        let child = thread::spawn(move || {
            if let Err(e) = ::run_server(address, 4, 100) {
                panic!("error: {}", e);
            }
        });

        // give server time to start
        thread::sleep(time::Duration::from_secs(1));

        let ret = ::run_client(client_address);
        if let Err(e) = ret {
            panic!("error: {}", e);
        }
        if let Err(e) = child.join() {
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("{:#?}", e),
            ))
        } else {
            Ok(())
        }
    }

    #[test]
    fn test_unix() {
        assert!(run_self_test("unix:/tmp/org.example.more".into()).is_ok());
    }
}
