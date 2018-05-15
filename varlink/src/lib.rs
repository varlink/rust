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
//!```rust,ignore
//!extern crate varlink;
//!
//!fn main() {
//!    varlink::generator::cargo_build_tosource("src/org.example.ping.varlink",
//!                                             /* rustfmt */ true);
//!}
//!```
//!
//!Add to your ```Cargo.toml```:
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
//!```rust
//!# use std::io;
//!# use varlink::{CallTrait, Result};
//!# struct _PingReply {pong: String}
//!# impl varlink::VarlinkReply for _PingReply {}
//!# struct _PingArgs {ping: String}
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallPing: _CallErr {
//!#     fn reply(&mut self, pong: String) -> Result<()> { Ok(()) }
//!# }
//!# impl<'a> _CallPing for varlink::Call<'a> {}
//!# pub trait VarlinkInterface {
//!#     fn ping(&self, call: &mut _CallPing, ping: String) -> Result<()>;
//!#     fn call_upgraded(&self, _call: &mut varlink::Call) -> Result<()> {Ok(())}
//!# }
//!# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
//!# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
//!#     _InterfaceProxy { inner }
//!# }
//!# impl varlink::Interface for _InterfaceProxy {
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
//!#                                                  method Ping(ping: string) -> (pong: string)" }
//!#     fn get_name(&self) -> &'static str { "org.example.ping" }
//!#     fn call_upgraded(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
//!#     fn call(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
//!# }
//!# fn main() {}
//!struct MyOrgExamplePing;
//!
//!impl VarlinkInterface for MyOrgExamplePing {
//!    fn ping(&self, call: &mut _CallPing, ping: String) -> Result<()> {
//!        return call.reply(ping);
//!    }
//!}
//!```
//!to implement the interface methods.
//!
//!If your varlink method is called `TestMethod`, the rust method to be implemented is called
//!`test_method`. The first parameter is of type `_CallTestMethod`, which has the method `reply()`.
//!
//!```rust
//!# use std::io;
//!# use varlink::{CallTrait, Result};
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallTestMethod: _CallErr {
//!#     fn reply(&mut self) -> Result<()> {
//!#         self.reply_struct(varlink::Reply::parameters(None))
//!#     }
//!# }
//!# impl<'a> _CallTestMethod for varlink::Call<'a> {}
//!# struct TestService;
//!# impl TestService {
//!fn test_method(&self, call: &mut _CallTestMethod, /* more arguments */) -> Result<()> {
//!    /* ... */
//!    return call.reply( /* more arguments */ );
//!}
//!# }
//!# fn main() {}
//!```
//!
//!A typical server creates a `VarlinkService` and starts a server via `varlink::listen()`
//!
//!```rust
//!# use std::io;
//!# mod org_example_ping {
//!# use std::io;
//!# use varlink::{self, Result};
//!# struct _PingReply {pong: String}
//!# impl varlink::VarlinkReply for _PingReply {}
//!# struct _PingArgs {ping: String}
//!# pub trait _CallErr: varlink::CallTrait {}
//!# impl<'a> _CallErr for varlink::Call<'a> {}
//!# pub trait _CallPing: _CallErr {
//!#     fn reply(&mut self, pong: String) -> Result<()> { Ok(()) }
//!# }
//!# impl<'a> _CallPing for varlink::Call<'a> {}
//!# pub trait VarlinkInterface {
//!#     fn ping(&self, call: &mut _CallPing, ping: String) -> Result<()>;
//!#     fn call_upgraded(&self, _call: &mut varlink::Call) -> Result<()> {Ok(())}
//!# }
//!# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
//!# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
//!#     _InterfaceProxy { inner }
//!# }
//!# impl varlink::Interface for _InterfaceProxy {
//!#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
//!#                                                  method Ping(ping: string) -> (pong: string)" }
//!#     fn get_name(&self) -> &'static str { "org.example.ping" }
//!#     fn call_upgraded(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
//!#     fn call(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
//!# }}
//!# use org_example_ping::*;
//!#
//!# struct MyOrgExamplePing;
//!#
//!# impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
//!#     fn ping(&self, call: &mut _CallPing, ping: String) -> varlink::Result<()> {
//!#         return call.reply(ping);
//!#     }
//!# }
//!# fn main_func() {
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
//!varlink::listen(service, args[1].clone(), 10, 0);
//!# }
//!# fn main() {}
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
extern crate tempfile;
extern crate unix_socket;
extern crate varlink_parser;

#[macro_use]
extern crate error_chain;

use serde::de;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::From;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

mod client;
pub mod generator;
mod server;
#[cfg(test)]
mod test;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Fmt(::std::fmt::Error);
        SerdeJson(::serde_json::Error);
    }

    errors {
        InterfaceNotFound(t: String) {
            display("Interface not found: '{}'", t)
        }
        InvalidParameter(t: String) {
            display("Invalid parameter: '{}'", t)
        }
        MethodNotFound(t: String) {
            display("Method not found: '{}'", t)
        }
        MethodNotImplemented(t: String) {
            display("Method not implemented: '{}'", t)
        }
        UnknownError(r: Reply) {
            display("Unknown error: '{:?}'", r)
        }
        CallContinuesMismatch {
            display("Call::reply() called with continues, but without more in the request")
        }
        MethodCalledAlready {
            display("Varlink: method called already")
        }
        ConnectionBusy {
            display("Varlink: connection busy with other method")
        }
        IteratorOldReply {
            display("Varlink: Iterator called on old reply")
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ErrorInterfaceNotFound {
    pub interface: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ErrorInvalidParameter {
    pub parameter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ErrorMethodNotImplemented {
    pub method: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ErrorMethodNotFound {
    pub method: Option<String>,
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
                    } => match serde_json::from_value::<ErrorInterfaceNotFound>(p) {
                        Ok(v) => ErrorKind::InterfaceNotFound(
                            v.interface.unwrap_or("".to_string()),
                        ).into(),
                        Err(_) => ErrorKind::InterfaceNotFound("".to_string()).into(),
                    },
                    _ => ErrorKind::InterfaceNotFound("".to_string()).into(),
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
                    } => match serde_json::from_value::<ErrorInvalidParameter>(p) {
                        Ok(v) => ErrorKind::InvalidParameter(v.parameter.unwrap_or("".to_string()))
                            .into(),
                        Err(_) => ErrorKind::InvalidParameter("".to_string()).into(),
                    },
                    _ => ErrorKind::InvalidParameter("".to_string()).into(),
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
                    } => match serde_json::from_value::<ErrorMethodNotFound>(p) {
                        Ok(v) => {
                            ErrorKind::MethodNotFound(v.method.unwrap_or("".to_string())).into()
                        }
                        Err(_) => ErrorKind::MethodNotFound("".to_string()).into(),
                    },
                    _ => ErrorKind::MethodNotFound("".to_string()).into(),
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
                    } => match serde_json::from_value::<ErrorMethodNotImplemented>(p) {
                        Ok(v) => ErrorKind::MethodNotImplemented(
                            v.method.unwrap_or("".to_string()),
                        ).into(),
                        Err(_) => ErrorKind::MethodNotImplemented("".to_string()).into(),
                    },
                    _ => ErrorKind::MethodNotImplemented("".to_string()).into(),
                }
            }
            _ => return ErrorKind::UnknownError(e).into(),
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

/// This trait has to be implemented by any varlink interface implementor.
/// All methods are generated by the varlink-rust-generator, so you don't have to care
/// about them.
pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call_upgraded(&self, call: &mut Call) -> Result<()>;
    fn call(&self, call: &mut Call) -> Result<()>;
}

