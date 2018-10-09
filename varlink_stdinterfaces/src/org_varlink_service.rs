#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use failure::{Backtrace, Context, Fail};
use serde_json;
use std::io::BufRead;
use std::sync::{Arc, RwLock};
use varlink::{self, CallTrait};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GetInfo_Reply {
    pub vendor: String,
    pub product: String,
    pub version: String,
    pub url: String,
    pub interfaces: Vec<String>,
}

impl varlink::VarlinkReply for GetInfo_Reply {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GetInfo_Args {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GetInterfaceDescription_Reply {
    pub description: String,
}

impl varlink::VarlinkReply for GetInterfaceDescription_Reply {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GetInterfaceDescription_Args {
    pub interface: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InterfaceNotFound_Args {
    pub interface: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvalidParameter_Args {
    pub parameter: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MethodNotFound_Args {
    pub method: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MethodNotImplemented_Args {
    pub method: String,
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "IO error")]
    Io_Error(::std::io::ErrorKind),
    #[fail(display = "(De)Serialization Error")]
    SerdeJson_Error(serde_json::error::Category),
    #[fail(display = "Varlink Error")]
    Varlink_Error(varlink::ErrorKind),
    #[fail(display = "Unknown error reply: '{:#?}'", _0)]
    VarlinkReply_Error(varlink::Reply),
    #[fail(display = "org.varlink.service.InterfaceNotFound: {:#?}", _0)]
    InterfaceNotFound(Option<InterfaceNotFound_Args>),
    #[fail(display = "org.varlink.service.InvalidParameter: {:#?}", _0)]
    InvalidParameter(Option<InvalidParameter_Args>),
    #[fail(display = "org.varlink.service.MethodNotFound: {:#?}", _0)]
    MethodNotFound(Option<MethodNotFound_Args>),
    #[fail(
        display = "org.varlink.service.MethodNotImplemented: {:#?}",
        _0
    )]
    MethodNotImplemented(Option<MethodNotImplemented_Args>),
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self.inner, f)
    }
}

impl Error {
    #[allow(dead_code)]
    pub fn kind(&self) -> ErrorKind {
        self.inner.get_context().clone()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Error {
        let kind = e.kind();
        e.context(ErrorKind::Io_Error(kind)).into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        let cat = e.classify();
        e.context(ErrorKind::SerdeJson_Error(cat)).into()
    }
}

#[allow(dead_code)]
pub type Result<T> = ::std::result::Result<T, Error>;

impl From<varlink::Error> for Error {
    fn from(e: varlink::Error) -> Self {
        let kind = e.kind();
        match kind {
            varlink::ErrorKind::Io(kind) => e.context(ErrorKind::Io_Error(kind)).into(),
            varlink::ErrorKind::SerdeJsonSer(cat) => {
                e.context(ErrorKind::SerdeJson_Error(cat)).into()
            }
            kind => e.context(ErrorKind::Varlink_Error(kind)).into(),
        }
    }
}

impl From<varlink::Reply> for Error {
    fn from(e: varlink::Reply) -> Self {
        if varlink::Error::is_error(&e) {
            return varlink::Error::from(e).into();
        }

        match e {
            varlink::Reply {
                error: Some(ref t), ..
            }
                if t == "org.varlink.service.InterfaceNotFound" =>
            {
                match e {
                    varlink::Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => ErrorKind::InterfaceNotFound(v).into(),
                        Err(_) => ErrorKind::InterfaceNotFound(None).into(),
                    },
                    _ => ErrorKind::InterfaceNotFound(None).into(),
                }
            }
            varlink::Reply {
                error: Some(ref t), ..
            }
                if t == "org.varlink.service.InvalidParameter" =>
            {
                match e {
                    varlink::Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => ErrorKind::InvalidParameter(v).into(),
                        Err(_) => ErrorKind::InvalidParameter(None).into(),
                    },
                    _ => ErrorKind::InvalidParameter(None).into(),
                }
            }
            varlink::Reply {
                error: Some(ref t), ..
            }
                if t == "org.varlink.service.MethodNotFound" =>
            {
                match e {
                    varlink::Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => ErrorKind::MethodNotFound(v).into(),
                        Err(_) => ErrorKind::MethodNotFound(None).into(),
                    },
                    _ => ErrorKind::MethodNotFound(None).into(),
                }
            }
            varlink::Reply {
                error: Some(ref t), ..
            }
                if t == "org.varlink.service.MethodNotImplemented" =>
            {
                match e {
                    varlink::Reply {
                        parameters: Some(p),
                        ..
                    } => match serde_json::from_value(p) {
                        Ok(v) => ErrorKind::MethodNotImplemented(v).into(),
                        Err(_) => ErrorKind::MethodNotImplemented(None).into(),
                    },
                    _ => ErrorKind::MethodNotImplemented(None).into(),
                }
            }
            _ => ErrorKind::VarlinkReply_Error(e).into(),
        }
    }
}
pub trait Call_GetInfo: varlink::CallTrait {
    fn reply(
        &mut self,
        vendor: String,
        product: String,
        version: String,
        url: String,
        interfaces: Vec<String>,
    ) -> varlink::Result<()> {
        self.reply_struct(
            GetInfo_Reply {
                vendor,
                product,
                version,
                url,
                interfaces,
            }.into(),
        )
    }
}

