use serde_json::{self, Value};

use std::convert::From;
use std::io;
use std::collections::HashMap;
use std::borrow::Cow;

use bytes::BytesMut;
use bytes::BufMut;

use futures::{future, Future, BoxFuture};

use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;
use tokio_io::codec::{Encoder, Decoder};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;

pub trait Interface {
    fn get_description(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn call(&self, Request) -> Result<Value, Error>;
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub interface: Cow<'static, str>,
    pub method: Cow<'static, str>,
    pub arguments: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub reply: Value,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ErrorDetails {
    pub id: Cow<'static, str>,
    pub message: Option<Cow<'static, str>>,
    pub data: Option<Value>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Error {
    pub error: ErrorDetails,
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error {
            error: ErrorDetails {
                id: "UnknownError".into(),
                message: Some(e.to_string().into()),
                ..Default::default()
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Ok(Reply),
    Err(Error),
}

pub struct NulJsonCodec;

impl Decoder for NulJsonCodec {
    type Item = Request;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Request>> {
        if let Some(i) = buf.iter().position(|&b| b == 0) {
            // remove the serialized frame from the buffer.

            let line = buf.split_to(i);
            //println!("got {:?}", line);
            // Also remove the '0'
            buf.split_to(1);

            Ok(Some(serde_json::from_slice(&line)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for NulJsonCodec {
    type Item = Response;
    type Error = io::Error;

    fn encode(&mut self, msg: Response, buf: &mut BytesMut) -> io::Result<()> {
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
}

pub struct Proto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Proto {
    // For this protocol style, `Request` matches the `Item` type of the codec's `Encoder`
    type Request = Request;

    // For this protocol style, `Response` matches the `Item` type of the codec's `Decoder`
    type Response = Response;

    // A bit of boilerplate to hook in the codec:
    type Transport = Framed<T, NulJsonCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(NulJsonCodec))
    }
}

#[derive(Deserialize)]
struct GetInterfaceArgs {
    name: Cow<'static, str>,
}

#[derive(Serialize, Deserialize, Default)]
struct Property {
    key: Cow<'static, str>,
    value: Cow<'static, str>,
}

#[derive(Serialize, Deserialize, Default)]
struct ServiceInfo {
    name: Cow<'static, str>,
    description: Cow<'static, str>,
    properties: Vec<Property>,
    interfaces: Vec<Cow<'static, str>>,
}

pub struct VarlinkService {
    info: ServiceInfo,
    ifaces: HashMap<Cow<'static, str>, Box<Interface>>,
}

impl VarlinkService {
    pub fn new(name: Cow<'static, str>,
               description: Cow<'static, str>,
               ifaces: Vec<Box<Interface>>)
               -> Self {
        let mut ifhashmap = HashMap::<Cow<'static, str>, Box<Interface>>::new();
        for i in ifaces {
            ifhashmap.insert(i.get_name().into(), i);
        }
        let mut ifnames: Vec<Cow<'static, str>> = Vec::new();
        ifnames.push("org.varlink.service".into());
        ifnames.extend(ifhashmap
                           .keys()
                           .map(|i| Cow::<'static, str>::from(i.clone())));
        VarlinkService {
            info: ServiceInfo {
                name: name,
                description: description,
                interfaces: ifnames,
                ..Default::default()
            },
            ifaces: ifhashmap,
        }
    }
}

impl Service for VarlinkService {
    // These types must match the corresponding protocol types:
    type Request = Request;
    type Response = Response;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = BoxFuture<Self::Response, Self::Error>;

    // Produce a future for computing a response from a request.
    fn call(&self, req: Self::Request) -> Self::Future {

        println!("Request: {}", serde_json::to_string(&req).unwrap());

        match req.interface.clone().as_ref() {
            "org.varlink.service" => {
                match self::Interface::call(self, req) {
                    Ok(val) => future::ok(Response::Ok(Reply { reply: val })).boxed(), 
                    Err(e) => future::ok(Response::Err(e)).boxed(),
                }
            }
            key => {
                if self.ifaces.contains_key(key) {
                    match self.ifaces[key].call(req) {
                        Ok(val) => future::ok(Response::Ok(Reply { reply: val })).boxed(),
                        Err(e) => future::ok(Response::Err(e)).boxed(),
                    }
                } else {
                    future::ok(Response::Err(Error {
                                                 error: ErrorDetails {
                                                     id: "InterfaceNotFound".into(),
                                                     message: Some("Interface not found".into()),
                                                     ..Default::default()
                                                 },
                                             })).boxed()
                }
            }

        }
    }
}

impl Interface for VarlinkService {
    fn get_description(&self) -> &'static str {
        r#"
# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

type Property (key: string, value: string)

# Returns information about a service. It contains the service name and the
# names of all available interfaces.
method GetInfo() -> (
  name: string,
  description: string,
  properties: Property[],
  interfaces: string[]
)

# Get the description of an interface that is implemented by this service.
method GetInterface(name: string) -> (interfacestring: string)

error BadRequest
error InterfaceNotFound
error MethodNotFound
error MethodNotImplemented

error InternalError (errno: int)
	"#
    }

    fn get_name(&self) -> &'static str {
        "org.varlink.service"
    }

    fn call(&self, req: Request) -> Result<Value, Error> {
        match req.method.as_ref() {
            "GetInfo" => {
                return Ok(serde_json::to_value(&self.info)?);
            }
            "GetInterface" => {
                if req.arguments == None {
                    return Err(Error {
                                   error: ErrorDetails {
                                       id: "InvalidParameter".into(),
                                       message: Some("Arguments empty".into()),
                                       ..Default::default()
                                   },
                               });
                }
                let args: GetInterfaceArgs = serde_json::from_value(req.arguments.unwrap())
                    .unwrap();
                match args.name.as_ref() {
                    "org.varlink.service" => Ok(json!({"interfacestring": self.get_description()})),
                    key => {
                        if self.ifaces.contains_key(key) {
                            Ok(json!({"interfacestring": self.ifaces[key].get_description()}))
                        } else {
                            Err(Error {
                                    error: ErrorDetails {
                                        id: "InvalidParameter".into(),
                                        message: Some("Interface in name not found".into()),
                                        ..Default::default()
                                    },
                                })
                        }
                    }
                }
            }
            _ => {
                Err(Error {
                        error: ErrorDetails {
                            id: "MethodNotFound".into(),
                            message: Some("Method not found".into()),
                            ..Default::default()
                        },
                    })
            }
        }
    }
}