/// The structure of a varlink request. Used to serialize json into it.
///
/// There should be no need to use this directly.
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Request<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneway: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<bool>,
    pub method: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl<'a> Request<'a> {
    pub fn create<S: Into<Cow<'a, str>>>(method: S, parameters: Option<Value>) -> Self {
        Request {
            more: None,
            oneway: None,
            upgrade: None,
            method: method.into(),
            parameters,
        }
    }
}

pub type StringHashMap<T> = HashMap<String, T>;

#[derive(Debug, PartialEq, Default)]
pub struct StringHashSet {
    inner: HashSet<String>,
}

impl StringHashSet {
    pub fn new() -> StringHashSet {
        StringHashSet {
            inner: HashSet::new(),
        }
    }
}

impl Deref for StringHashSet {
    type Target = HashSet<String>;

    fn deref(&self) -> &HashSet<String> {
        &self.inner
    }
}

impl DerefMut for StringHashSet {
    fn deref_mut(&mut self) -> &mut HashSet<String> {
        &mut self.inner
    }
}

impl Serialize for StringHashSet {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let null_obj: serde_json::Value = serde_json::Value::Object(serde_json::Map::new());

        let mut map = serializer.serialize_map(Some(self.inner.len()))?;
        for k in &self.inner {
            map.serialize_entry(k, &null_obj)?;
        }
        map.end()
    }
}

