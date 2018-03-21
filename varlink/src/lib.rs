//!Server support for the [varlink protocol](http://varlink.org)
//!
//!To create a varlink server in rust, place your varlink interface definition file in src/.
//!E.g. `src/org.example.ping.varlink`:
//!
//!```varlink
//!# Example service
//!interface org.example.ping
//!
//!# Returns the same string
//!method Ping(ping: string) -> (pong: string)
//!```
//!
//!Then create a `build.rs` file in your project directory:
//!
//!```rust,no_run
//!extern crate varlink;
//!
//!fn main() {
//!    varlink::generator::cargo_build("src/org.example.ping.varlink");
//!}
//!```
//!
//!Add to your `Cargo.toml`:
//!
//!```toml
//![package]
//!build = "build.rs"
//!```
//!
//!In your `main.rs` you can then use:
//!
//!```rust,ignore
//!mod org_example_ping {
//!    include!(concat!(env!("OUT_DIR"), "/org.example.ping.rs"));
//!}
//!```
//!and then implement the interface:
//!
//!```no_run
//!# use std::io;
//!# use varlink;
//!# use varlink::CallTrait;
//!# struct _PingReply {pong: Option<String>}
//!# impl varlink::VarlinkReply for _PingReply {}
//!# struct _PingArgs {ping: Option<String>}
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallPing: _CallErr {
//!#     fn reply(&mut self, pong: Option<String>) -> io::Result<()> { Ok(()) }
//!# }
//!# impl<'a> _CallPing for varlink::Call<'a> {}
//!# pub trait VarlinkInterface {
//!#     fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()>;
//!#     fn call_upgraded(&self, _call: &mut varlink::Call) -> io::Result<()> {Ok(())}
//!# }
//!# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
//!# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
//!#     _InterfaceProxy { inner }
//!# }
//!# impl varlink::Interface for _InterfaceProxy {
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
//!#     fn get_name(&self) -> &'static str { "org.example.ping" }
//!#     fn call_upgraded(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
//!#     fn call(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
//!# }
//!struct MyOrgExamplePing;
//!
//!impl VarlinkInterface for MyOrgExamplePing {
//!    fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
//!        return call.reply(ping);
//!    }
//!}
//!```
//!to implement the interface methods.
//!
//!If your varlink method is called `TestMethod`, the rust method to be implemented is called
//!`test_method`. The first parameter is of type `_CallTestMethod`, which has the method `reply()`.
//!
//!```no_run
//!# use std::io;
//!# use varlink::CallTrait;
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallTestMethod: _CallErr {
//!#     fn reply(&mut self) -> io::Result<()> {
//!#         self.reply_struct(varlink::Reply::parameters(None))
//!#     }
//!# }
//!# impl<'a> _CallTestMethod for varlink::Call<'a> {}
//!# struct TestService;
//!# impl TestService {
//!fn test_method(&self, call: &mut _CallTestMethod, /* more arguments */) -> io::Result<()> {
//!    /* ... */
//!    return call.reply( /* more arguments */ );
//!}
//!# }
//!```
//!
//!A typical server creates a `VarlinkService` and starts a server via `varlink::listen()`
//!
//!```no_run
//!# use std::io;
//!# mod org_example_ping {
//!# use std::io;
//!# use varlink;
//!# struct _PingReply {pong: Option<String>}
//!# impl varlink::VarlinkReply for _PingReply {}
//!# struct _PingArgs {ping: Option<String>}
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallPing: _CallErr {
//!#     fn reply(&mut self, pong: Option<String>) -> io::Result<()> { Ok(()) }
//!# }
//!# impl<'a> _CallPing for varlink::Call<'a> {}
//!# pub trait VarlinkInterface {
//!#     fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()>;
//!#     fn call_upgraded(&self, _call: &mut varlink::Call) -> io::Result<()> {Ok(())}
//!# }
//!# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
//!# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
//!#     _InterfaceProxy { inner }
//!# }
//!# impl varlink::Interface for _InterfaceProxy {
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
//!#     fn get_name(&self) -> &'static str { "org.example.ping" }
//!#     fn call_upgraded(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
//!#     fn call(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
//!# }}
//!# use org_example_ping::*;
//!#
//!# struct MyOrgExamplePing;
//!#
//!# impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
//!#     fn ping(&self, call: &mut _CallPing, ping: Option<String>) -> io::Result<()> {
//!#         return call.reply(ping);
//!#     }
//!# }
//!# fn main() {
//!let args: Vec<_> = std::env::args().collect();
//!let myorgexampleping = MyOrgExamplePing;
//!let myorgexampleping_interface = org_example_ping::new(Box::new(myorgexampleping));
//!
//!let service = varlink::VarlinkService::new(
//!    "org.varlink",
//!    "test service",
//!    "0.1",
//!    "http://varlink.org",
//!    vec![
//!        Box::new(myorgexampleping_interface),
//!        /* more interfaces ...*/
//!    ],
//!);
//!
//!varlink::listen(service, &args[1], 10, 0);
//!# }
//!```
//!
//!where args[1] would follow the varlink
//![address specification](https://github.com/varlink/documentation/wiki#address).
//!
//!Currently supported address URIs are:
//!
//!- TCP `tcp:127.0.0.1:12345` hostname/IP address and port
//!- UNIX socket `unix:/run/org.example.ftl` optional access `;mode=0666` parameter
//!- UNIX abstract namespace socket `unix:@org.example.ftl` (on Linux only)
//!- executed binary `exec:/usr/bin/org.example.ftl` via
//!  [socket activation](https://github.com/varlink/documentation/wiki#activation)
//!  (on Linux only)

extern crate bytes;
extern crate itertools;
extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate unix_socket;
extern crate varlink_parser;

use serde::ser::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Read, Write};

pub mod generator;
mod server;

/// This trait has to be implemented by any varlink interface implementor.
/// All methods are generated by the varlink-rust-generator, so you don't have to care
/// about them.
pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call(&self, &mut Call) -> io::Result<()>;
    fn call_upgraded(&self, &mut Call) -> io::Result<()>;
}

/// The structure of a varlink request. Used to serialize json into it.
///
/// There should be no need to use this directly.
///
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Request {
    pub more: Option<bool>,
    pub oneshot: Option<bool>,
    pub upgrade: Option<bool>,
    pub method: Cow<'static, str>,
    pub parameters: Option<Value>,
}

/// Marker trait for the rust code generated by the varlink-rust-generator
///
/// There should be no need to use this directly.
/// See the [CallTrait](trait.CallTrait.html) to use with the first Call parameter
pub trait VarlinkReply {}

/// The structure of a varlink reply. Used to deserialize it into json.
///
/// There should be no need to use this directly.
/// See the [CallTrait](trait.CallTrait.html) to use with the first Call parameter
#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub continues: Option<bool>,
    pub upgraded: Option<bool>,
    pub error: Option<Cow<'static, str>>,
    pub parameters: Option<Value>,
}

