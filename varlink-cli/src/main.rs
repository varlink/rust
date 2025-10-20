use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::str;

use clap::{App, Arg, SubCommand};
use colored_json::{
    Color, ColorMode, ColoredFormatter, Output, Paint, PrettyFormatter, Style, Styler,
};

use varlink::{
    Connection, GetInterfaceDescriptionReply, MethodCall, OrgVarlinkServiceClient,
    OrgVarlinkServiceInterface,
};
use varlink_parser::{Format, FormatColored, IDL};
use varlink_stdinterfaces::org_varlink_resolver::{VarlinkClient, VarlinkClientInterface};

#[cfg(target_os = "linux")]
mod proxy;
#[cfg(target_os = "linux")]
mod watchclose_epoll;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static + Send + Sync>>;

fn varlink_format(filename: &str, line_len: Option<&str>, should_colorize: bool) -> Result<()> {
    let mut buffer = String::new();
    File::open(Path::new(filename))
        .map_err(|e| format!("Failed to open '{filename}': {e}"))?
        .read_to_string(&mut buffer)
        .map_err(|e| format!("Failed to read '{filename}': {e}"))?;

    let idl =
        IDL::try_from(buffer.as_str()).map_err(|e| format!("Failed to parse '{filename}': {e}"))?;

    if should_colorize {
        println!(
            "{}",
            idl.get_multiline_colored(0, line_len.unwrap_or("80").parse::<usize>().unwrap_or(80))
        );
    } else {
        println!(
            "{}",
            idl.get_multiline(0, line_len.unwrap_or("80").parse::<usize>().unwrap_or(80))
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
    let bold: fn(w: &str) -> String = if should_colorize {
        static BOLD: Style = Style::new().bold();
        |w| w.paint(BOLD).to_string()
    } else {
        |w| w.to_string()
    };

    let connection = match activate {
        Some(activate) => Connection::with_activate(activate)
            .map_err(|e| format!("Failed to connect with activate '{activate}': {e}"))?,
        None => match bridge {
            Some(bridge) => Connection::with_bridge(bridge)
                .map_err(|e| format!("Failed to connect with bridge '{bridge}': {e}"))?,
            None => {
                let address = address.unwrap();
                if address.rfind(':').is_none() {
                    let conn = Connection::new(resolver).map_err(|e| {
                        format!("Failed to connect with resolver '{resolver}': {e}")
                    })?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(address.into()).call() {
                        Ok(r) => r.address,
                        _ => return Err(format!("Interface '{}' not found", address).into()),
                    };
                    Connection::with_address(&address)
                        .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
                } else {
                    Connection::with_address(&address)
                        .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
                }
            }
        },
    };

    let mut call = OrgVarlinkServiceClient::new(connection);
    let info = call
        .get_info()
        .map_err(|e| format!("Cannot call GetInfo(): {e}"))?;

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
        Connection::with_address(&address)
            .map_err(|e| format!("Cannot connect to '{address}': {e}"))?
    } else {
        interface = url;
        match activate {
            Some(activate) => Connection::with_activate(activate)
                .map_err(|e| format!("Failed to connect with activate '{activate}': {e}"))?,
            None => match bridge {
                Some(bridge) => Connection::with_bridge(bridge)
                    .map_err(|e| format!("Failed to connect with bridge '{bridge}': {e}"))?,
                None => {
                    let conn = Connection::new(resolver).map_err(|e| {
                        format!("Failed to connect with resolver '{resolver}': {e}")
                    })?;
                    let mut resolver = VarlinkClient::new(conn);
                    let address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => r.address,
                        _ => return Err(format!("Interface '{interface}' not found").into()),
                    };
                    Connection::with_address(&address)
                        .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
                }
            },
        }
    };

    if interface.find('.').is_none() {
        return Err(format!("Invalid address {}", url).into());
    }

    let mut call = OrgVarlinkServiceClient::new(connection);
    match call
        .get_interface_description(interface.to_string())
        .map_err(|e| format!("Can't get interface description for '{interface}': {e}"))?
    {
        GetInterfaceDescriptionReply {
            description: Some(desc),
        } => {
            if should_colorize {
                println!(
                    "{}",
                    IDL::try_from(desc.as_str())
                        .map_err(|e| format!("Can't parse '{desc}': {e}"))?
                        .get_multiline_colored(
                            0,
                            columns.unwrap_or("80").parse::<usize>().unwrap_or(80),
                        )
                );
            } else {
                println!(
                    "{}",
                    IDL::try_from(desc.as_str())
                        .map_err(|e| format!("Can't parse '{desc}': {e}"))?
                        .get_multiline(0, columns.unwrap_or("80").parse::<usize>().unwrap_or(80))
                );
            }
        }
        _ => return Err(format!("No description for {}", url).into()),
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
                .map_err(|e| format!("Failed to connect with activate '{activate}': {e}"))?
        }
        None => match bridge {
            Some(bridge) => {
                method = url;
                Connection::with_bridge(bridge)
                    .map_err(|e| format!("Failed to connect with bridge '{bridge}': {e}"))?
            }
            None => {
                if let Some(del) = url.rfind('/') {
                    address = &url[0..del];
                    method = &url[(del + 1)..];
                    if method.find('.').is_none() {
                        return Err(format!("Invalid address {}", url).into());
                    }
                } else {
                    if let Some(del) = url.rfind('.') {
                        interface = &url[0..del];
                        method = url;
                        if method.find('.').is_none() {
                            return Err(format!("Invalid address {}", url).into());
                        }
                    } else {
                        return Err(format!("Invalid address {}", url).into());
                    }
                    let conn = Connection::new(resolver).map_err(|e| {
                        format!("Failed to connect with resolver '{resolver}': {e}")
                    })?;
                    let mut resolver = VarlinkClient::new(conn);
                    address = match resolver.resolve(interface.into()).call() {
                        Ok(r) => {
                            resolved_address = r.address;
                            resolved_address.as_ref()
                        }
                        _ => return Err(format!("Interface '{}' not found", interface).into()),
                    };
                }
                Connection::with_address(address)
                    .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
            }
        },
    };

    let args = match args {
        Some(args) => serde_json::from_str(args)
            .map_err(|e| format!("Failed to parse JSON for '{args}': {e}"))?,
        None => serde_json::Value::Null,
    };

    let mut call = MethodCall::<serde_json::Value, serde_json::Value, varlink::Error>::new(
        connection,
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
            key: Color::Cyan.foreground(),
            string_value: Color::Magenta.foreground(),
            integer_value: Color::Magenta.foreground(),
            float_value: Color::Magenta.foreground(),
            bool_value: Color::Magenta.foreground(),
            nil_value: Color::Magenta.foreground(),
            string_include_quotation: false,
            object_colon: Default::default(),
        },
    );

    if !more {
        let ret = call.call();
        print_call_ret(color_mode, cf, ret, should_colorize, method, &args)?
    } else {
        for ret in call
            .more()
            .map_err(|e| format!("Failed to call method '{method}({args})': {e}"))?
        {
            print_call_ret(color_mode, cf.clone(), ret, should_colorize, method, &args)?
        }
    }

    Ok(())
}