impl<'de> de::Deserialize<'de> for StringHashSet {
    #[inline]
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StringHashSet;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            #[inline]
            fn visit_unit<E>(self) -> ::std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StringHashSet::new())
            }

            #[inline]
            fn visit_map<V>(self, mut visitor: V) -> ::std::result::Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut values = StringHashSet::new();

                while let Some(key) = visitor.next_key()? {
                    values.insert(key);
                }

                Ok(values)
            }
        }

        deserializer.deserialize_map(Visitor)
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continues: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgraded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
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

    pub fn error<S: Into<Cow<'static, str>>>(name: S, parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
            upgraded: None,
            error: Some(name.into()),
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
///```rust
///# use std::io;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethod: _CallErr {
///#     fn reply(&mut self) -> varlink::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method(&self, call: &mut _CallTestMethod, /* more arguments */) -> varlink::Result<()> {
///    /* ... */
///    return call.reply( /* more arguments */ );
///}
///# }
///# fn main() {}
///```
pub struct Call<'a> {
    writer: &'a mut Write,
    pub request: Option<&'a Request<'a>>,
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
/// ```rust
///# use std::io;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethod: _CallErr {
///#     fn reply(&mut self) -> varlink::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method(&self, call: &mut _CallTestMethod, testparam: i64) -> varlink::Result<()> {
///    match testparam {
///        0 ... 100 => {},
///        _ => {
///            return call.reply_invalid_parameter("testparam".into());
///        }
///    }
///    /* ... */
///    Ok(())
///}
///# }
///# fn main() {}
/// ```
///
/// For not yet implemented methods:
///
/// ```rust
///# use std::io;
///# pub trait _CallErr: varlink::CallTrait {}
///# impl<'a> _CallErr for varlink::Call<'a> {}
///# pub trait _CallTestMethodNotImplemented: _CallErr {
///#     fn reply(&mut self) -> varlink::Result<()> {
///#         self.reply_struct(varlink::Reply::parameters(None))
///#     }
///# }
///# impl<'a> _CallTestMethodNotImplemented for varlink::Call<'a> {}
///# struct TestService;
///# impl TestService {
///fn test_method_not_implemented(&self,
///                               call: &mut _CallTestMethodNotImplemented) -> varlink::Result<()> {
///    return call.reply_method_not_implemented("TestMethodNotImplemented".into());
///}
///# }
///# fn main() {}
/// ```
pub trait CallTrait {
    /// Don't use this directly. Rather use the standard `reply()` method.
    fn reply_struct(&mut self, reply: Reply) -> Result<()>;

    /// Set this to `true` to indicate, that more replies are following.
    ///
    ///# Examples
    ///
    ///```rust
    ///# use std::io;
    ///# pub trait _CallErr: varlink::CallTrait {}
    ///# impl<'a> _CallErr for varlink::Call<'a> {}
    ///# pub trait _CallTestMethod: _CallErr {
    ///#     fn reply(&mut self) -> varlink::Result<()> {
    ///#         self.reply_struct(varlink::Reply::parameters(None))
    ///#     }
    ///# }
    ///# impl<'a> _CallTestMethod for varlink::Call<'a> {}
    ///# struct TestService;
    ///# impl TestService {
    ///fn test_method(&self, call: &mut _CallTestMethod) -> varlink::Result<()> {
    ///    call.set_continues(true);
    ///    call.reply( /* more args*/ )?;
    ///    call.reply( /* more args*/ )?;
    ///    call.reply( /* more args*/ )?;
    ///    call.set_continues(false);
    ///    return call.reply( /* more args*/ );
    ///}
    ///# }
    ///# fn main() {}
    ///```
    fn set_continues(&mut self, cont: bool);