impl Reply {
    pub fn parameters(parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
            upgraded: None,
            error: None,
            parameters: parameters,
        }
    }

    pub fn error(name: Cow<'static, str>, parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
            upgraded: None,
            error: Some(name),
            parameters,
        }
    }
}

impl<T> From<T> for Reply
where
    T: VarlinkReply + Serialize,
{
    fn from(a: T) -> Self {
        Reply::parameters(Some(serde_json::to_value(a).unwrap()))
    }
}

/// Call is a struct, which is passed as the first argument to the interface methods in a derived form.
///
/// See also the [CallTrait](trait.CallTrait.html) to use with the first Call parameter
///
/// #Examples
///
/// If your varlink method is called `TestMethod`, the rust method to be implemented is called
/// `test_method`. The first parameter is of type `_CallTestMethod`, which has the method `reply()`.
///
///# Examples
///
/// ```rust,no_run
///# use std::io;
///# use varlink;
///# use varlink::CallTrait;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethod: _CallErr {
///#     fn reply(&mut self) -> io::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method(&self, call: &mut _CallTestMethod, /* more arguments */) -> io::Result<()> {
///    /* ... */
///    return call.reply( /* more arguments */ );
///}
///# }
/// ```
pub struct Call<'a> {
    writer: &'a mut Write,
    pub request: Option<&'a Request>,
    continues: bool,
    upgraded: bool,
}

