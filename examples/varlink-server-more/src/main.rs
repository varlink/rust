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

struct MyOrgExampleMore;

impl VarlinkInterface for MyOrgExampleMore {
    fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
        return call.reply(ping);
    }

    fn stop_serving(&self, call: &mut _CallStopServing) -> io::Result<()> {
        call.reply()?;
        Err(Error::new(ErrorKind::ConnectionRefused, "Disconnect"))
    }

    fn test_more(&self, call: &mut _CallTestMore, n: Option<i64>) -> io::Result<()> {
        if n == None {
            return call.reply_invalid_parameter(Some("n".into()));
        }
        let n = n.unwrap();

        call.set_continues(true);

        call.reply(Some(State {
            start: Some(true),
            ..Default::default()
        }))?;

        for i in 0..n {
            thread::sleep(time::Duration::from_secs(1));
            call.reply(Some(State {
                progress: Some(i * 100 / n),
                ..Default::default()
            }))?;
        }

        call.reply(Some(State {
            progress: Some(100),
            ..Default::default()
        }))?;

        call.set_continues(false);

        call.reply(Some(State {
            end: Some(true),
            ..Default::default()
        }))
    }
}

fn run_app(address: String, timeout: u64) -> io::Result<()> {
    let myexamplemore = MyOrgExampleMore;
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

fn main() {
    let mut args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    if !args[1].starts_with("--varlink") {
        eprintln!("Usage: {} --varlink=<varlink address>", args[0]);
        exit(1);
    }

    exit(match run_app(args.swap_remove(1)[10..].into(), 0) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

#[test]
fn test_unix() {
    if let Err(e) = run_app("unix:/tmp/org.example.more_unix".into(), 1) {
        panic!("error: {}", e);
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_unix_abstract() {
    if let Err(e) = run_app("unix:@org.example.more_unix".into(), 1) {
        panic!("error: {}", e);
    }
}

#[test]
fn test_tcp() {
    if let Err(e) = run_app("tcp:0.0.0.0:12345".into(), 1) {
        panic!("error: {}", e);
    }
}
