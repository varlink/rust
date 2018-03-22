#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_ping::*;
use std::env;
use std::io;
use std::process::exit;
use varlink::VarlinkService;

// Dynamically build the varlink rust code.
mod org_example_ping;

struct MyOrgExamplePing;

impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
    fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
        return call.reply(ping);
    }
}

fn run_app(address: String, timeout: u64) -> io::Result<()> {
    let myorgexampleping = MyOrgExamplePing;
    let myinterface = org_example_ping::new(Box::new(myorgexampleping));
    let service = VarlinkService::new(
        "org.varlink",
        "test ping service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    varlink::listen(service, &address, 10, timeout)
}

fn main() {
    let args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    exit(match run_app(args[1].clone(), 0) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

#[test]
fn test_unix() {
    assert!(run_app("unix:/tmp/org.example.ping_unix".into(), 1).is_ok());
}