impl<'a> Call_GetInfo for varlink::Call<'a> {}

pub trait Call_GetInterfaceDescription: varlink::CallTrait {
    fn reply(&mut self, description: String) -> varlink::Result<()> {
        self.reply_struct(GetInterfaceDescription_Reply { description }.into())
    }
}

impl<'a> Call_GetInterfaceDescription for varlink::Call<'a> {}

pub trait VarlinkInterface {
    fn get_info(&self, call: &mut Call_GetInfo) -> varlink::Result<()>;
    fn get_interface_description(
        &self,
        call: &mut Call_GetInterfaceDescription,
        interface: String,
    ) -> varlink::Result<()>;
    fn call_upgraded(
        &self,
        _call: &mut varlink::Call,
        _bufreader: &mut BufRead,
    ) -> varlink::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

pub trait VarlinkClientInterface {
    fn get_info(&mut self) -> varlink::MethodCall<GetInfo_Args, GetInfo_Reply, Error>;
    fn get_interface_description(
        &mut self,
        interface: String,
    ) -> varlink::MethodCall<GetInterfaceDescription_Args, GetInterfaceDescription_Reply, Error>;
}

#[allow(dead_code)]
pub struct VarlinkClient {
    connection: Arc<RwLock<varlink::Connection>>,
}

impl VarlinkClient {
    #[allow(dead_code)]
    pub fn new(connection: Arc<RwLock<varlink::Connection>>) -> Self {
        VarlinkClient { connection }
    }
}

impl VarlinkClientInterface for VarlinkClient {
    fn get_info(&mut self) -> varlink::MethodCall<GetInfo_Args, GetInfo_Reply, Error> {
        varlink::MethodCall::<GetInfo_Args, GetInfo_Reply, Error>::new(
            self.connection.clone(),
            "org.varlink.service.GetInfo",
            GetInfo_Args {},
        )
    }
    fn get_interface_description(
        &mut self,
        interface: String,
    ) -> varlink::MethodCall<GetInterfaceDescription_Args, GetInterfaceDescription_Reply, Error>
    {
        varlink::MethodCall::<GetInterfaceDescription_Args, GetInterfaceDescription_Reply, Error>::new(
            self.connection.clone(),
            "org.varlink.service.GetInterfaceDescription",
            GetInterfaceDescription_Args { interface },
        )
    }
}

#[allow(dead_code)]
pub struct VarlinkInterfaceProxy {
    inner: Box<VarlinkInterface + Send + Sync>,
}

#[allow(dead_code)]
pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> VarlinkInterfaceProxy {
    VarlinkInterfaceProxy { inner }
}

impl varlink::Interface for VarlinkInterfaceProxy {
    fn get_description(&self) -> &'static str {
        r#####################################"# The Varlink Service Interface is provided by every varlink service. It
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
"#####################################
    }

    fn get_name(&self) -> &'static str {
        "org.varlink.service"
    }

    fn call_upgraded(
        &self,
        call: &mut varlink::Call,
        bufreader: &mut BufRead,
    ) -> varlink::Result<Vec<u8>> {
        self.inner.call_upgraded(call, bufreader)
    }

    fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "org.varlink.service.GetInfo" => self.inner.get_info(call as &mut Call_GetInfo),
            "org.varlink.service.GetInterfaceDescription" => {
                if let Some(args) = req.parameters.clone() {
                    let args: GetInterfaceDescription_Args = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => {
                            let es = format!("{}", e);
                            let _ = call.reply_invalid_parameter(es.clone());
                            return Err(varlink::ErrorKind::SerdeJsonDe(es).into());
                        }
                    };
                    self.inner.get_interface_description(
                        call as &mut Call_GetInterfaceDescription,
                        args.interface,
                    )
                } else {
                    call.reply_invalid_parameter("parameters".into())
                }
            }

            m => call.reply_method_not_found(String::from(m)),
        }
    }
}
