use chainerror::*;
use std::io;

#[derive(Clone, PartialEq, Debug)]
pub enum ErrorKind {
    Io(::std::io::ErrorKind),
    SerdeJsonSer(::serde_json::error::Category),
    SerdeJsonDe(String),
    InterfaceNotFound(String),
    InvalidParameter(String),
    MethodNotFound(String),
    MethodNotImplemented(String),
    VarlinkErrorReply(crate::Reply),
    CallContinuesMismatch,
    MethodCalledAlready,
    ConnectionBusy,
    IteratorOldReply,
    Server,
    Timeout,
    ConnectionClosed,
    InvalidAddress,
    Generic,
}

impl ::std::error::Error for ErrorKind {}

impl ::std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            ErrorKind::Io(_) => write!(f, "IO error"),
            ErrorKind::SerdeJsonSer(_) => write!(f, "JSON Serialization Error"),
            ErrorKind::SerdeJsonDe(v) => write!(f, "JSON Deserialization Error of '{}'", v),
            ErrorKind::InterfaceNotFound(v) => write!(f, "Interface not found: '{}'", v),
            ErrorKind::InvalidParameter(v) => write!(f, "Invalid parameter: '{}'", v),
            ErrorKind::MethodNotFound(v) => write!(f, "Method not found: '{}'", v),
            ErrorKind::MethodNotImplemented(v) => write!(f, "Method not implemented: '{}'", v),
            ErrorKind::VarlinkErrorReply(v) => write!(f, "Unknown error reply: '{:#?}'", v),
            ErrorKind::CallContinuesMismatch => write!(
                f,
                "Call::reply() called with continues, but without more in the request"
            ),
            ErrorKind::MethodCalledAlready => write!(f, "Varlink: method called already"),
            ErrorKind::ConnectionBusy => write!(f, "Varlink: connection busy with other method"),
            ErrorKind::IteratorOldReply => write!(f, "Varlink: Iterator called on old reply"),
            ErrorKind::Server => write!(f, "Server Error"),
            ErrorKind::Timeout => write!(f, "Timeout Error"),
            ErrorKind::ConnectionClosed => write!(f, "Connection Closed"),
            ErrorKind::InvalidAddress => write!(f, "Invalid varlink address URI"),
            ErrorKind::Generic => Ok(()),
        }
    }
}

impl ChainErrorFrom<std::io::Error> for ErrorKind {
    fn chain_error_from(
        e: io::Error,
        line_filename: Option<(u32, &'static str)>,
    ) -> ChainError<Self> {
        match e.kind() {
            io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset => {
                ChainError::<_>::new(ErrorKind::ConnectionClosed, Some(Box::from(e)), line_filename)
            }

            kind => ChainError::<_>::new(ErrorKind::Io(kind), Some(Box::from(e)), line_filename),
        }
    }
}

impl ChainErrorFrom<serde_json::error::Error> for ErrorKind {
    fn chain_error_from(
        e: serde_json::error::Error,
        line_filename: Option<(u32, &'static str)>,
    ) -> ChainError<Self> {
        ChainError::<_>::new(
            ErrorKind::SerdeJsonSer(e.classify()),
            Some(Box::from(e)),
            line_filename,
        )
    }
}

pub type Result<T> = ChainResult<T, ErrorKind>;
pub type Error = ChainError<ErrorKind>;
