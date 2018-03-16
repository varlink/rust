extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate varlink;

use std::io;
use std::thread;
use std::process::exit;

use varlink::VarlinkService;

use std::sync::{Arc, RwLock};
use std::net::{Shutdown, TcpListener};

// Dynamically build the varlink rust code.
//mod io_systemd_network;
mod io_systemd_network {
    include!(concat!(env!("OUT_DIR"), "/io.systemd.network.rs"));
}

use io_systemd_network::*;

struct MyIoSystemdNetwork {
    pub state: Arc<RwLock<i64>>,
}

impl io_systemd_network::VarlinkInterface for MyIoSystemdNetwork {
    fn info(&self, call: &mut _CallInfo, ifindex: Option<i64>) -> io::Result<()> {
        // State example
        {
            let mut number = self.state.write().unwrap();

            *number += 1;

            println!("{}", *number);
        }

        match ifindex {
            Some(1) => {
                return call.reply(Some(NetdevInfo {
                    ifindex: Some(1),
                    ifname: Some("lo".into()),
                }));
            }
            Some(2) => {
                return call.reply(Some(NetdevInfo {
                    ifindex: Some(2),
                    ifname: Some("eth".into()),
                }));
            }
            Some(3) => {
                return call.reply_invalid_parameter(Some("ifindex".into()));
            }
            _ => {
                return call.reply_unknown_network_if_index(ifindex);
            }
        }
    }

    fn list(&self, call: &mut _CallList) -> io::Result<()> {
        // State example
        {
            let mut number = self.state.write().unwrap();

            *number -= 1;

            println!("{}", *number);
        }
        return call.reply(Some(vec![
            Netdev {
                ifindex: Some(1),
                ifname: Some("lo".into()),
            },
            Netdev {
                ifindex: Some(2),
                ifname: Some("eth0".into()),
            },
        ]));
    }
}

fn run_app() -> io::Result<()> {
    thread::spawn(|| -> io::Result<()> {
        let addr = "0.0.0.0:12345";
        let listener = TcpListener::bind(addr)?;
        let state = Arc::new(RwLock::new(0));
        println!("Listening on {}", addr);
        let myiosystemdnetwork = MyIoSystemdNetwork { state };
        let myinterface = io_systemd_network::new(Box::new(myiosystemdnetwork));
        let service = Arc::new(VarlinkService::new(
            "org.varlink",
            "test service",
            "0.1",
            "http://varlink.org",
            vec![Box::new(myinterface)],
        ));

        loop {
            let (mut stream, _addr) = listener.accept()?;
            let service = service.clone();
            let _join = thread::spawn(move || -> io::Result<()> {
                let mut stream_clone = stream.try_clone().expect("clone failed...");
                if let Err(e) = service.handle(&mut stream, &mut stream_clone) {
                    println!("Handle Error: {}", e);
                }
                let _ = stream.shutdown(Shutdown::Both);
                Ok(())
            });
        }
    }).join()
        .unwrap()
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
