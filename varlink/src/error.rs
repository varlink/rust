use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "IO error")]
    Io(::std::io::ErrorKind),
    #[fail(display = "JSON Serialization Error")]
    SerdeJsonSer(::serde_json::error::Category),
    #[fail(display = "JSON Deserialization Error of '{}'", _0)]
    SerdeJsonDe(String),
    #[fail(display = "Interface not found: '{}'", _0)]
    InterfaceNotFound(String),
    #[fail(display = "Invalid parameter: '{}'", _0)]
    InvalidParameter(String),
    #[fail(display = "Method not found: '{}'", _0)]
    MethodNotFound(String),
    #[fail(display = "Method not implemented: '{}'", _0)]
    MethodNotImplemented(String),
    #[fail(display = "Unknown error reply: '{:#?}'", _0)]
    VarlinkErrorReply(crate::Reply),
    #[fail(display = "Call::reply() called with continues, but without more in the request")]
    CallContinuesMismatch,
    #[fail(display = "Varlink: method called already")]
    MethodCalledAlready,
    #[fail(display = "Varlink: connection busy with other method")]
    ConnectionBusy,
    #[fail(display = "Varlink: Iterator called on old reply")]
    IteratorOldReply,
    #[fail(display = "Server Error")]
    Server,
    #[fail(display = "Timeout Error")]
    Timeout,
    #[fail(display = "Connection Closed")]
    ConnectionClosed,
    #[fail(display = "Invalid varlink address URI")]
    InvalidAddress,
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
        match kind {
            ::std::io::ErrorKind::BrokenPipe
            | ::std::io::ErrorKind::ConnectionReset
            | ::std::io::ErrorKind::ConnectionAborted => {
                e.context(ErrorKind::ConnectionClosed).into()
            }
            _ => e.context(ErrorKind::Io(kind)).into(),
        }
    }
}

impl From<::serde_json::Error> for Error {
    fn from(e: ::serde_json::Error) -> Error {
        let category = e.classify();
        e.context(ErrorKind::SerdeJsonSer(category)).into()
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
