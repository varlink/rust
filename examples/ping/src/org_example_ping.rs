#![doc = "This file was automatically generated by the varlink rust generator"]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use serde_derive::{Deserialize, Serialize};
use std::io::BufRead;
use std::sync::{Arc, RwLock};
use varlink::{self, CallTrait};
#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum ErrorKind {
    Varlink_Error,
    VarlinkReply_Error,
    PingError(Option<PingError_Args>),
}
impl ::std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            ErrorKind::Varlink_Error => write!(f, "Varlink Error"),
            ErrorKind::VarlinkReply_Error => write!(f, "Varlink error reply"),
            ErrorKind::PingError(v) => write!(f, "org.example.ping.PingError: {:#?}", v),
        }
    }
}
pub struct Error(
    pub ErrorKind,
    pub Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
    pub Option<&'static str>,
);
impl Error {
    #[allow(dead_code)]
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }
}
impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Error(e, None, None)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::error::Error as StdError;
        if let Some(ref o) = self.2 {
            std::fmt::Display::fmt(o, f)?;
        }
        std::fmt::Debug::fmt(&self.0, f)?;
        if let Some(e) = self.source() {
            std::fmt::Display::fmt("\nCaused by:\n", f)?;
            std::fmt::Debug::fmt(&e, f)?;
        }
        Ok(())
    }
}
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Error>;
impl From<varlink::Error> for Error {
    fn from(e: varlink::Error) -> Self {
        match e.kind() {
            varlink::ErrorKind::VarlinkErrorReply(r) => Error(
                ErrorKind::from(r),
                Some(Box::from(e)),
                Some(concat!(file!(), ":", line!(), ": ")),
            ),
            _ => Error(
                ErrorKind::Varlink_Error,
                Some(Box::from(e)),
                Some(concat!(file!(), ":", line!(), ": ")),
            ),
        }
    }
}
#[allow(dead_code)]
impl Error {
    pub fn source_varlink_kind(&self) -> Option<&varlink::ErrorKind> {
        use std::error::Error as StdError;
        let mut s: &dyn StdError = self;
        while let Some(c) = s.source() {
            let k = self
                .source()
                .and_then(|e| e.downcast_ref::<varlink::Error>())
                .map(|e| e.kind());
            if k.is_some() {
                return k;
            }
            s = c;
        }
        None
    }
}
impl From<&varlink::Reply> for ErrorKind {
    #[allow(unused_variables)]
    fn from(e: &varlink::Reply) -> Self {
        match e {
            varlink::Reply {
                error: Some(ref t), ..
            } if t == "org.example.ping.PingError" => match e {
                varlink::Reply {
                    parameters: Some(p),
                    ..
                } => match serde_json::from_value(p.clone()) {
                    Ok(v) => ErrorKind::PingError(v),
                    Err(_) => ErrorKind::PingError(None),
                },
                _ => ErrorKind::PingError(None),
            },
            _ => ErrorKind::VarlinkReply_Error,
        }
    }
}
#[allow(dead_code)]
pub trait VarlinkCallError: varlink::CallTrait {
    fn reply_ping_error(&mut self, r#parameter: i64) -> varlink::Result<()> {
        self.reply_struct(varlink::Reply::error(
            "org.example.ping.PingError",
            Some(
                serde_json::to_value(PingError_Args { r#parameter })
                    .map_err(varlink::map_context!())?,
            ),
        ))
    }
}
impl VarlinkCallError for varlink::Call<'_> {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PingError_Args {
    pub r#parameter: i64,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Ping_Reply {
    pub r#pong: String,
}
impl varlink::VarlinkReply for Ping_Reply {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Ping_Args {
    pub r#ping: String,
}
#[allow(dead_code)]
pub trait Call_Ping: VarlinkCallError {
    fn reply(&mut self, r#pong: String) -> varlink::Result<()> {
        self.reply_struct(Ping_Reply { r#pong }.into())
    }
}
impl Call_Ping for varlink::Call<'_> {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Upgrade_Reply {}
impl varlink::VarlinkReply for Upgrade_Reply {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Upgrade_Args {}
#[allow(dead_code)]
pub trait Call_Upgrade: VarlinkCallError {
    fn reply(&mut self) -> varlink::Result<()> {
        self.reply_struct(varlink::Reply::parameters(None))
    }
}
impl Call_Upgrade for varlink::Call<'_> {}
#[allow(dead_code)]
pub trait VarlinkInterface {
    fn ping(&self, call: &mut dyn Call_Ping, r#ping: String) -> varlink::Result<()>;
    fn upgrade(&self, call: &mut dyn Call_Upgrade) -> varlink::Result<()>;
    fn call_upgraded(
        &self,
        _call: &mut varlink::Call,
        _bufreader: &mut dyn BufRead,
    ) -> varlink::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}
#[allow(dead_code)]
pub trait VarlinkClientInterface {
    fn ping(&mut self, r#ping: String) -> varlink::MethodCall<Ping_Args, Ping_Reply, Error>;
    fn upgrade(&mut self) -> varlink::MethodCall<Upgrade_Args, Upgrade_Reply, Error>;
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
    fn ping(&mut self, r#ping: String) -> varlink::MethodCall<Ping_Args, Ping_Reply, Error> {
        varlink::MethodCall::<Ping_Args, Ping_Reply, Error>::new(
            self.connection.clone(),
            "org.example.ping.Ping",
            Ping_Args { r#ping },
        )
    }
    fn upgrade(&mut self) -> varlink::MethodCall<Upgrade_Args, Upgrade_Reply, Error> {
        varlink::MethodCall::<Upgrade_Args, Upgrade_Reply, Error>::new(
            self.connection.clone(),
            "org.example.ping.Upgrade",
            Upgrade_Args {},
        )
    }
}
#[allow(dead_code)]
pub struct VarlinkInterfaceProxy {
    inner: Box<dyn VarlinkInterface + Send + Sync>,
}
#[allow(dead_code)]
pub fn new(inner: Box<dyn VarlinkInterface + Send + Sync>) -> VarlinkInterfaceProxy {
    VarlinkInterfaceProxy { inner }
}
impl varlink::Interface for VarlinkInterfaceProxy {
    fn get_description(&self) -> &'static str {
        "# Example service\ninterface org.example.ping\n\n# Returns the same string\nmethod Ping(ping: string) -> (pong: string)\n\nmethod Upgrade() -> ()\n\nerror PingError(parameter: int)"
    }
    fn get_name(&self) -> &'static str {
        "org.example.ping"
    }
    fn call_upgraded(
        &self,
        call: &mut varlink::Call,
        bufreader: &mut dyn BufRead,
    ) -> varlink::Result<Vec<u8>> {
        self.inner.call_upgraded(call, bufreader)
    }
    fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "org.example.ping.Ping" => {
                if let Some(args) = req.parameters.clone() {
                    let args: Ping_Args = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => {
                            let es = format!("{}", e);
                            let _ = call.reply_invalid_parameter(es.clone());
                            return Err(varlink::context!(varlink::ErrorKind::SerdeJsonDe(es)));
                        }
                    };
                    self.inner.ping(call as &mut dyn Call_Ping, args.r#ping)
                } else {
                    call.reply_invalid_parameter("parameters".into())
                }
            }
            "org.example.ping.Upgrade" => self.inner.upgrade(call as &mut dyn Call_Upgrade),
            m => call.reply_method_not_found(String::from(m)),
        }
    }
}