    /// True, if this request does not want a reply.
    fn is_oneway(&self) -> bool;

    /// True, if this request accepts more than one reply.
    fn wants_more(&self) -> bool;

    fn get_request(&self) -> Option<&Request>;

    /// reply with the standard varlink `org.varlink.service.MethodNotFound` error
    fn reply_method_not_found(&mut self, method_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotFound",
            Some(serde_json::to_value(ErrorMethodNotFound {
                method: Some(method_name),
            })?),
        ))
    }

    /// reply with the standard varlink `org.varlink.service.MethodNotImplemented` error
    fn reply_method_not_implemented(&mut self, method_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotImplemented",
            Some(serde_json::to_value(ErrorMethodNotImplemented {
                method: Some(method_name),
            })?),
        ))
    }

    /// reply with the standard varlink `org.varlink.service.InvalidParameter` error
    fn reply_invalid_parameter(&mut self, parameter_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InvalidParameter",
            Some(serde_json::to_value(ErrorInvalidParameter {
                parameter: Some(parameter_name),
            })?),
        ))
    }
}

impl<'a> CallTrait for Call<'a> {
    fn reply_struct(&mut self, mut reply: Reply) -> Result<()> {
        if self.continues & &!self.wants_more() {
            return Err(ErrorKind::CallContinuesMismatch.into());
        }
        if self.continues {
            reply.continues = Some(true);
        }
        //serde_json::to_writer(&mut *self.writer, &reply)?;
        let b = serde_json::to_string(&reply)? + "\0";

        self.writer.write_all(b.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
    fn set_continues(&mut self, cont: bool) {
        self.continues = cont;
    }

    /// True, if this request does not want a reply.
    fn is_oneway(&self) -> bool {
        match self.request {
            Some(Request {
                oneway: Some(true), ..
            }) => true,
            _ => false,
        }
    }

    /// True, if this request accepts more than one reply.
    fn wants_more(&self) -> bool {
        match self.request {
            Some(Request {
                more: Some(true), ..
            }) => true,
            _ => false,
        }
    }
    fn get_request(&self) -> Option<&Request> {
        self.request
    }
}

impl<'a> Call<'a> {
    fn new(writer: &'a mut Write, request: &'a Request<'a>) -> Self {
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

    fn reply_interface_not_found(&mut self, arg: Option<String>) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InterfaceNotFound",
            match arg {
                Some(a) => Some(serde_json::to_value(ErrorInterfaceNotFound {
                    interface: Some(a),
                })?),
                None => None,
            },
        ))
    }

    fn reply_parameters(&mut self, parameters: Value) -> Result<()> {
        let reply = Reply::parameters(Some(parameters));
        //serde_json::to_writer(&mut *self.writer, &reply)?;
        let b = serde_json::to_string(&reply)? + "\0";

        self.writer.write_all(b.as_bytes())?;
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
    pub fn new<S: Into<String>>(address: S) -> io::Result<Arc<RwLock<Self>>> {
        let (mut stream, address) = client::VarlinkStream::connect(address)?;
        let (r, w) = stream.split()?;
        let bufreader = BufReader::new(r);
        Ok(Arc::new(RwLock::new(Connection {
            reader: Some(bufreader),
            writer: Some(w),
            address,
            stream,
        })))
    }
    pub fn address(&self) -> String {
        return self.address.clone();
    }
}

pub struct MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: std::convert::From<Error>
        + std::convert::From<std::io::Error>
        + std::convert::From<serde_json::Error>
        + std::convert::From<Reply>,
{
    connection: Arc<RwLock<Connection>>,
    request: Option<MRequest>,
    method: Option<Cow<'static, str>>,
    reader: Option<BufReader<Box<Read + Send + Sync>>>,
    writer: Option<Box<Write + Send + Sync>>,
    continues: bool,
    phantom_reply: PhantomData<MReply>,
    phantom_error: PhantomData<MError>,
}

impl<MRequest, MReply, MError> MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: std::convert::From<Error>
        + std::convert::From<std::io::Error>
        + std::convert::From<serde_json::Error>
        + std::convert::From<Reply>,
{
    pub fn new<S: Into<Cow<'static, str>>>(
        connection: Arc<RwLock<Connection>>,
        method: S,
        request: MRequest,
    ) -> Self {
        MethodCall::<MRequest, MReply, MError> {
            connection,
            request: Some(request),
            method: Some(method.into()),
            continues: false,
            reader: None,
            writer: None,
            phantom_reply: PhantomData,
            phantom_error: PhantomData,
        }
    }

