use std::alloc::System;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::str;

#[global_allocator]
static A: System = System;

use chainerror::*;
use clap::{App, Arg, SubCommand};
use colored_json::{ColorMode, ColoredFormatter, Colour, Output, PrettyFormatter, Style, Styler};

use varlink::{
    Connection, GetInterfaceDescriptionReply, MethodCall, OrgVarlinkServiceClient,
    OrgVarlinkServiceInterface,
};
use varlink_parser::{Format, FormatColored, Varlink};
use varlink_stdinterfaces::org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};

use crate::proxy::{handle, handle_connect};

#[cfg(test)]
mod test;

pub type Result<T> = std::result::Result<T, Box<std::error::Error>>;

mod proxy;

fn varlink_format(filename: &str, line_len: Option<&str>, should_colorize: bool) -> Result<()> {
    let mut buffer = String::new();
    File::open(Path::new(filename))
        .map_err(mstrerr!("Failed to open '{}'", filename))?
        .read_to_string(&mut buffer)
        .map_err(mstrerr!("Failed to read '{}'", filename))?;

    let v = Varlink::from_string(&buffer).map_err(mstrerr!("Failed to parse '{}'", buffer))?;
    if should_colorize {
        println!(
            "{}",
            v.interface
                .get_multiline_colored(0, line_len.unwrap_or("80").parse::<usize>().unwrap_or(80))
        );
    } else {
        println!(
            "{}",
            v.interface
                .get_multiline(0, line_len.unwrap_or("80").parse::<usize>().unwrap_or(80))
        );
    };
    Ok(())
}

fn varlink_info(
    address: Option<&str>,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
    should_colorize: bool,
) -> Result<()> {
    let connection = match activate {
        Some(activate) => Connection::with_activate(activate)
            .map_err(mstrerr!("Failed to connect with activate '{}'", activate))?,
        None => match bridge {
            Some(bridge) => Connection::with_bridge(bridge)
                .map_err(mstrerr!("Failed to connect with bridge '{}'", bridge))?,
            None => {
                let address = address.unwrap();
                if address.rfind(':').is_none() {
                    let conn = Connection::new(resolver)
                        .map_err(mstrerr!("Failed to connect with resolver '{}'", resolver))?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(address.into()).call() {
                        Ok(r) => r.address.clone(),
                        _ => Err(strerr!("Interface '{}' not found", address))?,
                    };
                    Connection::with_address(&address)
                        .map_err(mstrerr!("Failed to connect to '{}'", address))?
                } else {
                    Connection::with_address(&address)
                        .map_err(mstrerr!("Failed to connect to '{}'", address))?
                }
            }
        },
    };

    let mut call = OrgVarlinkServiceClient::new(connection);
    let info = call.get_info().map_err(mstrerr!("Cannot call GetInfo()"))?;

    let bold: fn(w: &str) -> String = if should_colorize {
        |w| Style::new().bold().paint(w).to_string()
    } else {
        |w| w.to_string()
    };

    println!("{} {}", bold("Vendor:"), info.vendor);
    println!("{} {}", bold("Product:"), info.product);
    println!("{} {}", bold("Version:"), info.version);
    println!("{} {}", bold("URL:"), info.url);
    println!("{}", bold("Interfaces:"));

    for i in info.interfaces {
        println!("  {}", i)
    }
    println!();
    Ok(())
}

