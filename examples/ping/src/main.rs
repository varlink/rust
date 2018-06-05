extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate getopts;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_ping::*;
use std::env;
use std::process::exit;
use varlink::VarlinkService;

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

    let address = match matches.opt_str("varlink") {
        None => {
            if !client_mode {
                eprintln!("Need varlink address in server mode.");
                print_usage(&program, &opts);
                return;
            }
            format!("exec:{}", program)
        }
        Some(a) => a,
    };

    let ret = if client_mode {
        run_client(&address)
    } else {
        run_server(&address, 0).map_err(|e| e.into())
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

fn run_client(address: &str) -> Result<()> {
    let connection = varlink::Connection::new(&address)?;
    let mut iface = VarlinkClient::new(connection);
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

    Ok(())
}

// Server

struct MyOrgExamplePing;

impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
    fn ping(&self, call: &mut Call_Ping, ping: String) -> varlink::Result<()> {
        call.reply(ping)
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

    varlink::listen(service, &address, 10, timeout)?;
    Ok(())
}
