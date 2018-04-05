#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use io_systemd_network::*;
use std::env;
use std::io;
use std::process::exit;
use std::sync::{Arc, RwLock};
use varlink::VarlinkService;

mod io_systemd_network;

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

fn run_app(address: String, timeout: u64) -> io::Result<()> {
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
fn test_client() {
    use varlink::OrgVarlinkServiceInterface;
    use std::{thread, time};
    use io_systemd_network::*;

    fn run_client_app(address: String) -> io::Result<()> {
        let conn = varlink::Connection::new(&address)?;

        let mut call = varlink::OrgVarlinkServiceClient::new(conn.clone());
        let info = call.get_info()?.recv()?;
        assert_eq!(&info.vendor, "org.varlink");
        assert_eq!(&info.product, "test service");
        assert_eq!(&info.version, "0.1");
        assert_eq!(&info.url, "http://varlink.org");
        assert_eq!(
            info.interfaces.get(1).unwrap().as_ref(),
            "io.systemd.network"
        );

        let description = call.get_interface_description("io.systemd.network".into())?
            .recv()?;

        assert!(description.description.is_some());

        let mut call = VarlinkClient::new(conn);

        match call.list()?.recv() {
            Ok(ListReply_ { netdevs: Some(vec) }) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[0].ifindex, Some(1));
                assert_eq!(vec[0].ifname, Some(String::from("lo")));
                assert_eq!(vec[1].ifindex, Some(2));
                assert_eq!(vec[1].ifname, Some(String::from("eth0")));
            }
            res => panic!("Unknown result {:?}", res),
        }

        match call.info(Some(1))?.recv() {
            Ok(InfoReply_ {
                info:
                    Some(NetdevInfo {
                        ifindex: Some(1),
                        ifname: Some(ref p),
                    }),
            }) if p == "lo" => {}
            res => panic!("Unknown result {:?}", res),
        }

        match call.info(Some(2))?.recv() {
            Ok(InfoReply_ {
                info:
                    Some(NetdevInfo {
                        ifindex: Some(2),
                        ifname: Some(ref p),
                    }),
            }) if p == "eth" => {}
            res => panic!("Unknown result {:?}", res),
        }

        match call.info(Some(3))?.recv() {
            Err(Error_::VarlinkError_(varlink::Error::InvalidParameter(
                varlink::ErrorInvalidParameter {
                    parameter: Some(ref p),
                },
            ))) if p == "ifindex" => {}
            res => panic!("Unknown result {:?}", res),
        }

        match call.info(Some(4))?.recv() {
            Err(Error_::UnknownNetworkIfIndex(UnknownNetworkIfIndexArgs_ { ifindex: Some(4) })) => {
            }
            res => panic!("Unknown result {:?}", res),
        }

        Ok(())
    }

    let child = thread::spawn(move || {
        if let Err(e) = run_app("unix:/tmp/io.systemd.network_client".into(), 4) {
            panic!("error: {}", e);
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let res = run_client_app("unix:/tmp/io.systemd.network_client".into());
    if res.is_err() {
        eprintln!("{:?}", res);
    }
    assert!(res.is_ok());

    assert!(child.join().is_ok());
}