fn print_call_ret(
    color_mode: ColorMode,
    cf: ColoredFormatter<PrettyFormatter>,
    ret: varlink::Result<serde_json::Value>,
    should_colorize: bool,
    method: &str,
    args: &serde_json::Value,
) -> Result<()> {
    let red: fn(w: &str) -> String = if should_colorize {
        |w| w.paint(Color::Red.foreground()).to_string()
    } else {
        |w| w.to_string()
    };

    let reply = ret.map_err(|e| {
        let cf = cf.clone();
        match e.kind() {
            varlink::ErrorKind::InterfaceNotFound(s) => format!(
                "Call failed with error: {}: {}",
                red("InterfaceNotFound"),
                s
            ),
            varlink::ErrorKind::MethodNotFound(s) => {
                format!("Call failed with error: {}: {}", red("MethodNotFound"), s)
            }
            varlink::ErrorKind::MethodNotImplemented(s) => format!(
                "Call failed with error: {}: {}",
                red("MethodNotImplemented"),
                s
            ),
            varlink::ErrorKind::InvalidParameter(s) => {
                format!("Call failed with error: {}: {}", red("InvalidParameter"), s)
            }
            varlink::ErrorKind::VarlinkErrorReply(varlink::Reply {
                error: Some(error),
                parameters: None,
                ..
            }) => format!("Call failed with error: {}", red(error)),
            varlink::ErrorKind::VarlinkErrorReply(varlink::Reply {
                error: Some(error),
                parameters: Some(parameters),
                ..
            }) => format!(
                "Call failed with error: {}\n{}",
                red(error),
                cf.to_colored_json(parameters, color_mode).unwrap()
            ),
            _ => format!("Failed to call method '{}({})'", &method, &args),
        }
    })?;

    println!(
        "{}",
        cf.to_colored_json(&reply, color_mode)
            .map_err(|e| format!("Failed to print json for '{reply}': {e}"))?
    );

    Ok(())
}

