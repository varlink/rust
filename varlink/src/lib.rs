//! client and server support for the varlink protocol

extern crate bytes;
extern crate itertools;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

extern crate varlink_parser;

pub mod generator;

use serde_json::Value;
use serde::ser::Serialize;

use std::convert::From;
use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Read, Write};

pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call(&self, &mut Call) -> io::Result<()>;
    fn call_upgraded(&self, &mut Call) -> io::Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Request {
    pub more: Option<bool>,
    pub oneshot: Option<bool>,
    pub upgrade: Option<bool>,
    pub method: Cow<'static, str>,
    pub parameters: Option<Value>,
}

pub trait VarlinkReply {}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    continues: Option<bool>,
    upgraded: Option<bool>,
    error: Option<Cow<'static, str>>,
    parameters: Option<Value>,
}

impl Reply {
    pub fn parameters(parameters: Value) -> Self {
        Reply {
            continues: None,
            upgraded: None,
            error: None,
            parameters: Some(parameters),
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
        Reply::parameters(serde_json::to_value(a).unwrap())
    }
}

pub struct Call<'a> {
    writer: &'a mut Write,
    pub request: Option<&'a Request>,
    pub continues: bool,
    pub upgraded: bool,
}

pub trait CallTrait {
    fn reply_struct(&mut self, Reply) -> io::Result<()>;
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
        let mut buf = serde_json::to_vec(&reply)?;
        buf.push(0);
        self.writer.write_all(&mut buf)?;
        self.writer.flush()?;
        Ok(())
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

    pub fn reply_method_not_found(&mut self, arg: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotFound".into(),
            match arg {
                Some(a) => {
                    let s = format!("{{  \"method\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    pub fn reply_method_not_implemented(&mut self, arg: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.MethodNotImplemented".into(),
            match arg {
                Some(a) => {
                    let s = format!("{{  \"method\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    pub fn reply_invalid_parameter(&mut self, arg: Option<String>) -> io::Result<()> {
        self.reply_struct(Reply::error(
            "org.varlink.service.InvalidParameter".into(),
            match arg {
                Some(a) => {
                    let s = format!("{{  \"parameter\" : \"{}\" }}", a);
                    Some(serde_json::from_str(s.as_ref()).unwrap())
                }
                None => None,
            },
        ))
    }

    fn reply_parameters(&mut self, parameters: Value) -> io::Result<()> {
        let reply = Reply::parameters(parameters);
        let mut buf = serde_json::to_vec(&reply)?;
        buf.push(0);
        self.writer.write_all(&mut buf)?;
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
    pub fn new(
        vendor: &str,
        product: &str,
        version: &str,
        url: &str,
        ifaces: Vec<Box<Interface + Send + Sync>>,
    ) -> Self {
        let mut ifhashmap = HashMap::<Cow<'static, str>, Box<Interface + Send + Sync>>::new();
        for i in ifaces {
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
