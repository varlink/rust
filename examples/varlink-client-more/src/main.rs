#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_more::*;
use std::env;
use std::io;
use std::process::exit;

mod org_example_more;

fn run_app(address: String) -> io::Result<()> {
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

fn main() {
    let mut args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    exit(match run_app(args.swap_remove(1)) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}
