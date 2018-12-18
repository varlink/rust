use std::env;
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::{thread, time};

use varlink::{Connection, VarlinkService};

use crate::org_example_more::*;
use chainerror::*;

// Dynamically build the varlink rust code.
mod org_example_more;

#[cfg(test)]
mod test;

pub type Result<T> = std::result::Result<T, Box<std::error::Error>>;


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
    opts.optopt("", "timeout", "server timeout", "<seconds>");
    opts.optopt("", "sleep", "sleep duration", "<milliseconds>");

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

    let timeout = matches
        .opt_str("timeout")
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(0);

    let sleep = matches
        .opt_str("sleep")
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(1000);

    let ret : Result<()> = if client_mode {
        let connection = match matches.opt_str("varlink") {
            None => Connection::with_activate(&format!("{} --varlink=$VARLINK_ADDRESS", program))
                .unwrap(),
            Some(address) => Connection::with_address(&address).unwrap(),
        };
        run_client(connection)
    } else if let Some(address) = matches.opt_str("varlink") {
        run_server(&address, timeout, sleep).map_err(|e|e.into())
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

fn run_client(connection: Arc<RwLock<varlink::Connection>>) -> Result<()> {
    let new_addr = {
        let conn = connection.read().unwrap();
        conn.address()
    };
    let mut iface = org_example_more::VarlinkClient::new(connection);

    let con2 = varlink::Connection::with_address(&new_addr)?;
    let mut pingiface = org_example_more::VarlinkClient::new(con2);

    for reply in iface.test_more(10).more()? {
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
                    let reply = pingiface.ping("Test".into()).call()?;
                    eprintln!("Pong: '{}'", reply.pong);
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
    fn ping(&self, call: &mut Call_Ping, ping: String) -> varlink::Result<()> {
        call.reply(ping)
    }

    fn stop_serving(&self, call: &mut Call_StopServing) -> varlink::Result<()> {
        call.reply()?;
        Err(into_cherr!(varlink::ErrorKind::ConnectionClosed))
    }
    fn test_more(&self, call: &mut Call_TestMore, n: i64) -> varlink::Result<()> {
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

fn run_server(address: &str, timeout: u64, sleep_duration: u64) -> varlink::Result<()> {
    let myexamplemore = MyOrgExampleMore { sleep_duration };
    let myinterface = org_example_more::new(Box::new(myexamplemore));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );
    varlink::listen(service, &address, 1, 10, timeout)?;
    Ok(())
}