fn varlink_help(
    url: &str,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
    columns: Option<&str>,
    should_colorize: bool,
) -> Result<()> {
    let address: &str;
    let interface: &str;

    let connection = if let Some(del) = url.rfind('/') {
        address = &url[0..del];
        interface = &url[(del + 1)..];
        Connection::with_address(&address).map_err(mstrerr!("Cannot connect to '{}'", address))?
    } else {
        interface = url;
        match activate {
            Some(activate) => Connection::with_activate(activate)
                .map_err(mstrerr!("Failed to connect with activate '{}'", activate))?,
            None => match bridge {
                Some(bridge) => Connection::with_bridge(bridge)
                    .map_err(mstrerr!("Failed to connect with bridge '{}'", bridge))?,
                None => {
                    let conn = Connection::new(resolver)
                        .map_err(mstrerr!("Failed to connect with resolver '{}'", resolver))?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => r.address.clone(),
                        _ => Err(strerr!("Interface '{}' not found", interface))?,
                    };
                    Connection::with_address(&address)
                        .map_err(mstrerr!("Failed to connect to '{}'", address))?
                }
            },
        }
    };

    if interface.find('.') == None {
        Err(strerr!("Invalid address {}", url))?
    }

    let mut call = OrgVarlinkServiceClient::new(connection);
    match call
        .get_interface_description(interface.to_string())
        .map_err(mstrerr!(
            "Can't get interface description for {}",
            interface
        ))? {
        GetInterfaceDescriptionReply {
            description: Some(desc),
        } => {
            if should_colorize {
                println!(
                    "{}",
                    Varlink::from_string(&desc)
                        .map_err(mstrerr!("Can't parse '{}'", desc))?
                        .interface
                        .get_multiline_colored(
                            0,
                            columns.unwrap_or("80").parse::<usize>().unwrap_or(80),
                        )
                );
            } else {
                println!(
                    "{}",
                    Varlink::from_string(&desc)
                        .map_err(mstrerr!("Can't parse '{}'", desc))?
                        .interface
                        .get_multiline(0, columns.unwrap_or("80").parse::<usize>().unwrap_or(80))
                );
            }
        }
        _ => Err(strerr!("No description for {}", url))?,
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
    should_colorize: bool,
) -> Result<()> {
    let resolved_address: String;
    let address: &str;
    let interface: &str;
    let method: &str;

    let connection = match activate {
        Some(activate) => {
            method = url;
            Connection::with_activate(activate)
                .map_err(mstrerr!("Failed to connect with activate '{}'", activate))?
        }
        None => match bridge {
            Some(bridge) => {
                method = url;
                Connection::with_bridge(bridge)
                    .map_err(mstrerr!("Failed to connect with bridge '{}'", bridge))?
            }
            None => {
                if let Some(del) = url.rfind('/') {
                    address = &url[0..del];
                    method = &url[(del + 1)..];
                    if method.find('.') == None {
                        return Err(strerr!("Invalid address {}", url).into());
                    }
                } else {
                    if let Some(del) = url.rfind('.') {
                        interface = &url[0..del];
                        method = url;
                        if method.find('.') == None {
                            return Err(strerr!("Invalid address {}", url).into());
                        }
                    } else {
                        return Err(strerr!("Invalid address {}", url).into());
                    }
                    let conn = Connection::new(resolver)
                        .map_err(mstrerr!("Failed to connect with resolver '{}'", resolver))?;
                    let mut resolver = VarlinkClient::new(conn);
                    address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => {
                            resolved_address = r.address.clone();
                            resolved_address.as_ref()
                        }
                        _ => Err(strerr!("Interface '{}' not found", interface))?,
                    };
                }
                Connection::with_address(address)
                    .map_err(mstrerr!("Failed to connect to '{}'", address))?
            }
        },
    };

    let args = match args {
        Some(args) => serde_json::from_str(args)
            .map_err(mstrerr!("Failed to parse JSON for '{}'", args.to_string()))?,
        None => serde_json::Value::Null,
    };

    let mut call = MethodCall::<serde_json::Value, serde_json::Value, varlink::ErrorKind>::new(
        connection.clone(),
        String::from(method),
        args.clone(),
    );

    let color_mode = if should_colorize {
        ColorMode::On
    } else {
        ColorMode::Off
    };

    let cf = ColoredFormatter::with_styler(
        PrettyFormatter::new(),
        Styler {
            array_brackets: Style::new(),
            object_brackets: Style::new(),
            key: Colour::Cyan.normal(),
            string_value: Colour::Purple.normal(),
            integer_value: Colour::Purple.normal(),
            float_value: Colour::Purple.normal(),
            bool_value: Colour::Purple.normal(),
            nil_value: Colour::Purple.normal(),
            string_include_quotation: false,
        },
    );

    if !more {
        let reply =
            call.call()
                .map_err(mstrerr!("Failed to call method '{}({})'", &method, &args))?;
        println!(
            "{}",
            cf.to_colored_json(&reply, color_mode)
                .map_err(mstrerr!("Failed to print json for '{}'", reply))?
        );
    } else {
        for reply in
            call.more()
                .map_err(mstrerr!("Failed to call method '{}({})'", method, args))?
        {
            println!(
                "{}",
                cf.clone()
                    .to_colored_json(
                        &reply.map_err(mstrerr!("Failed to call method '{}({})'", method, args))?,
                        color_mode,
                    )
                    .map_err(mstrerr!("Failed to print json for reply"))?
            );
        }
    }

    Ok(())
}

