#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate varlink;

use std::io;
use std::io::{Error, ErrorKind};
use std::process::exit;
use std::env;

use varlink::VarlinkService;

// Dynamically build the varlink rust code.
mod org_example_ping {
    include!(concat!(env!("OUT_DIR"), "/org.example.ping.rs"));
}

use org_example_ping::*;

struct MyOrgExamplePing {}

impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
    fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
        return call.reply(ping);
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

    let myorgexampleping = MyOrgExamplePing {};
    let myinterface = org_example_ping::new(Box::new(myorgexampleping));
    let service = VarlinkService::new(
        "org.varlink",
        "test ping service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    varlink::server::listen(service, &args[1], 10, 0)
}

fn main() {
    exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}
