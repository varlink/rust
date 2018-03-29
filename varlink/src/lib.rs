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
//!    varlink::generator::cargo_build_tosource("src/org.example.ping.varlink",
//!                                             /* rustfmt */ true);
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
//!mod org_example_ping;
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
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
//!#                                                  method Ping(ping: string) -> (pong: string)" }
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
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
//!#                                                  method Ping(ping: string) -> (pong: string)" }
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

use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

pub mod generator;
mod server;
mod client;

/// This trait has to be implemented by any varlink interface implementor.
/// All methods are generated by the varlink-rust-generator, so you don't have to care
/// about them.
pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call_upgraded(&self, call: &mut Call) -> io::Result<()>;
    fn call(&self, call: &mut Call) -> io::Result<()>;
}

/// The structure of a varlink request. Used to serialize json into it.
///
/// There should be no need to use this directly.
///
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Request {
    #[serde(skip_serializing_if = "Option::is_none")] pub more: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub oneshot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub upgrade: Option<bool>,
    pub method: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")] pub parameters: Option<Value>,
}

impl Request {
    pub fn create(method: Cow<'static, str>, parameters: Option<Value>) -> Self {
        Request {
            more: None,
            oneshot: None,
            upgrade: None,
            method: method.into(),
            parameters,
        }
    }
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
#[derive(Serialize, Deserialize, Debug)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")] pub continues: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub upgraded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub error: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub parameters: Option<Value>,
}

impl Reply {
    pub fn parameters(parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
            upgraded: None,
            error: None,
            parameters,
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

/// Call is a struct, which is passed as the first argument to the interface methods
/// in a derived form.
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
///fn test_method_not_implemented(&self,
///                               call: &mut _CallTestMethodNotImplemented) -> io::Result<()> {
///    return call.reply_method_not_implemented(Some("TestMethodNotImplemented".into()));
///}
///# }
/// ```
pub trait CallTrait {
    /// Don't use this directly. Rather use the standard `reply()` method.
    fn reply_struct(&mut self, reply: Reply) -> io::Result<()>;

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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorInterfaceNotFound {
    pub interface: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorInvalidParameter {
    pub parameter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorMethodNotImplemented {
    pub method: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorMethodNotFound {
    pub method: Option<String>,
}

pub enum Error {
    InterfaceNotFound(ErrorInterfaceNotFound),
    InvalidParameter(ErrorInvalidParameter),
    MethodNotFound(ErrorMethodNotFound),
    MethodNotImplemented(ErrorMethodNotImplemented),
    UnknownError(Reply),
    IOError(io::Error),
    JSONError(serde_json::Error),
}

impl From<Reply> for Error {
    fn from(e: Reply) -> Self {
        match e {
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.InterfaceNotFound" =>
            {
                match e {
                    Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => Error::InterfaceNotFound(v),
                        Err(_) => Error::InterfaceNotFound(ErrorInterfaceNotFound {
                            ..Default::default()
                        }),
                    },
                    _ => Error::InterfaceNotFound(ErrorInterfaceNotFound {
                        ..Default::default()
                    }),
                }
            }
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.InvalidParameter" =>
            {
                match e {
                    Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => Error::InvalidParameter(v),
                        Err(_) => Error::InvalidParameter(ErrorInvalidParameter {
                            ..Default::default()
                        }),
                    },
                    _ => Error::InvalidParameter(ErrorInvalidParameter {
                        ..Default::default()
                    }),
                }
            }
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.MethodNotFound" =>
            {
                match e {
                    Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => Error::MethodNotFound(v),
                        Err(_) => Error::MethodNotFound(ErrorMethodNotFound {
                            ..Default::default()
                        }),
                    },
                    _ => Error::MethodNotFound(ErrorMethodNotFound {
                        ..Default::default()
                    }),
                }
            }
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.MethodNotImplemented" =>
            {
                match e {
                    Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => Error::MethodNotImplemented(v),
                        Err(_) => Error::MethodNotImplemented(ErrorMethodNotImplemented {
                            ..Default::default()
                        }),
                    },
                    _ => Error::MethodNotImplemented(ErrorMethodNotImplemented {
                        ..Default::default()
                    }),
                }
            }
            _ => return Error::UnknownError(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        use serde_json::error::Category;
        match e.classify() {
            Category::Io => Error::IOError(e.into()),
            _ => Error::JSONError(e),
        }
    }
}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::InterfaceNotFound(e) => io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "org.varlink.service.InterfaceNotFound: '{}'",
                    serde_json::to_string_pretty(&e).unwrap()
                ),
            ),
            Error::InvalidParameter(e) => io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "org.varlink.service.InvalidParameter: '{}'",
                    serde_json::to_string_pretty(&e).unwrap()
                ),
            ),
            Error::MethodNotFound(e) => io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "org.varlink.service.MethodNotFound: '{}'",
                    serde_json::to_string_pretty(&e).unwrap()
                ),
            ),
            Error::MethodNotImplemented(e) => io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "org.varlink.service.MethodNotImplemented: '{}'",
                    serde_json::to_string_pretty(&e).unwrap()
                ),
            ),
            Error::IOError(e) => e,
            Error::JSONError(e) => e.into(),
            Error::UnknownError(e) => io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "unknown varlink error: {}",
                    serde_json::to_string_pretty(&e).unwrap()
                ),
            ),
        }
    }
}

