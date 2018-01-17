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

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error {
            error: "UnknownError".into(),
            ..Default::default()
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

impl VarlinkService {
    pub fn new(vendor: Cow<'static, str>,
               product: Cow<'static, str>,
               version: Cow<'static, str>,
               url: Cow<'static, str>,
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
                vendor: vendor,
                product: product,
                version: version,
                url: url,
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
        let n: usize = match req.method.rfind('.') {
            None => {
                return future::ok(Response::Err(Error {
                                                    error: "InterfaceNotFound".into(),
                                                    parameters: Some(json!({"interface": req.method})),
                                                    ..Default::default()
                                                })).boxed()
            }
            Some(x) => x,
        };
        let method: String = req.method.clone().into();
        let (iface, _) = method.split_at(n);

        match iface.as_ref() {
            "org.varlink.service" => {
                match self::Interface::call(self, req) {
                    Ok(val) => future::ok(Response::Ok(Reply { parameters: Some(val) })).boxed(), 
                    Err(e) => future::ok(Response::Err(e)).boxed(),
                }
            }
            key => {
                if self.ifaces.contains_key(key) {
                    match self.ifaces[key].call(req) {
                        Ok(val) => {
                            future::ok(Response::Ok(Reply { parameters: Some(val) })).boxed()
                        }
                        Err(e) => future::ok(Response::Err(e)).boxed(),
                    }
                } else {
                    future::ok(Response::Err(Error {
                                                 error: "InterfaceNotFound".into(),
                                                 parameters: Some(json!({"interface": key})),
                                                 ..Default::default()
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
                    return Err(Error {
                                   error: "InvalidParameter".into(),
                                   ..Default::default()
                               });
                }
                let args: GetInterfaceArgs = serde_json::from_value(req.parameters.unwrap())
                    .unwrap();
                match args.interface.as_ref() {
                    "org.varlink.service" => Ok(json!({"description": self.get_description()})),
                    key => {
                        if self.ifaces.contains_key(key) {
                            Ok(json!({"description": self.ifaces[key].get_description()}))
                        } else {
                            Err(Error {
                                    error: "InvalidParameter".into(),
                                    parameters: Some(json!({"parameter": "interface"})),
                                    ..Default::default()
                                })
                        }
                    }
                }
            }
            m => {
                let method: String = m.into();
                let n: usize = match method.rfind('.') {
                    None => 0,
                    Some(x) => x + 1,
                };
                let (_, method) = method.split_at(n);

                Err(Error {
                        error: "MethodNotFound".into(),
                        parameters: Some(json!({"method": method})),
                        ..Default::default()
                    })
            }
        }
    }
}
