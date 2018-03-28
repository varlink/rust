#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_ping::*;
use std::env;
use std::io;
use std::process::exit;

mod org_example_ping;

fn run_app(address: String) -> io::Result<()> {
    let connection = varlink::Connection::new(&address)?;
    let mut c = VarlinkClient::new(connection);
    let ping: Option<String> = Some("Test".into());
    let reply = c.ping(ping.clone())?.recv()?;
    println!("Got {:?}", reply);
    assert_eq!(ping, reply.pong);
    Ok(())
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

    exit(match run_app(args[1].clone()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}