    fn send(&mut self, oneway: bool, more: bool) -> ::std::result::Result<(), MError> {
        {
            let mut conn = self.connection.write().unwrap();
            let mut req = match (self.method.take(), self.request.take()) {
                (Some(method), Some(request)) => {
                    Request::create(method, Some(serde_json::to_value(request)?))
                }
                _ => {
                    return Err(Error::from(ErrorKind::MethodCalledAlready).into());
                }
            };

            if conn.reader.is_none() || conn.writer.is_none() {
                return Err(Error::from(ErrorKind::ConnectionBusy).into());
            }

            if oneway {
                req.oneway = Some(true);
            } else {
                self.reader = conn.reader.take();
            }

            if more {
                req.more = Some(true);
            }

            let mut w = conn.writer.take().unwrap();

            let b = serde_json::to_string(&req)? + "\0";

            w.write_all(b.as_bytes())?;
            w.flush()?;
            if oneway {
                conn.writer = Some(w);
            } else {
                self.writer = Some(w);
            }
        }
        Ok(())
    }

    pub fn call(&mut self) -> ::std::result::Result<MReply, MError> {
        self.send(false, false)?;
        self.recv()
    }

    pub fn oneway(&mut self) -> ::std::result::Result<(), MError> {
        self.send(true, false)
    }

    pub fn more(&mut self) -> ::std::result::Result<&mut Self, MError> {
        self.continues = true;
        self.send(false, true)?;
        Ok(self)
    }

    pub fn recv(&mut self) -> ::std::result::Result<MReply, MError> {
        if self.reader.is_none() || self.writer.is_none() {
            return Err(Error::from(ErrorKind::IteratorOldReply).into());
        }

        let mut buf = Vec::new();

        let mut reader = self.reader.take().unwrap();
        reader.read_until(0, &mut buf)?;
        self.reader = Some(reader);

        buf.pop();
        let reply: Reply = serde_json::from_slice(&buf)?;
        match reply.continues {
            Some(true) => self.continues = true,
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

        match reply {
            Reply {
                parameters: Some(p),
                ..
            } => {
                let mreply: MReply = serde_json::from_value(p)?;
                Ok(mreply)
            }
            Reply {
                parameters: None, ..
            } => {
                let mreply: MReply =
                    serde_json::from_value(serde_json::Value::Object(serde_json::Map::new()))?;
                Ok(mreply)
            }
        }
    }
}

impl<MRequest, MReply, MError> Iterator for MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: std::convert::From<Error>
        + std::convert::From<std::io::Error>
        + std::convert::From<serde_json::Error>
        + std::convert::From<Reply>,
{
    type Item = ::std::result::Result<MReply, MError>;
    fn next(&mut self) -> Option<::std::result::Result<MReply, MError>> {
        if !self.continues {
            return None;
        }

        Some(self.recv())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct GetInterfaceDescriptionArgs<'a> {
    interface: Cow<'a, str>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct ServiceInfo {
    pub vendor: Cow<'static, str>,
    pub product: Cow<'static, str>,
    pub version: Cow<'static, str>,
    pub url: Cow<'static, str>,
    pub interfaces: Vec<Cow<'static, str>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct GetInfoArgs;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct GetInterfaceDescriptionReply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl VarlinkReply for GetInterfaceDescriptionReply {}

pub struct OrgVarlinkServiceClient {
    connection: Arc<RwLock<Connection>>,
}

impl OrgVarlinkServiceClient {
    pub fn new(connection: Arc<RwLock<Connection>>) -> Self {
        OrgVarlinkServiceClient { connection }
    }
}

pub trait OrgVarlinkServiceInterface {
    fn get_info(&mut self) -> Result<ServiceInfo>;
    fn get_interface_description<S: Into<Cow<'static, str>>>(
        &mut self,
        interface: S,
    ) -> Result<GetInterfaceDescriptionReply>;
}

impl OrgVarlinkServiceInterface for OrgVarlinkServiceClient {
    fn get_info(&mut self) -> Result<ServiceInfo> {
        MethodCall::<GetInfoArgs, ServiceInfo, Error>::new(
            self.connection.clone(),
            "org.varlink.service.GetInfo",
            GetInfoArgs {},
        ).call()
    }
    fn get_interface_description<S: Into<Cow<'static, str>>>(
        &mut self,
        interface: S,
    ) -> Result<GetInterfaceDescriptionReply> {
        MethodCall::<GetInterfaceDescriptionArgs, GetInterfaceDescriptionReply, Error>::new(
            self.connection.clone(),
            "org.varlink.service.GetInterfaceDescription",
            GetInterfaceDescriptionArgs {
                interface: interface.into(),
            },
        ).call()
    }
}

/// VarlinkService handles all the I/O and dispatches method calls to the registered interfaces.
pub struct VarlinkService {
    info: ServiceInfo,
    ifaces: HashMap<Cow<'static, str>, Box<Interface + Send + Sync>>,
}

impl Interface for VarlinkService {
    fn get_description(&self) -> &'static str {
        r#"# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

# Get a list of all the interfaces a service provides and information
# about the implementation.
method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: []string
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