#[cfg(unix)]
fn varlink_bridge(address: Option<&str>) -> Result<()> {
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let stdin = ::std::io::stdin();
    let stdout = ::std::io::stdout();

    let inbuf =
        unsafe { ::std::io::BufReader::new(::std::fs::File::from_raw_fd(stdin.as_raw_fd())) };
    let outw = unsafe { ::std::fs::File::from_raw_fd(stdout.as_raw_fd()) };

    if let Some(address) = address {
        handle_connect(address, inbuf, outw)
            .map_err(mstrerr!("Error bridging to address {}", address))?;
    } else {
        handle(inbuf, outw).map_err(mstrerr!("Error bridging"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn varlink_bridge(address: Option<&str>) -> Result<()> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};

    let stdin = ::std::io::stdin();
    let stdout = ::std::io::stdout();

    let inbuf = unsafe {
        ::std::io::BufReader::new(::std::fs::File::from_raw_handle(stdin.as_raw_handle()))
    };
    let outw = unsafe { ::std::fs::File::from_raw_handle(stdout.as_raw_handle()) };

    if let Some(address) = address {
        handle_connect(address, inbuf, outw)
            .map_err(mstrerr!("Error bridging to address {}", address))?;
    } else {
        handle(inbuf, outw).map_err(mstrerr!("Error bridging"))?;
    }
    Ok(())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    #[cfg(windows)]
    let _enabled = colored_json::enable_ansi_support();

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
        .arg(Arg::with_name("debug").long("debug").help("print debug"))
        .arg(
            Arg::with_name("color")
                .long("color")
                .possible_values(&["on", "off", "auto"])
                .default_value("auto")
                .help("colorize output"),
        )
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
                .required(false),
        )
        .arg(
            Arg::with_name("activate")
                .short("A")
                .long("activate")
                .value_name("COMMAND")
                .help("Service to socket-activate and connect to")
                .long_help(
                    "Service to socket-activate and connect to. The temporary UNIX socket \
                     address is exported as $VARLINK_ADDRESS.",
                )
                .takes_value(true)
                .required(false),
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
                    Arg::with_name("COLUMNS")
                        .short("c")
                        .long("cols")
                        .help("maximum width of the output")
                        .required(false)
                        .takes_value(true),
                )
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
                )
                .arg(
                    Arg::with_name("COLUMNS")
                        .short("c")
                        .long("cols")
                        .help("maximum width of the output")
                        .required(false)
                        .takes_value(true),
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

    if let Err(e) = do_main(&mut app) {
        if app.get_matches().is_present("debug") {
            eprintln!("{:?}", e);
        } else {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    }
}

fn do_main(app: &mut App) -> Result<()> {
    let matches = app.clone().get_matches();
    let resolver = matches.value_of("resolver").unwrap();
    let bridge = matches.value_of("bridge");
    let activate = matches.value_of("activate");
    let color = matches.value_of("color").unwrap();
    let color_bool = match color {
        "on" => true,
        "off" => false,
        _ => ColorMode::should_colorize(&Output::StdOut),
    };

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            app.gen_completions_to("varlink", shell.parse().unwrap(), &mut io::stdout());
        }
        ("format", Some(sub_matches)) => {
            let filename = sub_matches.value_of("FILE").unwrap();
            let cols = sub_matches.value_of("COLUMNS");

            varlink_format(filename, cols, color_bool)?
        }
        ("info", Some(sub_matches)) => {
            let address = sub_matches.value_of("ADDRESS");
            if address.is_none() && activate.is_none() && bridge.is_none() {
                app.print_help().map_err(mstrerr!("Couldn't print help"))?;
                println!();
                Err(strerr!("No ADDRESS or activation or bridge"))?
            }

            varlink_info(address, resolver, activate, bridge, color_bool)?
        }
        ("bridge", Some(sub_matches)) => {
            let address = sub_matches.value_of("connect");
            varlink_bridge(address)?
        }
        ("help", Some(sub_matches)) => {
            let interface = sub_matches.value_of("INTERFACE").unwrap();
            let cols = sub_matches.value_of("COLUMNS");
            varlink_help(interface, resolver, activate, bridge, cols, color_bool)?
        }
        ("call", Some(sub_matches)) => {
            let method = sub_matches.value_of("METHOD").unwrap();
            let args = sub_matches.value_of("ARGUMENTS");
            let more = sub_matches.is_present("more");

            varlink_call(method, args, more, resolver, activate, bridge, color_bool)?
        }
        (_, _) => {
            app.print_help().map_err(mstrerr!("Couldn't print help"))?;
            println!();
        }
    }
    Ok(())
}