/// CallTrait provides convenience methods for the `Call` struct, which is passed as
/// the first argument to the interface methods.
///
///#  Examples
///
/// For an invalid parameter:
///
/// ```rust,no_run
///# use std::io;
///# use varlink;
///# use varlink::CallTrait;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethod: _CallErr {
///#     fn reply(&mut self) -> io::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method(&self, call: &mut _CallTestMethod, testparam: Option<i64>) -> io::Result<()> {
///    match testparam {
///        Some(i) => if i > 100 {
///            return call.reply_invalid_parameter(Some("testparam".into()));
///        },
///        None => {
///            return call.reply_invalid_parameter(Some("testparam".into()));
///        }
///    }
///    /* ... */
///    Ok(())
///}
///# }
/// ```
///
/// For not yet implemented methods:
///
/// ```rust,no_run
///# use std::io;
///# use varlink;
///# use varlink::CallTrait;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethodNotImplemented: _CallErr {
///#     fn reply(&mut self) -> io::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethodNotImplemented for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method_not_implemented(&self, call: &mut _CallTestMethodNotImplemented) -> io::Result<()> {
///    return call.reply_method_not_implemented(Some("TestMethodNotImplemented".into()));
///}
///# }
/// ```
pub trait CallTrait {
    /// Don't use this directly. Rather use the standard `reply()` method.
    fn reply_struct(&mut self, Reply) -> io::Result<()>;

    /// Set this to `true` to indicate, that more replies are following.
    ///
    ///# Examples
    ///
    ///```rust,no_run
    ///# use std::io;
    ///# use varlink::CallTrait;
    ///# pub trait _CallErr: varlink::CallTrait {}
    ///# impl<'a> _CallErr for varlink::Call<'a> {}
    ///# pub trait _CallTestMethod: _CallErr {
    ///#     fn reply(&mut self) -> io::Result<()> {
    ///#         self.reply_struct(varlink::Reply::parameters(None))
    ///#     }
    ///# }
    ///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
    ///# struct TestService;
    ///# impl TestService {
    ///fn test_method(&self, call: &mut _CallTestMethod) -> io::Result<()> {
    ///    call.set_continues(true);
    ///    call.reply( /* more args*/ )?;
    ///    call.reply( /* more args*/ )?;
    ///    call.reply( /* more args*/ )?;
    ///    call.set_continues(false);
    ///    return call.reply( /* more args*/ );
    ///}
    ///# }
    ///```
    fn set_continues(&mut self, cont: bool);

