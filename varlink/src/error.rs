use std::io;
#[cfg(feature = "chainerror")]
pub use chainerror::*;

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
            ErrorKind::VarlinkErrorReply(v) => write!(f, "Varlink error reply: '{:#?}'", v),
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

impl From<&std::io::Error> for ErrorKind {
    fn from(e: &io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset => ErrorKind::ConnectionClosed,
            kind => ErrorKind::Io(kind),
        }
    }
}

impl From<&serde_json::error::Error> for ErrorKind {
    fn from(e: &serde_json::error::Error) -> Self {
        ErrorKind::SerdeJsonSer(e.classify())
    }
}

#[cfg(feature = "chainerror")]
derive_err_kind!(Error, ErrorKind);

#[cfg(not(feature = "chainerror"))]
pub struct Error(
    pub ErrorKind,
    pub Option<Box<dyn std::error::Error + 'static>>,
    pub Option<&'static str>,
);

#[cfg(not(feature = "chainerror"))]
impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }
}

#[cfg(not(feature = "chainerror"))]
impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Error(e, None, None)
    }
}

#[cfg(not(feature = "chainerror"))]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1.as_ref().map(|e| e.as_ref())
    }
}

#[cfg(not(feature = "chainerror"))]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(not(feature = "chainerror"))]
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

#[macro_export]
#[cfg(not(feature = "chainerror"))]
macro_rules! minto_cherr {
    ( $k:ident ) => (
        |e| $crate::cherr!(e, $k::from(&e))
    );
    ( $enum:ident $(:: $enum_path:ident)* ) => (
        |e| $crate::cherr!(e, $enum $(:: $enum_path)*::from(&e))
    );
}

#[macro_export]
#[cfg(not(feature = "chainerror"))]
macro_rules! cherr {
    ( $k:expr ) => ({
        $crate::error::Error($k, None, Some(concat!(file!(), ":", line!(), ": ")))
    });
    ( None, $k:expr ) => ({
        $crate::error::Error($k, None, Some(concat!(file!(), ":", line!(), ": ")))
    });
    ( None, $fmt:expr, $($arg:tt)+ ) => ({
        $crate::cherr!(None, format!($fmt, $($arg)+ ))
    });
    ( None, $fmt:expr, $($arg:tt)+ ) => ({
        $crate::error::cherr!(None, format!($fmt, $($arg)+ ))
    });
    ( $e:path, $k:expr ) => ({
        $crate::error::Error($k, Some(Box::from($e)), Some(concat!(file!(), ":", line!(), ": ")))
    });
    ( $e:path, $fmt:expr, $($arg:tt)+ ) => ({
        $crate::cherr!($e, format!($fmt, $($arg)+ ))
    });
}

pub type Result<T> = std::result::Result<T, Error>;
