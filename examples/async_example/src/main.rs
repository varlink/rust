use std::env;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use tokio::sync::RwLock;
use varlink::listen_async;

// Using the `varlink_derive::varlink_file_async!` macro for async code generation
varlink_derive::varlink_file_async!(org_example_network, "src/org.example.network.varlink");

use crate::org_example_network::VarlinkClientInterface;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Main

fn print_usage(program: &str, opts: &getopts::Options) {
    let brief = format!("Usage: {} [--varlink=<address>] [--client]", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
async fn main() {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optopt("", "varlink", "varlink address URL", "<address>");
    opts.optflag("", "client", "run in client mode");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f);
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
        let address = matches
            .opt_str("varlink")
            .unwrap_or_else(|| "tcp:127.0.0.1:12345".to_string());
        run_client(&address).await
    } else if let Some(address) = matches.opt_str("varlink") {
        run_server(&address, 10).await.map_err(|e| e.into())
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

async fn run_client(address: &str) -> Result<()> {
    let connection = varlink::AsyncConnection::with_address(address)
        .await
        .context("Failed to create connection")?;

    let client = org_example_network::VarlinkClient::new(connection);

    match client.list().call().await {
        Ok(org_example_network::List_Reply { netdevs: vec }) => {
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0].ifindex, 1);
            assert_eq!(vec[0].ifname, String::from("lo"));
            assert_eq!(vec[1].ifindex, 2);
            assert_eq!(vec[1].ifname, String::from("eth0"));
        }
        res => panic!("Unknown result {:?}", res),
    }

    match client.info(1).call().await {
        Ok(org_example_network::Info_Reply {
            info:
                org_example_network::NetdevInfo {
                    ifindex: 1,
                    ifname: ref p,
                },
        }) if p == "lo" => {}
        res => panic!("Unknown result {:?}", res),
    }

    match client.info(2).call().await {
        Ok(org_example_network::Info_Reply {
            info:
                org_example_network::NetdevInfo {
                    ifindex: 2,
                    ifname: ref p,
                },
        }) if p == "eth" => {}
        res => panic!("Unknown result {:?}", res),
    }

    let e = client.info(3).call().await.err().unwrap();
    match e.kind() {
        org_example_network::ErrorKind::Varlink_Error => match e.source_varlink_kind() {
            Some(varlink::ErrorKind::InvalidParameter(ref p)) if p == "ifindex" => {}
            _ => panic!("Unknown result\n{:?}\n", e),
        },
        _ => panic!("Unknown result\n{:?}\n", e),
    }

    let e = client.info(4).call().await.err().unwrap();
    match e.source_varlink_kind() {
        Some(varlink::ErrorKind::VarlinkErrorReply(varlink::Reply {
            error: Some(ref t), ..
        })) if t == "org.example.network.UnknownNetworkIfIndex" => {}
        _ => panic!("Unknown result\n{:?}\n", e),
    }

    match client.info(4).call().await.err().unwrap().kind() {
        org_example_network::ErrorKind::UnknownNetworkIfIndex(Some(
            org_example_network::UnknownNetworkIfIndex_Args { ifindex: 4 },
        )) => {}
        res => panic!("Unknown result {:?}", res),
    }

    Ok(())
}

// Server

struct MyOrgExampleNetwork {
    pub state: Arc<RwLock<i64>>,
}

#[async_trait]
impl org_example_network::VarlinkInterface for MyOrgExampleNetwork {
    async fn info(
        &self,
        call: &mut dyn org_example_network::Call_Info,
        ifindex: i64,
    ) -> varlink::Result<()> {
        // State example
        {
            let mut number = self.state.write().await;
            *number += 1;
            eprintln!("{}", *number);
        }

        match ifindex {
            1 => call.reply(org_example_network::NetdevInfo {
                ifindex: 1,
                ifname: "lo".into(),
            }),
            2 => call.reply(org_example_network::NetdevInfo {
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

    async fn list(&self, call: &mut dyn org_example_network::Call_List) -> varlink::Result<()> {
        // State example
        {
            let mut number = self.state.write().await;
            *number -= 1;
            eprintln!("{}", *number);
        }
        call.reply(vec![
            org_example_network::Netdev {
                ifindex: 1,
                ifname: "lo".into(),
            },
            org_example_network::Netdev {
                ifindex: 2,
                ifname: "eth0".into(),
            },
        ])
    }
}

async fn run_server<S: ?Sized + AsRef<str>>(address: &S, timeout: u64) -> varlink::Result<()> {
    let state = Arc::new(RwLock::new(0));
    let myiosystemdnetwork = Arc::new(MyOrgExampleNetwork { state });
    let service = Arc::new(org_example_network::new(myiosystemdnetwork));

    listen_async(
        service,
        address.as_ref().to_string(),
        &varlink::ListenAsyncConfig {
            idle_timeout: Duration::from_secs(timeout),
            ..Default::default()
        },
    )
    .await?;

    Ok(())
}