#[cfg(target_os = "linux")]
fn varlink_bridge(
    address: Option<&str>,
    resolver: &str,
    activate: Option<&str>,
    bridge: Option<&str>,
) -> Result<()> {
    use crate::proxy::{handle, handle_connect};

    let connection = match activate {
        Some(activate) => Connection::with_activate_no_rw(activate)
            .map_err(|e| format!("Failed to connect with activate '{activate}': {e}"))?,
        None => match bridge {
            Some(bridge) => Connection::with_bridge_no_rw(bridge)
                .map_err(|e| format!("Failed to connect with bridge '{bridge}': {e}"))?,
            None => {
                if let Some(address) = address {
                    if address.rfind(':').is_none() {
                        let conn = Connection::new(resolver).map_err(|e| {
                            format!("Failed to connect with resolver '{resolver}': {e}")
                        })?;
                        let mut resolver = VarlinkClient::new(conn);
                        let address = match resolver.resolve(address.into()).call() {
                            Ok(r) => r.address,
                            _ => return Err(format!("Interface '{}' not found", address).into()),
                        };
                        Connection::with_address_no_rw(&address)
                            .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
                    } else {
                        Connection::with_address_no_rw(&address)
                            .map_err(|e| format!("Failed to connect to '{address}': {e}"))?
                    }
                } else {
                    let stdin = ::std::io::stdin();
                    let stdout = ::std::io::stdout();
                    handle(resolver, stdin, stdout).map_err(|e| format!("Bridging: {e}"))?;
                    return Ok(());
                }
            }
        },
    };

    let stdin = ::std::io::stdin();
    let stdout = ::std::io::stdout();

    let r = handle_connect(connection, stdin, stdout);

    if let Err(ref e) = r {
        if let Some(io_e) = e.downcast_ref::<std::io::Error>() {
            if io_e.kind() == std::io::ErrorKind::BrokenPipe {
                return Ok(());
            }
        }
    }
    r.map_err(|e| format!("Bridging: {e}"))?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn varlink_bridge(
    _address: Option<&str>,
    _resolver: &str,
    _activate: Option<&str>,
    _bridge: Option<&str>,
) -> Result<()> {
    Err("Not implemented for this architecture. Waiting for a stable rust async interface.".into())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
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
        let matches = app.get_matches();

        let color = matches.value_of("color").unwrap();
        let color_bool = match color {
            "on" => true,
            "off" => false,
            _ => ColorMode::should_colorize(Output::StdOut),
        };
        let red_bold: fn(w: &str) -> String = if color_bool {
            |w| w.paint(Color::Red.bold()).to_string()
        } else {
            |w| w.to_string()
        };

        if matches.is_present("debug") {
            eprintln!("{:?}", e);
        } else {
            eprintln!("{} {}", red_bold("Error:"), e);
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
    let should_colorize = match color {
        "on" => true,
        "off" => false,
        _ => ColorMode::should_colorize(Output::StdOut),
    };

    match matches.subcommand() {
        ("completions", Some(sub_matches)) => {
            let shell = sub_matches.value_of("SHELL").unwrap();
            app.gen_completions_to("varlink", shell.parse().unwrap(), &mut io::stdout());
        }
        ("format", Some(sub_matches)) => {
            let filename = sub_matches.value_of("FILE").unwrap();
            let cols = sub_matches.value_of("COLUMNS");

            varlink_format(filename, cols, should_colorize)?
        }
        ("info", Some(sub_matches)) => {
            let address = sub_matches.value_of("ADDRESS");
            if address.is_none() && activate.is_none() && bridge.is_none() {
                app.print_help()
                    .map_err(|e| format!("Couldn't print help: {e}"))?;
                println!();
                return Err("No ADDRESS or activation or bridge".to_string().into());
            }

            varlink_info(address, resolver, activate, bridge, should_colorize)?
        }
        ("bridge", Some(sub_matches)) => {
            let address = sub_matches.value_of("connect");
            varlink_bridge(address, resolver, activate, bridge)?
        }
        ("help", Some(sub_matches)) => {
            let interface = sub_matches.value_of("INTERFACE").unwrap();
            let cols = sub_matches.value_of("COLUMNS");
            varlink_help(interface, resolver, activate, bridge, cols, should_colorize)?
        }
        ("call", Some(sub_matches)) => {
            let method = sub_matches.value_of("METHOD").unwrap();
            let args = sub_matches.value_of("ARGUMENTS");
            let more = sub_matches.is_present("more");

            varlink_call(
                method,
                args,
                more,
                resolver,
                activate,
                bridge,
                should_colorize,
            )?
        }
        (_, _) => {
            app.print_help()
                .map_err(|e| format!("Couldn't print help: {e}"))?;
            println!();
        }
    }
    Ok(())
}