    fn call_upgraded(&self, call: &mut Call) -> Result<()> {
        call.upgraded = false;
        Ok(())
    }

    fn call(&self, call: &mut Call) -> Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "org.varlink.service.GetInfo" => {
                return call.reply_parameters(serde_json::to_value(&self.info)?);
            }
            "org.varlink.service.GetInterfaceDescription" => match req.parameters.as_ref() {
                None => {
                    return call.reply_invalid_parameter("parameters".into());
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
                                return call.reply_invalid_parameter("interface".into());
                            }
                        }
                    }
                }
            },
            m => {
                return call.reply_method_not_found(m.to_string());
            }
        }
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
    ///```rust
    ///# use std::io;
    ///# struct Interface;
    ///# impl varlink::Interface for Interface {
    ///# fn get_description(&self) -> &'static str {
    ///#                    "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
    ///# fn get_name(&self) -> &'static str { "org.example.ping" }
    ///# fn call_upgraded(&self, call: &mut varlink::Call) -> varlink::Result<()> { Ok(()) }
    ///# fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> { Ok(()) }
    ///# }
    ///# fn main_f() {
    ///# let interface_foo = Interface{};
    ///# let interface_bar = Interface{};
    ///# let interface_baz = Interface{};
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
    ///# }
    ///# fn main() {}
    ///```
    pub fn new<S: Into<Cow<'static, str>>>(
        vendor: S,
        product: S,
        version: S,
        url: S,
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
                vendor: vendor.into(),
                product: product.into(),
                version: version.into(),
                url: url.into(),
                interfaces: ifnames,
            },
            ifaces: ifhashmap,
        }
    }

    fn call(&self, iface: String, call: &mut Call) -> Result<()> {
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

    fn call_upgraded(&self, iface: String, call: &mut Call) -> Result<()> {
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
    pub fn handle(&self, reader: &mut Read, writer: &mut Write) -> Result<()> {
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

                    let n: usize = match req.method.rfind('.') {
                        None => {
                            let method: String = String::from(req.method.as_ref());
                            let mut call = Call::new(writer, &req);
                            return call.reply_interface_not_found(Some(method));
                        }
                        Some(x) => x,
                    };

                    let iface = String::from(&req.method[..n]);

                    let mut call = Call::new(writer, &req);
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
pub fn listen<S: Into<String>>(
    service: VarlinkService,
    varlink_uri: S,
    num_worker: usize,
    accept_timeout: u64,
) -> Result<()> {
    match server::do_listen(service, varlink_uri, num_worker, accept_timeout) {
        Err(server::ServerError::IoError(e)) => Err(e.into()),
        _ => Ok(()),
    }
}
