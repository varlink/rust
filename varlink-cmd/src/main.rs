extern crate clap;
extern crate serde_json;
extern crate varlink;
extern crate varlink_parser;

#[macro_use]
extern crate error_chain;

use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::str;
use varlink::{Connection, GetInterfaceDescriptionReply, MethodCall, OrgVarlinkServiceClient,
              OrgVarlinkServiceInterface};
use varlink_parser::Varlink;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Fmt(::std::fmt::Error);
            SerdeJson(::serde_json::Error);
            Clap(::clap::Error);
        }
        links {
            Varlink(::varlink::Error, ::varlink::ErrorKind);
        }
        errors {
            NotImplemented(t: String) {
                display("Not yet implemented: '{}'", t)
            }
        }
    }
}

use errors::*;

fn varlink_format(filename: &str) -> Result<()> {
    let mut buffer = String::new();
    File::open(Path::new(filename))?.read_to_string(&mut buffer)?;

    let v = Varlink::from_string(&buffer)?;
    println!("{}", v.interface);
    Ok(())
}

fn varlink_info(address: &str) -> Result<()> {
    let conn = Connection::new(address)?;
    let mut call = OrgVarlinkServiceClient::new(conn);
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

fn varlink_help(url: &str) -> Result<()> {
    let del: Vec<&str> = url.split("/").collect();
    let address: &str;
    let interface: &str;

    match del.len() {
        2 => {
            address = del[0];
            interface = del[1];
        }
        _ => {
            return Err(
                ErrorKind::NotImplemented(format!("Resolver not yet implemented {}", url)).into(),
            );
        }
    };

    let conn = Connection::new(address)?;
    let mut call = OrgVarlinkServiceClient::new(conn);
    match call.get_interface_description(interface.into())? {
        GetInterfaceDescriptionReply {
            description: Some(desc),
        } => println!("{}", desc),
        _ => {
            return Err(ErrorKind::NotImplemented(format!("No description for {}", url)).into());
        }
    };

    Ok(())
}

fn varlink_call(url: &str, args: Option<&str>, more: bool) -> Result<()> {
    let del: Vec<&str> = url.split("/").collect();
    let address: &str;
    let method: &str;

    match del.len() {
        2 => {
            address = del[0];
            method = del[1];
        }
        _ => {
            return Err(
                ErrorKind::NotImplemented(format!("Resolver not yet implemented {}", url)).into(),
            );
        }
    };

    let conn = Connection::new(address)?;
    let args = match args {
        Some(args) => serde_json::from_str(args)?,
        None => serde_json::Value::Null,
    };

    let mut call = MethodCall::<serde_json::Value, serde_json::Value, varlink::Error>::new(
        conn.clone(),
        String::from(method).into(),
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

fn main() -> Result<()> {
    let mut app = App::new("varlink")
        .version("0.1")
        .arg(
            Arg::with_name("timeout")
                .short("t")
                .long("timeout")
                .value_name("SECONDS")
                .help("time in seconds to wait for a reply")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("resolver")
                .short("R")
                .long("resolver")
                .value_name("ADDRESS")
                .help("address of the resolver")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("bridge")
                .about("Bridge varlink messages to services on this machine")
                .long_about(
                    "Bridge varlink messages on standard in and out to varlink services on this \
                     machine.",
                ),
        )
        .subcommand(
            SubCommand::with_name("call")
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
                .about("Format a varlink service file")
                .arg(
                    Arg::with_name("FILE")
                        .required(true)
                        .help("The varlink interface definition file to format"),
                ),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("Print information about a service")
                .long_about("Prints information about the service running at ADDRESS.")
                .arg(Arg::with_name("ADDRESS").required(true)),
        )
        .subcommand(
            SubCommand::with_name("help")
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
                .about("Resolve an interface name to a varlink address")
                .long_about("Resolve INTERFACE to the varlink address that implements it.")
                .arg(Arg::with_name("INTERFACE").required(true)),
        )
        .subcommand(
            SubCommand::with_name("completions")
                .about("Generates completion scripts for your shell")
                .arg(
                    Arg::with_name("SHELL")
                        .required(true)
                        .possible_values(&["bash", "fish", "zsh"])
                        .help("The shell to generate the script for"),
                ),
        );
    let matches = app.clone().get_matches();

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
            let address = sub_matches.value_of("ADDRESS").unwrap();
            varlink_info(address)?
        }
        ("help", Some(sub_matches)) => {
            let interface = sub_matches.value_of("INTERFACE").unwrap();
            varlink_help(interface)?
        }
        ("call", Some(sub_matches)) => {
            let method = sub_matches.value_of("METHOD").unwrap();
            let args = sub_matches.value_of("ARGUMENTS");
            let more = sub_matches.is_present("more");
            varlink_call(method, args, more)?
        }
        (_, _) => app.print_help()?,
    }
    Ok(())
}
