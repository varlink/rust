use chainerror::*;
use std::env;
use std::process::exit;
use std::sync::{Arc, RwLock};

use varlink::{Connection, OrgVarlinkServiceInterface, VarlinkService};

use crate::io_systemd_network::VarlinkClientInterface;

mod io_systemd_network;

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

    let ret: Result<()> = if client_mode {
        let connection = match matches.opt_str("varlink") {
            None => Connection::with_activate(&format!("{} --varlink=$VARLINK_ADDRESS", program))
                .unwrap(),
            Some(address) => Connection::with_address(&address).unwrap(),
        };
        run_client(connection)
    } else if let Some(address) = matches.opt_str("varlink") {
        run_server(&address, 0).map_err(|e| e.into())
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
    let mut iface = varlink::OrgVarlinkServiceClient::new(connection.clone());
    {
        let info = iface
            .get_info()
            .map_err(mstrerr!("Error calling get_info()"))?;
        assert_eq!(&info.vendor, "org.varlink");
        assert_eq!(&info.product, "test service");
        assert_eq!(&info.version, "0.1");
        assert_eq!(&info.url, "http://varlink.org");
        assert_eq!(&info.interfaces[1], "io.systemd.network");
    }
    let description = iface
        .get_interface_description("io.systemd.network")
        .map_err(mstrerr!("Error calling get_interface_description()"))?;

    assert!(description.description.is_some());

    let mut iface = io_systemd_network::VarlinkClient::new(connection);

    match iface.list().call() {
        Ok(io_systemd_network::List_Reply { netdevs: vec }) => {
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0].ifindex, 1);
            assert_eq!(vec[0].ifname, String::from("lo"));
            assert_eq!(vec[1].ifindex, 2);
            assert_eq!(vec[1].ifname, String::from("eth0"));
        }
        res => panic!("Unknown result {:?}", res),
    }

    match iface.info(1).call() {
        Ok(io_systemd_network::Info_Reply {
            info:
                io_systemd_network::NetdevInfo {
                    ifindex: 1,
                    ifname: ref p,
                },
        }) if p == "lo" => {}
        res => panic!("Unknown result {:?}", res),
    }

    match iface.info(2).call() {
        Ok(io_systemd_network::Info_Reply {
            info:
                io_systemd_network::NetdevInfo {
                    ifindex: 2,
                    ifname: ref p,
                },
        }) if p == "eth" => {}
        res => panic!("Unknown result {:?}", res),
    }

    let e = iface.info(3).call().err().unwrap();

    match e.kind() {
        io_systemd_network::ErrorKind::Varlink_Error => {
            let e = e.find_chain_cause::<varlink::ErrorKind>().unwrap();
            match e.kind() {
                varlink::ErrorKind::InvalidParameter(ref p) if p == "ifindex" => {}
                _ => panic!("Unknown result\n{:?}\n", e),
            }
        }
        _ => panic!("Unknown result\n{:?}\n", e),
    }

    match iface.info(4).call().err().unwrap().kind() {
        io_systemd_network::ErrorKind::UnknownNetworkIfIndex(Some(
            io_systemd_network::UnknownNetworkIfIndex_Args { ifindex: 4 },
        )) => {}
        res => panic!("Unknown result {:?}", res),
    }

    Ok(())
}

struct MyIoSystemdNetwork {
    pub state: Arc<RwLock<i64>>,
}

impl io_systemd_network::VarlinkInterface for MyIoSystemdNetwork {
    fn info(&self, call: &mut io_systemd_network::Call_Info, ifindex: i64) -> varlink::Result<()> {
        // State example
        {
            let mut number = self.state.write().unwrap();

            *number += 1;

            eprintln!("{}", *number);
        }

        match ifindex {
            1 => call.reply(io_systemd_network::NetdevInfo {
                ifindex: 1,
                ifname: "lo".into(),
            }),
            2 => call.reply(io_systemd_network::NetdevInfo {
                ifindex: 2,
                ifname: "eth".into(),
            }),
            3 => {
                call.reply_invalid_parameter("ifindex".into())?;
                Ok(())
            }
            _ => call.reply_unknown_network_if_index(ifindex),
        }
    }

    fn list(&self, call: &mut io_systemd_network::Call_List) -> varlink::Result<()> {
        // State example
        {
            let mut number = self.state.write().unwrap();

            *number -= 1;

            eprintln!("{}", *number);
        }
        call.reply(vec![
            io_systemd_network::Netdev {
                ifindex: 1,
                ifname: "lo".into(),
            },
            io_systemd_network::Netdev {
                ifindex: 2,
                ifname: "eth0".into(),
            },
        ])
    }
}

fn run_server<S: ?Sized + AsRef<str>>(address: &S, timeout: u64) -> varlink::Result<()> {
    let state = Arc::new(RwLock::new(0));
    let myiosystemdnetwork = MyIoSystemdNetwork { state };
    let myinterface = io_systemd_network::new(Box::new(myiosystemdnetwork));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    varlink::listen(service, address, 1, 10, timeout)?;
    Ok(())
}
