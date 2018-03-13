use serde_json::{self, Value};

use std::convert::From;
use std::io;
use std::collections::HashMap;
use std::borrow::Cow;

use bytes::BytesMut;
use bytes::BufMut;

use std::io::{BufRead, BufReader, Read, Write};

pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call(&self, Request) -> Result<Value, Error>;
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub method: Cow<'static, str>,
    pub parameters: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub parameters: Option<Value>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Error {
    pub error: Cow<'static, str>,
    pub parameters: Option<Value>,
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

impl From<VarlinkError> for Error {
    fn from(e: VarlinkError) -> Self {
        Error {
            error: match e {
                VarlinkError::MethodNotFound(_) => "org.varlink.service.MethodNotFound".into(),
                VarlinkError::InterfaceNotFound(_) => {
                    "org.varlink.service.InterfaceNotFound".into()
                }
                VarlinkError::MethodNotImplemented(_) => {
                    "org.varlink.service.MethodNotImplemented".into()
                }
                VarlinkError::InvalidParameter(_) => "org.varlink.service.InvalidParameter".into(),
            },
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

impl From<serde_json::Error> for Error {
    fn from(_e: serde_json::Error) -> Self {
        VarlinkError::InvalidParameter(Some(_e.to_string().into())).into()
    }
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Ok(Reply),
    Err(Error),
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

    fn call(&self, req: Request) -> Result<Value, Error> {
        match req.method.as_ref() {
            "org.varlink.service.GetInfo" => {
                return Ok(serde_json::to_value(&self.info)?);
            }
            "org.varlink.service.GetInterfaceDescription" => {
                if req.parameters == None {
                    return Err(VarlinkError::InvalidParameter(None).into());
                }
                let args: GetInterfaceArgs =
                    serde_json::from_value(req.parameters.unwrap()).unwrap();
                match args.interface.as_ref() {
                    "org.varlink.service" => Ok(json!({"description": self.get_description()})),
                    key => {
                        if self.ifaces.contains_key(key) {
                            Ok(json!({"description": self.ifaces[key].get_description()}))
                        } else {
                            Err(VarlinkError::InvalidParameter(Some("interface".into())).into())
                        }
                    }
                }
            }
            _ => {
                let method: String = req.method.clone().into();
                let n: usize = match method.rfind('.') {
                    None => 0,
                    Some(x) => x + 1,
                };
                let m = String::from(&method[n..]);
                Err(VarlinkError::MethodNotFound(Some(m.clone().into())).into())
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

    fn call(&self, req: Request) -> Response {
        println!("Request: {}", serde_json::to_string(&req).unwrap());
        let n: usize = match req.method.rfind('.') {
            None => {
                return Response::Err(
                    VarlinkError::InterfaceNotFound(Some(req.method.into())).into(),
                )
            }
            Some(x) => x,
        };
        let iface = String::from(&req.method[..n]);

        match iface.as_ref() {
            "org.varlink.service" => match self::Interface::call(self, req) {
                Ok(val) => Response::Ok(Reply {
                    parameters: Some(val),
                }),
                Err(e) => Response::Err(e),
            },
            key => {
                if self.ifaces.contains_key(key) {
                    match self.ifaces[key].call(req) {
                        Ok(val) => Response::Ok(Reply {
                            parameters: Some(val.clone()),
                        }),
                        Err(e) => Response::Err(e),
                    }
                } else {
                    Response::Err(
                        VarlinkError::InterfaceNotFound(Some(iface.clone().into())).into(),
                    )
                }
            }
        }
    }

    fn encode(&self, msg: Response, buf: &mut BytesMut) -> io::Result<()> {
        match msg {
            Response::Ok(val) => {
                println!("Response: {}", serde_json::to_string(&val).unwrap());
                buf.extend(serde_json::to_vec(&val)?)
            }
            Response::Err(val) => {
                println!("Response: {}", serde_json::to_string(&val).unwrap());
                buf.extend(serde_json::to_vec(&val)?)
            }
        }
        buf.put_u8(0);
        Ok(())
    }

    pub fn handle(&self, reader: &mut Read, writer: &mut Write) -> io::Result<()> {
        let mut bufreader = BufReader::new(reader);
        loop {
            let mut buf = Vec::new();
            let read_bytes = bufreader.read_until(b'\0', &mut buf).unwrap();
            if read_bytes > 0 {
                let req: Request = serde_json::from_slice(&buf)?;
                let res = self.call(req);
                let mut buf = BytesMut::new();
                self.encode(res, &mut buf)?;
                writer.write_all(&mut buf)?;
            } else {
                break;
            }
        }
        Ok(())
    }
}
