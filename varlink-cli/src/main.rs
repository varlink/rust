extern crate clap;
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate assert_cmd;
extern crate serde_json;
extern crate varlink;
extern crate varlink_parser;

use clap::{App, Arg, SubCommand};
use error::{ErrorKind, Result};
use failure::ResultExt;
use org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};
use proxy::{handle, handle_connect};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;
use std::str;
use varlink::{
    Connection, GetInterfaceDescriptionReply, MethodCall, OrgVarlinkServiceClient,
    OrgVarlinkServiceInterface,
};

use varlink_parser::Varlink;

#[cfg(test)]
mod test;

mod error;
mod org_varlink_resolver;
mod proxy;

fn varlink_format(filename: &str) -> Result<()> {
    let mut buffer = String::new();
    File::open(Path::new(filename))?.read_to_string(&mut buffer)?;

    let v = Varlink::from_string(&buffer)?;
    println!("{}", v.interface);
    Ok(())
}

fn varlink_info(
    address: Option<&str>,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
) -> Result<()> {
    let connection = match activate {
        Some(activate) => Connection::with_activate(activate)?,
        None => match bridge {
            Some(bridge) => Connection::with_bridge(bridge)?,
            None => {
                let address = address.unwrap();
                if address.rfind(':').is_none() {
                    let conn = Connection::new(resolver)?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(address.into()).call() {
                        Ok(r) => r.address.clone(),
                        _ => Err(varlink::Error::from(varlink::ErrorKind::InterfaceNotFound(
                            address.into(),
                        )))?,
                    };
                    Connection::with_address(&address)?
                } else {
                    Connection::with_address(&address)?
                }
            }
        },
    };
    let mut call = OrgVarlinkServiceClient::new(connection);
    let info = call.get_info()?;
    println!("Vendor: {}", info.vendor);
    println!("Product: {}", info.product);
    println!("Version: {}", info.version);
    println!("URL: {}", info.url);
    println!("Interfaces:");
    for i in info.interfaces {
        println!("  {}", i)
    }

    Ok(())
}

fn varlink_help(
    url: &str,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
) -> Result<()> {
    let address: &str;
    let interface: &str;

    let connection = if let Some(del) = url.rfind('/') {
        address = &url[0..del];
        interface = &url[(del + 1)..];
        Connection::with_address(&address)?
    } else {
        interface = url;
        match activate {
            Some(activate) => Connection::with_activate(activate)?,
            None => match bridge {
                Some(bridge) => Connection::with_bridge(bridge)?,
                None => {
                    let conn = Connection::new(resolver)?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => r.address.clone(),
                        _ => Err(varlink::Error::from(varlink::ErrorKind::InterfaceNotFound(
                            interface.into(),
                        )))?,
                    };
                    Connection::with_address(&address)?
                }
            },
        }
    };

    if interface.find('.') == None {
        Err(varlink::Error::from(varlink::ErrorKind::InvalidAddress))?
    }

    let mut call = OrgVarlinkServiceClient::new(connection);
    match call.get_interface_description(interface.to_string())? {
        GetInterfaceDescriptionReply {
            description: Some(desc),
        } => println!("{}", desc),
        _ => {
            return Err(ErrorKind::NoDescription(format!("No description for {}", url)).into());
        }
    };

    Ok(())
}

fn varlink_call(
    url: &str,
    args: Option<&str>,
    more: bool,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
) -> Result<()> {
    let resolved_address: String;
    let address: &str;
    let interface: &str;
    let method: &str;

    let connection = match activate {
        Some(activate) => {
            method = url;
            Connection::with_activate(activate)?
        }
        None => match bridge {
            Some(bridge) => {
                method = url;
                Connection::with_bridge(bridge)?
            }
            None => {
                if let Some(del) = url.rfind('/') {
                    address = &url[0..del];
                    method = &url[(del + 1)..];
                    if method.find('.') == None {
                        return Err(varlink::Error::from(varlink::ErrorKind::InvalidAddress).into());
                    }
                } else {
                    if let Some(del) = url.rfind('.') {
                        interface = &url[0..del];
                        method = url;
                        if method.find('.') == None {
                            return Err(
                                varlink::Error::from(varlink::ErrorKind::InvalidAddress).into()
                            );
                        }
                    } else {
                        return Err(varlink::Error::from(varlink::ErrorKind::InvalidAddress).into());
                    }
                    let conn = Connection::new(resolver)?;
                    let mut resolver = VarlinkClient::new(conn);
                    address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => {
                            resolved_address = r.address.clone();
                            resolved_address.as_ref()
                        }
                        _ => Err(varlink::Error::from(varlink::ErrorKind::InterfaceNotFound(
                            interface.into(),
                        )))?,
                    };
                }
                Connection::with_address(address).context(ErrorKind::Connection(address.into()))?
            }
        },
    };

    let args = match args {
        Some(args) => serde_json::from_str(args).context(ErrorKind::SerdeJsonDe(args.to_string()))?,
        None => serde_json::Value::Null,
    };

    let mut call = MethodCall::<serde_json::Value, serde_json::Value, varlink::Error>::new(
        connection.clone(),
        String::from(method),
        args,
    );

    if !more {
        let reply = call.call()?;
        println!("{}", serde_json::to_string_pretty(&reply)?);
    } else {
        for reply in call.more()? {
            println!("{}", serde_json::to_string_pretty(&reply?)?);
        }
    }

    Ok(())
}

