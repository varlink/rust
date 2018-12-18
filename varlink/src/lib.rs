/*!
[Server](#server) and [client](#client) support for the [varlink protocol](http://varlink.org)

 # Server

To create a varlink server in rust, place your varlink interface definition file in src/.
E.g. `src/org.example.ping.varlink`:

```varlink
# Example service
interface org.example.ping

# Returns the same string
method Ping(ping: string) -> (pong: string)
```

Then create a `build.rs` file in your project directory:

```rust,ignore
extern crate varlink_generator;

fn main() {
    varlink_generator::cargo_build_tosource("src/org.example.ping.varlink",
                                             /* rustfmt */ true);
}
```

 For more code generation functions see the [`generator functions`].

Add to your ```Cargo.toml```:

```toml
[package]
build = "build.rs"
[build-dependencies]
varlink_generator = "6"
```

In your `main.rs` you can then use:

```rust,ignore
mod org_example_ping;
```
and then implement the interface:

```rust
# #![allow(non_camel_case_types)]
# #![allow(non_snake_case)]
# use std::io;
# use varlink::{CallTrait, Result};
# struct Ping_Reply {pong: String}
# impl varlink::VarlinkReply for Ping_Reply {}
# struct _PingArgs {ping: String}
# pub trait VarlinkCallError: varlink::CallTrait {}
# impl<'a> VarlinkCallError for varlink::Call<'a> {}
# pub trait Call_Ping: VarlinkCallError {
#     fn reply(&mut self, pong: String) -> Result<()> { Ok(()) }
# }
# impl<'a> Call_Ping for varlink::Call<'a> {}
# pub trait VarlinkInterface {
#     fn ping(&self, call: &mut Call_Ping, ping: String) -> Result<()>;
#     fn call_upgraded(&self, _call: &mut varlink::Call, bufreader: &mut io::BufRead) ->
# Result<Vec<u8>> {Ok(Vec::new())}
# }
# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
#     _InterfaceProxy { inner }
# }
# impl varlink::Interface for _InterfaceProxy {
#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
#                                                  method Ping(ping: string) -> (pong: string)" }
#     fn get_name(&self) -> &'static str { "org.example.ping" }
#     fn call_upgraded(&self, call: &mut varlink::Call, _bufreader: &mut io::BufRead) ->
# Result<Vec<u8>> { Ok(Vec::new()) }
#     fn call(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
# }
# fn main() {}
struct MyOrgExamplePing;

impl VarlinkInterface for MyOrgExamplePing {
    fn ping(&self, call: &mut Call_Ping, ping: String) -> Result<()> {
        return call.reply(ping);
    }
}
```
to implement the interface methods.

If your varlink method is called `TestMethod`, the rust method to be implemented is called
`test_method`. The first parameter is of type `Call_TestMethod`, which has the method `reply()`.

```rust
# #![allow(non_camel_case_types)]
# #![allow(non_snake_case)]
# use std::io;
# use varlink::{CallTrait, Result};
# pub trait VarlinkCallError: varlink::CallTrait {}
# impl<'a> VarlinkCallError for varlink::Call<'a> {}
# pub trait Call_TestMethod: VarlinkCallError {
#     fn reply(&mut self) -> Result<()> {
#         self.reply_struct(varlink::Reply::parameters(None))
#     }
# }
# impl<'a> Call_TestMethod for varlink::Call<'a> {}
# struct TestService;
# impl TestService {
fn test_method(&self, call: &mut Call_TestMethod, /* more arguments */) -> Result<()> {
    /* ... */
return call.reply( /* more arguments */ );
}
# }
# fn main() {}
```

A typical server creates a `VarlinkService` and starts a server via [`varlink::listen`]

```rust
# #![allow(non_camel_case_types)]
# #![allow(non_snake_case)]
# use std::io;
# mod org_example_ping {
# use std::io;
# use varlink::{self, Result};
# struct Ping_Reply {pong: String}
# impl varlink::VarlinkReply for Ping_Reply {}
# struct _PingArgs {ping: String}
# pub trait VarlinkCallError: varlink::CallTrait {}
# impl<'a> VarlinkCallError for varlink::Call<'a> {}
# pub trait Call_Ping: VarlinkCallError {
#     fn reply(&mut self, pong: String) -> Result<()> { Ok(()) }
# }
# impl<'a> Call_Ping for varlink::Call<'a> {}
# pub trait VarlinkInterface {
#     fn ping(&self, call: &mut Call_Ping, ping: String) -> Result<()>;
#     fn call_upgraded(&self, _call: &mut varlink::Call, bufreader: &mut io::BufRead) ->
# Result<Vec<u8>> {Ok(Vec::new())}
# }
# pub struct _InterfaceProxy {inner: Box<VarlinkInterface + Send + Sync>}
# pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {
#     _InterfaceProxy { inner }
# }
# impl varlink::Interface for _InterfaceProxy {
#     fn get_description(&self) -> &'static str { "interface org.example.ping\n\
#                                                  method Ping(ping: string) -> (pong: string)" }
#     fn get_name(&self) -> &'static str { "org.example.ping" }
#     fn call_upgraded(&self, call: &mut varlink::Call, _bufreader: &mut io::BufRead) ->
# Result<Vec<u8>> { Ok(Vec::new()) }
#     fn call(&self, call: &mut varlink::Call) -> Result<()> { Ok(()) }
# }}
# use crate::org_example_ping::*;
#
# struct MyOrgExamplePing;
#
# impl org_example_ping::VarlinkInterface for MyOrgExamplePing {
#     fn ping(&self, call: &mut Call_Ping, ping: String) -> varlink::Result<()> {
#         return call.reply(ping);
#     }
# }
# fn main_func() {
let args: Vec<_> = std::env::args().collect();
let myorgexampleping = MyOrgExamplePing;
let myorgexampleping_interface = org_example_ping::new(Box::new(myorgexampleping));

