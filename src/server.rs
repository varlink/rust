use serde_json::{self, Value};
use serde::ser::Serialize;

use std::convert::From;
use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{self, BufRead, BufReader, Read, Write};

pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call(&self, &mut Call) -> io::Result<()>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub method: Cow<'static, str>,
    pub parameters: Option<Value>,
}

pub trait VarlinkReply {}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    error: Option<Cow<'static, str>>,
    parameters: Option<Value>,
}

impl Reply {
    pub fn parameters(parameters: Value) -> Self {
        Reply {
            error: None,
            parameters: Some(parameters),
        }
    }

    pub fn error(name: Cow<'static, str>, parameters: Option<Value>) -> Self {
        Reply {
            error: Some(name),
            parameters,
        }
    }
}

impl<T> From<T> for Reply
where
    T: VarlinkReply,
    T: Serialize,
{
    fn from(a: T) -> Self {
        Reply::parameters(serde_json::to_value(a).unwrap())
    }
}

pub struct Call<'a> {
    writer: &'a mut Write,
    pub request: &'a Request,
}

impl<'a> Call<'a> {
    fn new(writer: &'a mut Write, request: &'a Request) -> Self {
        Call { writer, request }
    }

    fn reply_parameters(&mut self, parameters: Value) -> io::Result<()> {
        let reply = Reply::parameters(parameters);
        let mut buf = serde_json::to_vec(&reply)?;
        buf.push(0);
        self.writer.write_all(&mut buf)?;
        Ok(())
    }

    pub fn reply(&mut self, reply: Reply) -> io::Result<()> {
        let mut buf = serde_json::to_vec(&reply)?;
        buf.push(0);
        self.writer.write_all(&mut buf)?;
        Ok(())
    }
}

/*
# The requested interface was not found.
error InterfaceNotFound (interface: string)

# The requested method was not found
error MethodNotFound (method: string)

# The interface defines the requested method, but the service does not
# implement it.
error MethodNotImplemented (method: string)

# One of the passed parameters is invalid.
error InvalidParameter (parameter: string)
*/