impl Error {
    pub fn is_error(r: &Reply) -> bool {
        match r.error {
            Some(ref t) => match t.as_ref() {
                "org.varlink.service.InvalidParameter" => true,
                "org.varlink.service.InterfaceNotFound" => true,
                "org.varlink.service.MethodNotFound" => true,
                "org.varlink.service.MethodNotImplemented" => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl<'a> CallTrait for Call<'a> {
    fn reply_struct(&mut self, mut reply: Reply) -> io::Result<()> {
        if self.continues && !self.wants_more() {
            return Err(io::Error::new(
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
            Some(&Request {
                oneshot: Some(true),
                ..
            }) => true,
            _ => false,
        }
    }

    /// True, if this request accepts more than one reply.
    pub fn wants_more(&self) -> bool {
        match self.request {
            Some(&Request {
                more: Some(true), ..
            }) => true,
            _ => false,
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

pub struct Connection {
    reader: Option<BufReader<Box<Read + Send + Sync>>>,
    writer: Option<Box<Write + Send + Sync>>,
    address: String,
    #[allow(dead_code)] // For the stream Drop()
    stream: client::VarlinkStream,
}

impl Connection {
    pub fn new(address: &str) -> io::Result<Arc<RwLock<Self>>> {
        let (mut stream, addr) = client::VarlinkStream::connect(address)?;
        let (r, w) = stream.split()?;
        let bufreader = BufReader::new(r);
        Ok(Arc::new(RwLock::new(Connection {
            reader: Some(bufreader),
            writer: Some(w),
            address: addr,
            stream: stream,
        })))
    }
    pub fn address(&self) -> String {
        return self.address.clone();
    }
}

pub struct MethodCall<MRequest, MReply, MError> {
    connection: Arc<RwLock<Connection>>,
    reader: Option<BufReader<Box<Read + Send + Sync>>>,
    writer: Option<Box<Write + Send + Sync>>,
    continues: bool,
    phantom_request: PhantomData<MRequest>,
    phantom_reply: PhantomData<MReply>,
    phantom_error: PhantomData<MError>,
}

impl<MRequest, MReply, MError> MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: std::convert::From<std::io::Error>
        + std::convert::From<serde_json::Error>
        + std::convert::From<Reply>,
{
    pub fn call(
        connection: Arc<RwLock<Connection>>,
        method: String,
        request: MRequest,
        more: bool,
    ) -> io::Result<Self> {
        let mut s = MethodCall::<MRequest, MReply, MError> {
            connection,
            continues: true,
            reader: None,
            writer: None,
            phantom_request: PhantomData,
            phantom_reply: PhantomData,
            phantom_error: PhantomData,
        };

        {
            let mut conn = s.connection.write().unwrap();
            let mut req = Request::create(method.into(), Some(serde_json::to_value(request)?));

            if conn.reader.is_none() || conn.writer.is_none() {
                return Err(io::Error::new(
                    ErrorKind::Other,
                    "Varlink: connection busy with other method",
                ));
            }

            s.reader = conn.reader.take();

            if more {
                req.more = Some(more);
            }

            let mut w = conn.writer.take().unwrap();

            serde_json::to_writer(&mut *w, &req)?;
            w.write_all(b"\0")?;
            w.flush()?;
            s.writer = Some(w);
        }
        Ok(s)
    }

    pub fn recv(&mut self) -> Result<MReply, MError> {
        if self.reader.is_none() || self.writer.is_none() {
            return Err(MError::from(io::Error::new(
                ErrorKind::Other,
                "Varlink: Iterator called on \
                 old reply",
            )));
        }

        let mut buf = Vec::new();

        let mut reader = self.reader.take().unwrap();
        reader.read_until(0, &mut buf)?;
        self.reader = Some(reader);

        buf.pop();
        let reply: Reply = serde_json::from_slice(&buf)?;
        match reply.continues {
            Some(v) => self.continues = v,
            _ => {
                self.continues = false;
                let mut conn = self.connection.write().unwrap();
                conn.reader = self.reader.take();
                conn.writer = self.writer.take();
            }
        }
        if reply.error != None {
            return Err(MError::from(reply));
        }

        let mreply: MReply = serde_json::from_value(reply.parameters.unwrap())?;
        Ok(mreply)
    }
}

impl<MRequest, MReply, MError> Iterator for MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: std::convert::From<std::io::Error>
        + std::convert::From<serde_json::Error>
        + std::convert::From<Reply>,
{
    type Item = Result<MReply, MError>;
    fn next(&mut self) -> Option<Result<MReply, MError>> {
        if !self.continues {
            return None;
        }

        Some(self.recv())
    }
}

#[derive(Serialize, Deserialize)]
struct GetInterfaceDescriptionArgs {
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
            "org.varlink.service.GetInterfaceDescription" => match req.parameters.as_ref() {
                None => {
                    return call.reply_invalid_parameter(None);
                }
                Some(val) => {
                    let args: GetInterfaceDescriptionArgs = serde_json::from_value(val.clone())?;
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
                }
            },
            _ => {
                let method: String = req.method.clone().into();
                let n: usize = match method.rfind('.') {
                    None => 0,
                    Some(x) => x + 1,
                };
                let m = String::from(&method[n..]);
                return call.reply_method_not_found(Some(m));
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
    ///# fn get_description(&self) -> &'static str {
    ///#                    "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
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
                    return call.reply_interface_not_found(Some(iface.clone()));
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
                    return call.reply_interface_not_found(Some(iface.clone()));
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
            match upgraded {
                false => {
                    let mut buf = Vec::new();
                    if bufreader.read_until(b'\0', &mut buf)? <= 0 {
                        break;
                    }
                    // pop the last zero byte
                    buf.pop();
                    let req: Request = serde_json::from_slice(&buf)?;
                    let mut call = Call::new(writer, &req);

                    let n: usize = match req.method.rfind('.') {
                        None => {
                            return call.reply_interface_not_found(Some(String::from(
                                req.method.as_ref(),
                            )));
                        }
                        Some(x) => x,
                    };

                    let iface = String::from(&req.method[..n]);

                    self.call(iface.clone(), &mut call)?;

                    upgraded = call.upgraded;
                    if upgraded {
                        last_iface = iface;
                    }
                }
                true => {
                    let mut call = Call::new_upgraded(writer);
                    self.call_upgraded(last_iface.clone(), &mut call)?;
                    upgraded = call.upgraded;
                }
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
        Err(server::ServerError::IoError(e)) => Err(e),
        _ => Ok(()),
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