let service = varlink::VarlinkService::new(
"org.varlink",
"test service",
"0.1",
"http://varlink.org",
vec![
Box::new(myorgexampleping_interface),
], // more interfaces ...
);

varlink::listen(service, &args[1], 1, 10, 0);
# }
# fn main() {}
```

where args[1] would follow the varlink
[address specification](https://github.com/varlink/documentation/wiki#address).

Currently supported address URIs are:

- TCP `tcp:127.0.0.1:12345` hostname/IP address and port
- UNIX socket `unix:/run/org.example.ftl` optional access `;mode=0666` parameter
- UNIX abstract namespace socket `unix:@org.example.ftl` (on Linux only)

# Client

Setup your project, just like in the [server](#server) case with a varlink file
and a ```build.rs``` file.

In your `main.rs` you can then use:

```rust,ignore
mod org_example_ping;
use org_example_ping;
let connection = Connection::with_address("unix:/tmp/org.example.ping").unwrap();
let mut ping_service = org_example_ping::VarlinkClient::new(connection);
let reply = ping_service.ping(String::from("Test")).call()?;
assert_eq!(String::from("Test"), reply.pong);
```

A connection can be established via the [`connection builder`] functions.
The ```org_example_ping::VarlinkClient``` implements ```org_example_ping::VarlinkClientInterface```,
which has all the varlink methods (names converted from camel case to lowercase snake case).
The ```PingString()``` method would be named ```ping_string()```.

To iterate over a ```more``` call
```rust,ignore
for reply in my_more_service.test_more(/* params */).more()? { /*...*/ }
```

The reply struct is placed in a structure named after the method with ```_Reply``` appended.
So, the reply to the ```Ping()``` method in our example is in a struct called ```Ping_Reply```.

[`connection builder`]: struct.Connection.html#methods
[`varlink::listen`]: fn.listen.html
[`generator functions`]: https://docs.rs/varlink_generator

!*/

#![doc(
    html_logo_url = "https://varlink.org/images/varlink.png",
    html_favicon_url = "https://varlink.org/images/varlink-small.png"
)]

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::From;
use std::io::{BufRead, BufReader, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::process::Child;
use std::sync::{Arc, RwLock};

use chainerror::*;
use serde::de::{self, DeserializeOwned};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use tempfile::TempDir;

pub use crate::client::VarlinkStream;
use crate::client::{varlink_bridge, varlink_exec};
pub use crate::error::{Error, ErrorKind, Result};
pub use crate::server::Stream as ServerStream;
pub use crate::server::{listen, Listener};

mod client;

mod error;
mod server;
#[cfg(test)]
mod test;

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

impl From<Reply> for ErrorKind {
    fn from(e: Reply) -> Self {
        match e {
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.InterfaceNotFound" => match e {
                Reply {
                    parameters: Some(p),
                    ..
                } => match serde_json::from_value::<ErrorInterfaceNotFound>(p) {
                    Ok(v) => ErrorKind::InterfaceNotFound(v.interface.unwrap_or_default()),
                    Err(_) => ErrorKind::InterfaceNotFound(String::new()),
                },
                _ => ErrorKind::InterfaceNotFound(String::new()),
            },
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.InvalidParameter" => match e {
                Reply {
                    parameters: Some(p),
                    ..
                } => match serde_json::from_value::<ErrorInvalidParameter>(p) {
                    Ok(v) => ErrorKind::InvalidParameter(v.parameter.unwrap_or_default()),
                    Err(_) => ErrorKind::InvalidParameter(String::new()),
                },
                _ => ErrorKind::InvalidParameter(String::new()),
            },
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.MethodNotFound" => match e {
                Reply {
                    parameters: Some(p),
                    ..
                } => match serde_json::from_value::<ErrorMethodNotFound>(p) {
                    Ok(v) => ErrorKind::MethodNotFound(v.method.unwrap_or_default()),
                    Err(_) => ErrorKind::MethodNotFound(String::new()),
                },
                _ => ErrorKind::MethodNotFound(String::new()),
            },
            Reply {
                error: Some(ref t), ..
            } if t == "org.varlink.service.MethodNotImplemented" => match e {
                Reply {
                    parameters: Some(p),
                    ..
                } => match serde_json::from_value::<ErrorMethodNotImplemented>(p) {
                    Ok(v) => ErrorKind::MethodNotImplemented(v.method.unwrap_or_default()),
                    Err(_) => ErrorKind::MethodNotImplemented(String::new()),
                },
                _ => ErrorKind::MethodNotImplemented(String::new()),
            },
            _ => ErrorKind::VarlinkErrorReply(e),
        }
    }
}

impl ErrorKind {
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

/**
 This trait has to be implemented by any varlink interface implementor.
 All methods are generated by the varlink-rust-generator, so you don't have to care
 about them.
**/
pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call_upgraded(&self, call: &mut Call, bufreader: &mut BufRead) -> Result<Vec<u8>>;
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

#[derive(Debug, PartialEq, Default, Clone)]
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

            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continues: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl Reply {
    pub fn parameters(parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
            error: None,
            parameters,
        }
    }

    pub fn error<S: Into<Cow<'static, str>>>(name: S, parameters: Option<Value>) -> Self {
        Reply {
            continues: None,
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

/**
 Call is a struct, which is passed as the first argument to the interface methods
 in a derived form.

 See also the [CallTrait](trait.CallTrait.html) to use with the first Call parameter

 If your varlink method is called `TestMethod`, the rust method to be implemented is called
 `test_method`. The first parameter is of type `Call_TestMethod`, which has the method `reply()`.

 # Examples

```rust
 # #![allow(non_camel_case_types)]
 # #![allow(non_snake_case)]
 # use std::io;
 # pub trait VarlinkCallError: varlink::CallTrait {}
 # impl<'a> VarlinkCallError for varlink::Call<'a> {}
 # pub trait Call_TestMethod: VarlinkCallError {
 #     fn reply(&mut self) -> varlink::Result<()> {
 #         self.reply_struct(varlink::Reply::parameters(None))
 #     }
 # }
 # impl<'a> Call_TestMethod for varlink::Call<'a> {}
 # struct TestService;
 # impl TestService {
 fn test_method(&self, call: &mut Call_TestMethod, /* more arguments */) -> varlink::Result<()> {
     /* ... */
     return call.reply( /* more arguments */ );
 }
 # }
 # fn main() {}
 ```
**/
pub struct Call<'a> {
    pub writer: &'a mut Write,
    pub request: Option<&'a Request<'a>>,
    continues: bool,
    upgraded: bool,
}

/**
  CallTrait provides convenience methods for the `Call` struct, which is passed as
  the first argument to the interface methods.

 #  Examples

  For an invalid parameter:

  ```rust
 # #![allow(non_camel_case_types)]
 # #![allow(non_snake_case)]
 # use std::io;
 # pub trait VarlinkCallError: varlink::CallTrait {}
 # impl<'a> VarlinkCallError for varlink::Call<'a> {}
 # pub trait Call_TestMethod: VarlinkCallError {
 #     fn reply(&mut self) -> varlink::Result<()> {
 #         self.reply_struct(varlink::Reply::parameters(None))
 #     }
 # }
 # impl<'a> Call_TestMethod for varlink::Call<'a> {}
 # struct TestService;
 # impl TestService {
 fn test_method(&self, call: &mut Call_TestMethod, testparam: i64) -> varlink::Result<()> {
     match testparam {
         0 ... 100 => {},
         _ => {
             return call.reply_invalid_parameter("testparam".into());
         }
     }
     /* ... */
     Ok(())
 }
 # }
 # fn main() {}
  ```

  For not yet implemented methods:

  ```rust
 # #![allow(non_camel_case_types)]
 # #![allow(non_snake_case)]
 # use std::io;
 # pub trait VarlinkCallError: varlink::CallTrait {}
 # impl<'a> VarlinkCallError for varlink::Call<'a> {}
 # pub trait Call_TestMethodNotImplemented: VarlinkCallError {
 #     fn reply(&mut self) -> varlink::Result<()> {
 #         self.reply_struct(varlink::Reply::parameters(None))
 #     }
 # }
 # impl<'a> Call_TestMethodNotImplemented for varlink::Call<'a> {}
 # struct TestService;
 # impl TestService {
 fn test_method_not_implemented(&self,
                                call: &mut Call_TestMethodNotImplemented) -> varlink::Result<()> {
     return call.reply_method_not_implemented("TestMethodNotImplemented".into());
 }
 # }
 # fn main() {}
 ```
**/

pub trait CallTrait {
    ///  Don't use this directly. Rather use the standard `reply()` method.
    fn reply_struct(&mut self, reply: Reply) -> Result<()>;

    /** Set this to `true` to indicate, that more replies are following.

    # Examples

    ```rust
    # #![allow(non_camel_case_types)]
    # #![allow(non_snake_case)]
    # use std::io;
    # pub trait VarlinkCallError: varlink::CallTrait {}
    # impl<'a> VarlinkCallError for varlink::Call<'a> {}
    # pub trait Call_TestMethod: VarlinkCallError {
    #     fn reply(&mut self) -> varlink::Result<()> {
    #         self.reply_struct(varlink::Reply::parameters(None))
    #     }
    # }
    # impl<'a> Call_TestMethod for varlink::Call<'a> {}
    # struct TestService;
    # impl TestService {
    fn test_method(&self, call: &mut Call_TestMethod) -> varlink::Result<()> {
         call.set_continues(true);
         call.reply( /* more args*/ )?;
         call.reply( /* more args*/ )?;
         call.reply( /* more args*/ )?;
         call.set_continues(false);
         return call.reply( /* more args*/ );
     }
     # }
     # fn main() {}
    ```
    **/
    fn set_continues(&mut self, cont: bool);

    fn to_upgraded(&mut self);

    /// True, if this request does not want a reply.
    fn is_oneway(&self) -> bool;

    /// True, if this request accepts more than one reply.
    fn wants_more(&self) -> bool;

    fn get_request(&self) -> Option<&Request>;

    /// reply with the standard varlink `org.varlink.service.MethodNotFound` error
    fn reply_method_not_found(&mut self, method_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotFound",
            Some(
                serde_json::to_value(ErrorMethodNotFound {
                    method: Some(method_name),
                })
                .map_err(minto_cherr!())?,
            ),
        ))
    }

    /// reply with the standard varlink `org.varlink.service.MethodNotImplemented` error
    fn reply_method_not_implemented(&mut self, method_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotImplemented",
            Some(
                serde_json::to_value(ErrorMethodNotImplemented {
                    method: Some(method_name),
                })
                .map_err(minto_cherr!())?,
            ),
        ))
    }

    /// reply with the standard varlink `org.varlink.service.InvalidParameter` error
    fn reply_invalid_parameter(&mut self, parameter_name: String) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InvalidParameter",
            Some(
                serde_json::to_value(ErrorInvalidParameter {
                    parameter: Some(parameter_name),
                })
                .map_err(minto_cherr!())?,
            ),
        ))
    }
}

impl<'a> CallTrait for Call<'a> {
    fn reply_struct(&mut self, mut reply: Reply) -> Result<()> {
        if self.continues && (!self.wants_more()) {
            return Err(into_cherr!(ErrorKind::CallContinuesMismatch));
        }
        if self.continues {
            reply.continues = Some(true);
        }
        //serde_json::to_writer(&mut *self.writer, &reply)?;
        let b = serde_json::to_string(&reply).map_err(minto_cherr!())? + "\0";

        self.writer
            .write_all(b.as_bytes())
            .map_err(minto_cherr!())?;
        self.writer.flush().map_err(minto_cherr!())?;
        Ok(())
    }

    fn set_continues(&mut self, cont: bool) {
        self.continues = cont;
    }

    fn to_upgraded(&mut self) {
        self.upgraded = true;
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
    pub fn new(writer: &'a mut Write, request: &'a Request<'a>) -> Self {
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
            upgraded: true,
        }
    }

    pub fn reply_interface_not_found(&mut self, arg: Option<String>) -> Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InterfaceNotFound",
            match arg {
                Some(a) => Some(
                    serde_json::to_value(ErrorInterfaceNotFound { interface: Some(a) })
                        .map_err(minto_cherr!())?,
                ),
                None => None,
            },
        ))
    }

    fn reply_parameters(&mut self, parameters: Value) -> Result<()> {
        let reply = Reply::parameters(Some(parameters));
        //serde_json::to_writer(&mut *self.writer, &reply)?;
        let b = serde_json::to_string(&reply).map_err(minto_cherr!())? + "\0";

        self.writer
            .write_all(b.as_bytes())
            .map_err(minto_cherr!())?;
        self.writer.flush().map_err(minto_cherr!())?;
        Ok(())
    }
}

/// A client connection builder to a varlink service.
#[derive(Default)]
pub struct Connection {
    pub reader: Option<BufReader<Box<Read + Send + Sync>>>,
    pub writer: Option<Box<Write + Send + Sync>>,
    address: String,
    #[allow(dead_code)] // For the stream Drop()
    stream: Option<client::VarlinkStream>,
    child: Option<Child>,
    tempdir: Option<TempDir>,
}

impl Connection {
    /**
     Create a connection with a varlink URI

     see [with_address](#method.with_address)
    **/
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: ?Sized + AsRef<str>>(address: &S) -> Result<Arc<RwLock<Self>>> {
        Self::with_address(address)
    }

    /** Create a connection with a varlink URI

    following the varlink
    [address specification](https: //github.com/varlink/documentation/wiki#address).

    Currently supported address URIs are:

    - TCP `tcp:127.0.0.1:12345` hostname/IP address and port
    - UNIX socket `unix:/run/org.example.ftl`
    - UNIX abstract namespace socket `unix:@org.example.ftl` (on Linux only)

    # Examples

    ```rust,ignore
    let connection = Connection::with_address("unix:/tmp/org.example.myservice");
    let connection = Connection::with_address("tcp:127.0.0.1:12345");
    ```
    **/
    pub fn with_address<S: ?Sized + AsRef<str>>(address: &S) -> Result<Arc<RwLock<Self>>> {
        let (mut stream, address) = client::VarlinkStream::connect(address)?;
        let (r, w) = stream.split()?;
        let bufreader = BufReader::new(r);
        Ok(Arc::new(RwLock::new(Connection {
            reader: Some(bufreader),
            writer: Some(w),
            address,
            stream: Some(stream),
            child: None,
            tempdir: None,
        })))
    }

    /** Create a connection to a service, which is executed in the background.

    Create a connection to a service, which is started with `command` and passed a socket pair
    via socket activation. The address of the unix socket is set in the environment variable
    `VARLINK_ADDRESS`. Additionally the socket activation variables `LISTEN_FDS=1`,
    `LISTEN_FDNAMES=varlink` and `LISTEN_PID` are set.

    # Examples

    ```rust,ignore
    let connection = Connection::with_activate("myservice --varlink=$VARLINK_ADDRESS");
    ```
    **/
    pub fn with_activate<S: ?Sized + AsRef<str>>(command: &S) -> Result<Arc<RwLock<Self>>> {
        let (child, unix_address, temp_dir) = varlink_exec(command)?;
        let (mut stream, address) = client::VarlinkStream::connect(&unix_address)?;
        let (reader, writer) = stream.split()?;
        let bufreader = BufReader::new(reader);
        Ok(Arc::new(RwLock::new(Connection {
            reader: Some(bufreader),
            writer: Some(writer),
            address,
            stream: Some(stream),
            child: Some(child),
            tempdir: temp_dir,
        })))
    }

    /** Create a connection to a service via stdin/stdout of a specified command.

    Create a "bridge" to e.g. another host via `ssh` or other connection commands.
    On the remote side `varlink bridge` is typically started.
    The connection will go through stdin/stdout of the command and the remote bridge command
    will multiplex to the wanted varlink services.

    Of course with `ssh` there are better options, like unix socket
    forwarding `-L local_socket:remote_socket`.

    # Examples

    ```rust,ignore
    let connection = Connection::with_bridge("ssh my.example.org -- varlink bridge");
    ```
    **/
    pub fn with_bridge<S: ?Sized + AsRef<str>>(command: &S) -> Result<Arc<RwLock<Self>>> {
        let (child, mut stream) = varlink_bridge(command)?;
        let (r, w) = stream.split()?;
        let bufreader = BufReader::new(r);
        Ok(Arc::new(RwLock::new(Connection {
            reader: Some(bufreader),
            writer: Some(w),
            address: "bridge".into(),
            stream: Some(stream),
            child: Some(child),
            tempdir: None,
        })))
    }

    /**
     Return the `address` used by the connection.

     Only useful, if you want to clone a connection built
     [with_activate](#method.with_activate) or [with_address](#method.with_address)
    **/
    pub fn address(&self) -> String {
        self.address.clone()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(ref mut stream) = self.stream {
            let _r = stream.shutdown();
        }
        if let Some(ref mut child) = self.child {
            let _res = child.kill();
            let _res = child.wait();
        }
        if self.tempdir.is_some() {
            if let Some(dir) = self.tempdir.take() {
                use std::fs;
                let _r = fs::remove_dir_all(dir);
            }
        }
    }
}

pub struct MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: chainerror::ChainErrorFrom<ErrorKind>
        + chainerror::ChainErrorFrom<Reply>
        + chainerror::ChainErrorFrom<serde_json::error::Error>
        + chainerror::ChainErrorFrom<::std::io::Error>,
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

impl<MRequestParameters, MReply, MError> MethodCall<MRequestParameters, MReply, MError>
where
    MRequestParameters: Serialize,
    MReply: DeserializeOwned,
    MError: chainerror::ChainErrorFrom<ErrorKind>
        + chainerror::ChainErrorFrom<Reply>
        + chainerror::ChainErrorFrom<serde_json::error::Error>
        + chainerror::ChainErrorFrom<::std::io::Error>,
{
    pub fn new<S: Into<Cow<'static, str>>>(
        connection: Arc<RwLock<Connection>>,
        method: S,
        parameters: MRequestParameters,
    ) -> Self {
        MethodCall::<MRequestParameters, MReply, MError> {
            connection,
            request: Some(parameters),
            method: Some(method.into()),
            continues: false,
            reader: None,
            writer: None,
            phantom_reply: PhantomData,
            phantom_error: PhantomData,
        }
    }

    fn send(&mut self, oneway: bool, more: bool, upgrade: bool) -> ChainResult<(), MError> {
        {
            let mut conn = self.connection.write().unwrap();
            let mut req = match (self.method.take(), self.request.take()) {
                (Some(method), Some(request)) => Request::create(
                    method,
                    Some(serde_json::to_value(request).map_err(minto_cherr!())?),
                ),
                _ => {
                    return Err(into_cherr!(ErrorKind::MethodCalledAlready));
                }
            };

            if conn.reader.is_none() || conn.writer.is_none() {
                return Err(into_cherr!(ErrorKind::ConnectionBusy));
            }

            if oneway {
                req.oneway = Some(true);
            } else {
                self.reader = conn.reader.take();
            }

            if more {
                req.more = Some(true);
            }

            if upgrade {
                req.upgrade = Some(true);
            }

            let mut w = conn.writer.take().unwrap();

            let b = serde_json::to_string(&req).map_err(minto_cherr!())? + "\0";

            w.write_all(b.as_bytes()).map_err(minto_cherr!())?;
            w.flush().map_err(minto_cherr!())?;
            if oneway {
                conn.writer = Some(w);
            } else {
                self.writer = Some(w);
            }
        }
        Ok(())
    }

    pub fn call(&mut self) -> ChainResult<MReply, MError> {
        self.send(false, false, false)?;
        self.recv()
    }

    pub fn upgrade(&mut self) -> ChainResult<MReply, MError> {
        self.send(false, false, true)?;
        self.recv()
    }

    pub fn oneway(&mut self) -> ChainResult<(), MError> {
        self.send(true, false, false)
    }

    pub fn more(&mut self) -> ChainResult<&mut Self, MError> {
        self.continues = true;
        self.send(false, true, false)?;
        Ok(self)
    }

    pub fn recv(&mut self) -> ChainResult<MReply, MError> {
        if self.reader.is_none() || self.writer.is_none() {
            return Err(into_cherr!(ErrorKind::IteratorOldReply));
        }

        let mut buf = Vec::new();

        let mut reader = self.reader.take().unwrap();
        reader
            .read_until(0, &mut buf)
            .map_err(minto_cherr!())?;
        self.reader = Some(reader);
        if buf.is_empty() {
            return Err(into_cherr!(ErrorKind::ConnectionClosed));
        }
        buf.pop();
        let reply: Reply = serde_json::from_slice(&buf).map_err(minto_cherr!())?;
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
            return Err(into_cherr!(reply));
        }

        match reply {
            Reply {
                parameters: Some(p),
                ..
            } => {
                let mreply: MReply = serde_json::from_value(p).map_err(minto_cherr!())?;
                Ok(mreply)
            }
            Reply {
                parameters: None, ..
            } => {
                let mreply: MReply =
                    serde_json::from_value(serde_json::Value::Object(serde_json::Map::new()))
                        .map_err(minto_cherr!())?;
                Ok(mreply)
            }
        }
    }
}

impl<MRequest, MReply, MError> Iterator for MethodCall<MRequest, MReply, MError>
where
    MRequest: Serialize,
    MReply: DeserializeOwned,
    MError: chainerror::ChainErrorFrom<ErrorKind>
        + chainerror::ChainErrorFrom<Reply>
        + chainerror::ChainErrorFrom<serde_json::error::Error>
        + chainerror::ChainErrorFrom<::std::io::Error>,
{
    type Item = ChainResult<MReply, MError>;
    fn next(&mut self) -> Option<ChainResult<MReply, MError>> {
        if !self.continues {
            return None;
        }

        Some(self.recv())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct GetInterfaceDescriptionArgs<'a> {
    pub interface: Cow<'a, str>,
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
        MethodCall::<GetInfoArgs, ServiceInfo, ErrorKind>::new(
            self.connection.clone(),
            "org.varlink.service.GetInfo",
            GetInfoArgs {},
        )
        .call()
    }
    fn get_interface_description<S: Into<Cow<'static, str>>>(
        &mut self,
        interface: S,
    ) -> Result<GetInterfaceDescriptionReply> {
        MethodCall::<GetInterfaceDescriptionArgs, GetInterfaceDescriptionReply, ErrorKind>::new(
            self.connection.clone(),
            "org.varlink.service.GetInterfaceDescription",
            GetInterfaceDescriptionArgs {
                interface: interface.into(),
            },
        )
        .call()
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

    fn call_upgraded(&self, call: &mut Call, _bufreader: &mut BufRead) -> Result<Vec<u8>> {
        call.upgraded = false;
        Ok(Vec::new())
    }

    fn call(&self, call: &mut Call) -> Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "org.varlink.service.GetInfo" => call.reply_parameters(
                serde_json::to_value(&self.info).map_err(minto_cherr!())?,
            ),
            "org.varlink.service.GetInterfaceDescription" => match req.parameters.as_ref() {
                None => call.reply_invalid_parameter("parameters".into()),
                Some(val) => {
                    let args: GetInterfaceDescriptionArgs =
                        serde_json::from_value(val.clone()).map_err(minto_cherr!())?;
                    match args.interface.as_ref() {
                        "org.varlink.service" => {
                            call.reply_parameters(json!({"description": self.get_description()}))
                        }
                        key => {
                            if self.ifaces.contains_key(key) {
                                call.reply_parameters(
                                    json!({"description": self.ifaces[key].get_description()}),
                                )
                            } else {
                                call.reply_invalid_parameter("interface".into())
                            }
                        }
                    }
                }
            },
            m => call.reply_method_not_found(m.to_string()),
        }
    }
}

impl VarlinkService {
    /**
     Create a new `VarlinkService`.

     See the [Service](https://github.com/varlink/documentation/wiki/Service) section of the
     varlink wiki about the `vendor`, `product`, `version` and `url`.

     The `interfaces` vector is an array of varlink `Interfaces` this service provides.

    # Examples

    ```rust
    # use std::io;
    # struct Interface;
    # impl varlink::Interface for Interface {
    # fn get_description(&self) -> &'static str {
    #                    "interface org.example.ping\nmethod Ping(ping: string) -> (pong: string)" }
    # fn get_name(&self) -> &'static str { "org.example.ping" }
    # fn call_upgraded(&self, call: &mut varlink::Call, _bufreader: &mut io::BufRead) ->
    # varlink::Result<Vec<u8>> { Ok(Vec::new()) }
    # fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> { Ok(()) }
    # }
    # fn main_f() {
    # let interface_foo = Interface{};
    # let interface_bar = Interface{};
    # let interface_baz = Interface{};
    let service = varlink::VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![
            Box::new(interface_foo),
            Box::new(interface_bar),
            Box::new(interface_baz),
        ],
    );
    # }
    # fn main() {}
    ```
    **/
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
        ifnames.extend(ifhashmap.keys().cloned());
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

    fn call(&self, iface: &str, call: &mut Call) -> Result<()> {
        match iface {
            "org.varlink.service" => self::Interface::call(self, call),
            key => {
                if self.ifaces.contains_key(key) {
                    self.ifaces[key].call(call)
                } else {
                    call.reply_interface_not_found(Some(iface.into()))
                }
            }
        }
    }

    fn call_upgraded(
        &self,
        iface: &str,
        call: &mut Call,
        bufreader: &mut BufRead,
    ) -> Result<Vec<u8>> {
        match iface {
            "org.varlink.service" => self::Interface::call_upgraded(self, call, bufreader),
            key => {
                if self.ifaces.contains_key(key) {
                    self.ifaces[key].call_upgraded(call, bufreader)
                } else {
                    call.reply_interface_not_found(Some(iface.into()))?;
                    Ok(Vec::new())
                }
            }
        }
    }
}

pub trait ConnectionHandler {
    fn handle(
        &self,
        bufreader: &mut BufRead,
        writer: &mut Write,
        upgraded_iface: Option<String>,
    ) -> Result<(Vec<u8>, Option<String>)>;
}

impl ConnectionHandler for VarlinkService {
    /**
     ```handle()``` consumes every null terminated message from ```reader```
     and writes the reply to ```writer```.

     This method can be used to implement your own server.
     Pass it one or more null terminated received messages in a ```BufReader``` and reply to the
     sender with the filled ```writer``` buffer.

     Returns Ok(true), if the connection is ```upgraded```. For ```upgraded``` connections
     messages are in legacy format and

    # Examples

    ```rust
    # #![allow(non_camel_case_types)]
    # #![allow(non_snake_case)]
    # use std::io;
    use varlink::{ConnectionHandler, VarlinkService};

    # fn main_func() {
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![], // more interfaces ...
    );
    let mut in_buf = io::BufReader::new("received null terminated message(s) go here \000".as_bytes());
    let mut out: Vec<u8> = Vec::new();
    assert!(service.handle(&mut in_buf, &mut out, None).is_ok());
    # }
    # fn main() {}
    ```
    **/
    fn handle(
        &self,
        bufreader: &mut BufRead,
        writer: &mut Write,
        upgraded_last_interface: Option<String>,
    ) -> Result<(Vec<u8>, Option<String>)> {
        let mut upgraded_iface = upgraded_last_interface.clone();
        loop {
            if let Some(iface) = upgraded_iface {
                let mut call = Call::new_upgraded(writer);
                let unread = self.call_upgraded(&iface, &mut call, bufreader)?;
                return Ok((unread, Some(iface)));
            }

            let mut buf = Vec::new();
            let len = bufreader
                .read_until(b'\0', &mut buf)
                .map_err(minto_cherr!())?;

            if len == 0 {
                // EOF
                return Ok((buf, None));
            }

            if buf.get(len - 1).unwrap_or(&b'x') != &b'\0' {
                // Incomplete message
                return Ok((buf, None));
            }

            // pop the last zero byte
            buf.pop();

            let req: Request = serde_json::from_slice(&buf).map_err(|e| {
                cherr!(
                    e,
                    ErrorKind::SerdeJsonDe(String::from_utf8_lossy(&buf).to_string())
                )
            })?;

            let n: usize = match req.method.rfind('.') {
                None => {
                    let method: String = String::from(req.method.as_ref());
                    let mut call = Call::new(writer, &req);
                    call.reply_interface_not_found(Some(method))?;
                    return Ok((Vec::new(), None));
                }
                Some(x) => x,
            };

            let iface = String::from(&req.method[..n]);

            let mut call = Call::new(writer, &req);
            self.call(&iface, &mut call)?;

            if call.upgraded {
                upgraded_iface = Some(iface);
                break;
            }
        }
        #[cfg(any(feature = "bufreader_buffer", feature = "nightly"))]
        return Ok((bufreader.buffer(), upgraded_iface));

        #[cfg(not(any(feature = "bufreader_buffer", feature = "nightly")))]
        return Ok((Vec::new(), upgraded_iface));
    }
}
