#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate varlink;

use std::io;
use std::io::{Error, ErrorKind};
use std::process::exit;
use std::env;
use std::{thread, time};

use varlink::VarlinkService;

// Dynamically build the varlink rust code.
mod org_example_more {
    include!(concat!(env!("OUT_DIR"), "/org.example.more.rs"));
}

use org_example_more::*;

struct MyOrgExampleMore {}

impl org_example_more::VarlinkInterface for MyOrgExampleMore {
    fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
        return call.reply(ping);
    }

    fn stop_serving(&self, call: &mut _CallStopServing) -> io::Result<()> {
        call.reply()?;
        Err(Error::new(ErrorKind::ConnectionRefused, "Disconnect"))
    }

    fn test_method_not_implemented(
        &self,
        call: &mut _CallTestMethodNotImplemented,
    ) -> io::Result<()> {
        return call.reply_method_not_implemented(Some("TestMethodNotImplemented".into()));
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

fn run_app() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Usage: {} <varlink address>", args[0]),
            ))
        }
    };

    let myexamplemore = MyOrgExampleMore {};
    let myinterface = org_example_more::new(Box::new(myexamplemore));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );
    varlink::server::listen(service, &args[1], 100, 10)
}

fn main() {
    exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}