fn varlink_bridge(address: Option<&str>) -> Result<()> {
    let stdin = ::std::io::stdin();
    let stdout = ::std::io::stdout();

    let inbuf =
        unsafe { ::std::io::BufReader::new(::std::fs::File::from_raw_fd(stdin.as_raw_fd())) };
    let outw = unsafe { ::std::fs::File::from_raw_fd(stdout.as_raw_fd()) };

    if let Some(address) = address {
        handle_connect(address, inbuf, outw)?;
    } else {
        handle(inbuf, outw)?;
    }
    Ok(())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let mut app = App::new("varlink")
        .version(VERSION)
        /*
                .arg(
                    Arg::with_name("timeout")
                        .short("t")
                        .long("timeout")
                        .value_name("SECONDS")
                        .help("time in seconds to wait for a reply")
                        .takes_value(true),
                )
        */
        .arg(
            Arg::with_name("resolver")
                .short("R")
                .long("resolver")
                .value_name("ADDRESS")
                .help("address of the resolver")
                .takes_value(true)
                .required(false)
                .default_value("unix:/run/org.varlink.resolver"),
        )
        .arg(
            Arg::with_name("bridge")
                .short("b")
                .long("bridge")
                .value_name("COMMAND")
                .help("Command to execute and connect to")
                .takes_value(true)
                .required(false)
        )
        .arg(
            Arg::with_name("activate")
                .short("A")
                .long("activate")
                .value_name("COMMAND")
                .help("Service to socket-activate and connect to")
                .long_help("Service to socket-activate and connect to. The temporary UNIX socket \
                address is exported as $VARLINK_ADDRESS.")
                .takes_value(true)
                .required(false)
        )
        .subcommand(
            SubCommand::with_name("bridge")
                .version(VERSION)
                .about("Bridge varlink messages from stdio to services on this machine")
                .long_about(
                    "Bridge varlink messages on stdin and stdout to varlink services on this \
                     machine.",
                )
                .arg(
                    Arg::with_name("connect")
                        .short("C")
                        .long("connect")
                        .value_name("ADDRESS")
                        .help("connect directly to ADDRESS")
                        .required(false)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("call")
                .version(VERSION)
                .about("Call a method")
                .long_about("Call METHOD on INTERFACE at ADDRESS. ARGUMENTS must be valid JSON.")
                .arg(
                    Arg::with_name("more")
                        .short("m")
                        .long("more")
                        .help("wait for multiple method returns if supported"),
                )
                .arg(
                    Arg::with_name("METHOD")
                        .value_name("[ADDRESS/]INTERFACE.METHOD")
                        .required(true),
                )
                .arg(Arg::with_name("ARGUMENTS").required(false)),
        )
        .subcommand(
            SubCommand::with_name("format")
                .version(VERSION)
                .about("Format a varlink service file")
                .arg(
                    Arg::with_name("FILE")
                        .required(true)
                        .help("The varlink interface definition file to format"),
                ),
        )
        .subcommand(
            SubCommand::with_name("info")
                .version(VERSION)
                .about("Print information about a service")
                .long_about("Prints information about the service running at ADDRESS.")
                .arg(Arg::with_name("ADDRESS").required(false)),
        )
        .subcommand(
            SubCommand::with_name("help")
                .version(VERSION)
                .about("Print interface description or service information")
                .long_about("Prints information about INTERFACE.")
                .arg(
                    Arg::with_name("INTERFACE")
                        .value_name("[ADDRESS/]INTERFACE")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("resolve")
                .version(VERSION)
                .about("Resolve an interface name to a varlink address")
                .long_about("Resolve INTERFACE to the varlink address that implements it.")
                .arg(Arg::with_name("INTERFACE").required(true)),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .version(VERSION)
                .about("Generates completion scripts for your shell")
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh"])
                        .help("The shell to generate the script for"),
                ),
        );
    let matches = app.clone().get_matches();
    let resolver = matches.value_of("resolver").unwrap();
    let bridge = matches.value_of("bridge");
    let activate = matches.value_of("activate");

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            app.gen_completions_to("varlink", shell.parse().unwrap(), &mut io::stdout());
        }
        ("format", Some(sub_matches)) => {
            let filename = sub_matches.value_of("FILE").unwrap();
            varlink_format(filename)?
        }
        ("info", Some(sub_matches)) => {
            let address = sub_matches.value_of("ADDRESS");
            if address.is_none() && activate.is_none() && bridge.is_none() {
                app.print_help().context(ErrorKind::Argument)?;
                println!();
                return Err(
                    ErrorKind::Connection("No ADDRESS or activation or bridge".into()).into(),
                );
            }

            varlink_info(address, resolver, activate, bridge)?
        }
        ("bridge", Some(sub_matches)) => {
            let address = sub_matches.value_of("connect");
            varlink_bridge(address)?
        }
        ("help", Some(sub_matches)) => {
            let interface = sub_matches.value_of("INTERFACE").unwrap();
            varlink_help(interface, resolver, activate, bridge)?
        }
        ("call", Some(sub_matches)) => {
            let method = sub_matches.value_of("METHOD").unwrap();
            let args = sub_matches.value_of("ARGUMENTS");
            let more = sub_matches.is_present("more");
            varlink_call(method, args, more, resolver, activate, bridge)?
        }
        (_, _) => {
            app.print_help().context(ErrorKind::Argument)?;
            println!();
        }
    }
    Ok(())
}