    /// reply with the standard varlink `org.varlink.service.MethodNotFound` error
    fn reply_method_not_found(&mut self, method_name: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotFound".into(),
            match method_name {
                Some(a) => {
                    let s = format!("{{  \"method\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    /// reply with the standard varlink `org.varlink.service.MethodNotImplemented` error
    fn reply_method_not_implemented(&mut self, method_name: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotImplemented".into(),
            match method_name {
                Some(a) => {
                    let s = format!("{{  \"method\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    /// reply with the standard varlink `org.varlink.service.InvalidParameter` error
    fn reply_invalid_parameter(&mut self, parameter_name: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InvalidParameter".into(),
            match parameter_name {
                Some(a) => {
                    let s = format!("{{  \"parameter\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }
}

impl<'a> CallTrait for Call<'a> {
    fn reply_struct(&mut self, mut reply: Reply) -> io::Result<()> {
        if self.continues && !self.wants_more() {
            return Err(Error::new(
                ErrorKind::Other,
                "Call::reply() called with continues, but without more in the request",
            ));
        }
        if self.continues {
            reply.continues = Some(true);
        }
        serde_json::to_writer(&mut *self.writer, &reply)?;
        self.writer.write_all(b"\0")?;
        self.writer.flush()?;
        Ok(())
    }
    fn set_continues(&mut self, cont: bool) {
        self.continues = cont;
    }
}

impl<'a> Call<'a> {
    fn new(writer: &'a mut Write, request: &'a Request) -> Self {
        Call {
            writer,
            request: Some(request),
            continues: false,
            upgraded: false,
        }
    }
    fn new_upgraded(writer: &'a mut Write) -> Self {
        Call {
            writer,
            request: None,
            continues: false,
            upgraded: false,
        }
    }

    /// True, if this request does not want a reply.
    pub fn is_oneshot(&self) -> bool {
        match self.request {
            Some(req) => {
                if let Some(val) = req.oneshot {
                    val
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// True, if this request accepts more than one reply.
    pub fn wants_more(&self) -> bool {
        match self.request {
            Some(req) => if let Some(val) = req.more {
                val
            } else {
                false
            },
            None => false,
        }
    }

    fn reply_interface_not_found(&mut self, arg: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InterfaceNotFound".into(),
            match arg {
                Some(a) => {
                    let s = format!("{{  \"interface\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    fn reply_parameters(&mut self, parameters: Value) -> io::Result<()> {
        let reply = Reply::parameters(Some(parameters));
        serde_json::to_writer(&mut *self.writer, &reply)?;
        self.writer.write_all(b"\0")?;
        self.writer.flush()?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct GetInterfaceArgs {
    interface: Cow<'static, str>,
}

#[derive(Serialize, Deserialize)]
struct ServiceInfo {
    vendor: Cow<'static, str>,
    product: Cow<'static, str>,
    version: Cow<'static, str>,
    url: Cow<'static, str>,
    interfaces: Vec<Cow<'static, str>>,
}

/// VarlinkService handles all the I/O and dispatches method calls to the registered interfaces.
pub struct VarlinkService {
    info: ServiceInfo,
    ifaces: HashMap<Cow<'static, str>, Box<Interface + Send + Sync>>,
}

impl Interface for VarlinkService {
    fn get_description(&self) -> &'static str {
        r#"
# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

# Get a list of all the interfaces a service provides and information
# about the implementation.
method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: string[]
)

# Get the description of an interface that is implemented by this service.
method GetInterfaceDescription(interface: string) -> (description: string)

# The requested interface was not found.
error InterfaceNotFound (interface: string)

# The requested method was not found
error MethodNotFound (method: string)

# The interface defines the requested method, but the service does not
# implement it.
error MethodNotImplemented (method: string)

# One of the passed parameters is invalid.
error InvalidParameter (parameter: string)
	"#
    }

    fn get_name(&self) -> &'static str {
        "org.varlink.service"
    }

    fn call(&self, call: &mut Call) -> io::Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "org.varlink.service.GetInfo" => {
                return call.reply_parameters(serde_json::to_value(&self.info)?);
            }
            "org.varlink.service.GetInterfaceDescription" => {
                if req.parameters == None {
                    return call.reply_invalid_parameter(None);
                }
                if let Some(val) = req.parameters.clone() {
                    let args: GetInterfaceArgs = serde_json::from_value(val)?;
                    match args.interface.as_ref() {
                        "org.varlink.service" => {
                            return call.reply_parameters(
                                json!({"description": self.get_description()}),
                            );
                        }
                        key => {
                            if self.ifaces.contains_key(key) {
                                return call.reply_parameters(
                                    json!({"description": self.ifaces[key].get_description()}),
                                );
                            } else {
                                return call.reply_invalid_parameter(Some("interface".into()));
                            }
                        }
                    }
                } else {
                    return call.reply_invalid_parameter(Some("interface".into()));
                }
            }
            _ => {
                let method: String = req.method.clone().into();
                let n: usize = match method.rfind('.') {
                    None => 0,
                    Some(x) => x + 1,
                };
                let m = String::from(&method[n..]);
                return call.reply_method_not_found(Some(m.into()));
            }
        }
    }
    fn call_upgraded(&self, call: &mut Call) -> io::Result<()> {
        call.upgraded = false;
        Ok(())
    }
}

impl VarlinkService {
    /// Create a new `VarlinkService`.
    ///
    /// See the [Service](https://github.com/varlink/documentation/wiki/Service) section of the
    /// varlink wiki about the `vendor`, `product`, `version` and `url`.
    ///
    /// The `interfaces` vector is an array of varlink `Interfaces` this service provides.
    ///
    ///# Examples
    ///
    ///```rust,no_run
    ///# use varlink;
    ///# use std::io;
    ///# struct Interface;
    ///# impl varlink::Interface for Interface {
    ///# fn get_description(&self) -> &'static str { "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
    ///# fn get_name(&self) -> &'static str { "org.example.ping" }
    ///# fn call_upgraded(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
    ///# fn call(&self, call: &mut varlink::Call) -> io::Result<()> { Ok(()) }
    ///# }
    ///# let interface_foo = Interface;
    ///# let interface_bar = Interface;
    ///# let interface_baz = Interface;
    ///let service = varlink::VarlinkService::new(
    ///    "org.varlink",
    ///    "test service",
    ///    "0.1",
    ///    "http://varlink.org",
    ///    vec![
    ///        Box::new(interface_foo),
    ///        Box::new(interface_bar),
    ///        Box::new(interface_baz),
    ///    ],
    ///);
    ///```
    pub fn new(
        vendor: &str,
        product: &str,
        version: &str,
        url: &str,
        interfaces: Vec<Box<Interface + Send + Sync>>,
    ) -> Self {
        let mut ifhashmap = HashMap::<Cow<'static, str>, Box<Interface + Send + Sync>>::new();
        for i in interfaces {
            ifhashmap.insert(i.get_name().into(), i);
        }
        let mut ifnames: Vec<Cow<'static, str>> = Vec::new();
        ifnames.push("org.varlink.service".into());
        ifnames.extend(
            ifhashmap
                .keys()
                .map(|i| Cow::<'static, str>::from(i.clone())),
        );
        VarlinkService {
            info: ServiceInfo {
                vendor: String::from(vendor).into(),
                product: String::from(product).into(),
                version: String::from(version).into(),
                url: String::from(url).into(),
                interfaces: ifnames,
            },
            ifaces: ifhashmap,
        }
    }

    fn call(&self, iface: String, call: &mut Call) -> io::Result<()> {
        match iface.as_ref() {
            "org.varlink.service" => return self::Interface::call(self, call),
            key => {
                if self.ifaces.contains_key(key) {
                    return self.ifaces[key].call(call);
                } else {
                    return call.reply_interface_not_found(Some(iface.clone().into()));
                }
            }
        }
    }

    fn call_upgraded(&self, iface: String, call: &mut Call) -> io::Result<()> {
        match iface.as_ref() {
            "org.varlink.service" => return self::Interface::call_upgraded(self, call),
            key => {
                if self.ifaces.contains_key(key) {
                    return self.ifaces[key].call_upgraded(call);
                } else {
                    return call.reply_interface_not_found(Some(iface.clone().into()));
                }
            }
        }
    }

    /// Handles incoming varlink messages from `reader` and sends the reply on `writer`.
    ///
    /// This method can be used to implement your own server.
    pub fn handle(&self, reader: &mut Read, writer: &mut Write) -> io::Result<()> {
        let mut bufreader = BufReader::new(reader);
        let mut upgraded = false;
        let mut last_iface = String::from("");
        loop {
            if !upgraded {
                let mut buf = Vec::new();
                let read_bytes = bufreader.read_until(b'\0', &mut buf)?;
                if read_bytes > 0 {
                    buf.pop();
                    let req: Request = serde_json::from_slice(&buf)?;
                    let mut call = Call::new(writer, &req);

                    let n: usize = match req.method.rfind('.') {
                        None => {
                            let method = req.method.clone();
                            return call.reply_interface_not_found(Some(method.into()));
                        }
                        Some(x) => x,
                    };

                    let iface = String::from(&req.method[..n]);

                    self.call(iface.clone(), &mut call)?;
                    upgraded = call.upgraded;
                    if upgraded {
                        last_iface = iface;
                    }
                } else {
                    break;
                }
            } else {
                let mut call = Call::new_upgraded(writer);
                self.call_upgraded(last_iface.clone(), &mut call)?;
                upgraded = call.upgraded;
            }
        }
        Ok(())
    }
}

/// `listen` creates a server, with `num_worker` threads listening on `varlink_uri`.
///
/// If an `accept_timeout` != 0 is specified, this function returns after the specified
/// amount of seconds, if no new connection is made in that time frame. It still waits for
/// all pending connections to finish.
///
///# Examples
///
///```
///let service = varlink::VarlinkService::new(
///    "org.varlink",
///    "test service",
///    "0.1",
///    "http://varlink.org",
///    vec![/* Your varlink interfaces go here */],
///);
///
///if let Err(e) = varlink::listen(service, "unix:/tmp/test_listen_timeout", 10, 1) {
///    panic!("Error listen: {}", e);
///}
///```
///# Note
/// You don't have to use this simple server. With the `VarlinkService::handle()` method you
/// can implement your own server model using whatever framework you prefer.
pub fn listen(
    service: VarlinkService,
    varlink_uri: &str,
    num_worker: usize,
    accept_timeout: u64,
) -> io::Result<()> {
    match server::do_listen(service, varlink_uri, num_worker, accept_timeout) {
        Err(e) => match e {
            server::ServerError::IoError(e) => Err(e),
            server::ServerError::AcceptTimeout => Ok(()),
        },
        Ok(_) => Ok(()),
    }
}

#[test]
fn test_listen() {
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![/* Your varlink interfaces go here */],
    );

    if let Err(e) = listen(service, "unix:/tmp/test_listen_timeout", 10, 1) {
        panic!("Error listen: {}", e);
    }
}