#[derive(Debug)]
pub enum VarlinkError {
    InterfaceNotFound(Option<Cow<'static, str>>),
    MethodNotFound(Option<Cow<'static, str>>),
    MethodNotImplemented(Option<Cow<'static, str>>),
    InvalidParameter(Option<Cow<'static, str>>),
}

impl From<VarlinkError> for Reply {
    fn from(e: VarlinkError) -> Self {
        Reply {
            error: Some(match e {
                VarlinkError::MethodNotFound(_) => "org.varlink.service.MethodNotFound".into(),
                VarlinkError::InterfaceNotFound(_) => {
                    "org.varlink.service.InterfaceNotFound".into()
                }
                VarlinkError::MethodNotImplemented(_) => {
                    "org.varlink.service.MethodNotImplemented".into()
                }
                VarlinkError::InvalidParameter(_) => "org.varlink.service.InvalidParameter".into(),
            }),
            parameters: match e {
                VarlinkError::InterfaceNotFound(m) => match m {
                    Some(i) => Some(
                        serde_json::from_str(format!("{{ \"interface\" : \"{}\" }}", i).as_ref())
                            .unwrap(),
                    ),
                    None => None,
                },
                VarlinkError::MethodNotFound(m) => match m {
                    Some(me) => {
                        let method: String = me.into();
                        let n: usize = match method.rfind('.') {
                            None => 0,
                            Some(x) => x + 1,
                        };
                        let (_, method) = method.split_at(n);
                        let s = format!("{{  \"method\" : \"{}\" }}", method);
                        Some(serde_json::from_str(s.as_ref()).unwrap())
                    }
                    None => None,
                },
                VarlinkError::MethodNotImplemented(m) => match m {
                    Some(me) => {
                        let method: String = me.into();
                        let n: usize = match method.rfind('.') {
                            None => 0,
                            Some(x) => x + 1,
                        };
                        let (_, method) = method.split_at(n);
                        let s = format!("{{  \"method\" : \"{}\" }}", method);
                        Some(serde_json::from_str(s.as_ref()).unwrap())
                    }
                    None => None,
                },
                VarlinkError::InvalidParameter(m) => match m {
                    Some(i) => Some(
                        serde_json::from_str(format!("{{ \"parameter\" : \"{}\" }}", i).as_ref())
                            .unwrap(),
                    ),
                    None => None,
                },
            },
        }
    }
}

impl From<serde_json::Error> for Reply {
    fn from(_e: serde_json::Error) -> Self {
        VarlinkError::InvalidParameter(Some(_e.to_string().into())).into()
    }
}

#[derive(Deserialize)]
struct GetInterfaceArgs {
    interface: Cow<'static, str>,
}

#[derive(Serialize, Deserialize, Default)]
struct Property {
    key: Cow<'static, str>,
    value: Cow<'static, str>,
}

#[derive(Serialize, Deserialize, Default)]
struct ServiceInfo {
    vendor: Cow<'static, str>,
    product: Cow<'static, str>,
    version: Cow<'static, str>,
    url: Cow<'static, str>,
    interfaces: Vec<Cow<'static, str>>,
}

pub struct VarlinkService {
    info: ServiceInfo,
    ifaces: HashMap<Cow<'static, str>, Box<Interface>>,
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
        match call.request.method.as_ref() {
            "org.varlink.service.GetInfo" => {
                return call.reply_parameters(serde_json::to_value(&self.info)?);
            }
            "org.varlink.service.GetInterfaceDescription" => {
                if call.request.parameters == None {
                    return call.reply(VarlinkError::InvalidParameter(None).into());
                }
                if let Some(val) = call.request.parameters.clone() {
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
                                return call.reply(
                                    VarlinkError::InvalidParameter(Some("interface".into())).into(),
                                );
                            }
                        }
                    }
                } else {
                    return call.reply(
                        VarlinkError::InvalidParameter(Some("interface".into())).into(),
                    );
                }
            }
            _ => {
                let method: String = call.request.method.clone().into();
                let n: usize = match method.rfind('.') {
                    None => 0,
                    Some(x) => x + 1,
                };
                let m = String::from(&method[n..]);
                return call.reply(VarlinkError::MethodNotFound(Some(m.clone().into())).into());
            }
        }
    }
}

impl VarlinkService {
    pub fn new(
        vendor: &str,
        product: &str,
        version: &str,
        url: &str,
        ifaces: Vec<Box<Interface>>,
    ) -> Self {
        let mut ifhashmap = HashMap::<Cow<'static, str>, Box<Interface>>::new();
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
                ..Default::default()
            },
            ifaces: ifhashmap,
        }
    }

    fn call(&self, call: &mut Call) -> io::Result<()> {
        println!("Request: {}", serde_json::to_string(&call.request).unwrap());
        let n: usize = match call.request.method.rfind('.') {
            None => {
                let method = call.request.method.clone();
                return call.reply(VarlinkError::InterfaceNotFound(Some(method.into())).into());
            }
            Some(x) => x,
        };
        let iface = String::from(&call.request.method[..n]);

        match iface.as_ref() {
            "org.varlink.service" => return self::Interface::call(self, call),
            key => {
                if self.ifaces.contains_key(key) {
                    return self.ifaces[key].call(call);
                } else {
                    return call.reply(
                        VarlinkError::InterfaceNotFound(Some(iface.clone().into())).into(),
                    );
                }
            }
        }
    }

    pub fn handle(&self, reader: &mut Read, writer: &mut Write) -> io::Result<()> {
        let mut bufreader = BufReader::new(reader);
        loop {
            let mut buf = Vec::new();
            let read_bytes = bufreader.read_until(b'\0', &mut buf).unwrap();
            if read_bytes > 0 {
                buf.pop();
                let req: Request = serde_json::from_slice(&buf)?;
                self.call(&mut Call::new(writer, &req))?;
            } else {
                break;
            }
        }
        Ok(())
    }
}
