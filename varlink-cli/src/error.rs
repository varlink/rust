use failure::{Backtrace, Context, Fail};

pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, PartialEq, Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "IO error")]
    Io(::std::io::ErrorKind),
    #[fail(display = "Serialization Error")]
    SerdeJsonSer(::serde_json::error::Category),
    #[fail(display = "JSON Deserialization Error of '{}'", _0)]
    SerdeJsonDe(String),
    #[fail(display = "Not yet implemented: '{}'", _0)]
    NotImplemented(String),
    #[fail(display = "Parse Error: '{}'", _0)]
    Parser(::varlink_parser::ErrorKind),
    #[fail(display = "Argument Error")]
    Argument,
    #[fail(display = "Connection Error for '{}'", _0)]
    Connection(String),
    #[fail(display = "Call failed with error: {}\n{}", error, parameters)]
    VarlinkError { error: String, parameters: String },
    #[fail(display = "{}", _0)]
    Varlink(::varlink::ErrorKind),
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

impl ::std::fmt::Debug for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self.inner, f)
    }
}

#[allow(dead_code)]
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
        e.context(ErrorKind::Io(kind)).into()
    }
}

impl From<::serde_json::Error> for Error {
    fn from(e: ::serde_json::Error) -> Error {
        let cat = e.classify();
        e.context(ErrorKind::SerdeJsonSer(cat)).into()
    }
}

impl From<::varlink_parser::Error> for Error {
    fn from(e: ::varlink_parser::Error) -> Self {
        let kind = e.kind();
        e.context(ErrorKind::Parser(kind)).into()
    }
}

impl From<::varlink::Error> for Error {
    fn from(e: ::varlink::Error) -> Self {
        let kind = e.kind();
        match kind {
            ::varlink::ErrorKind::Io(kind) => e.context(ErrorKind::Io(kind)).into(),
            ::varlink::ErrorKind::SerdeJsonSer(cat) => {
                e.context(ErrorKind::SerdeJsonSer(cat)).into()
            }
            ::varlink::ErrorKind::SerdeJsonDe(buf) => e.context(ErrorKind::SerdeJsonDe(buf)).into(),
            ::varlink::ErrorKind::VarlinkErrorReply(reply) => {
                e.context(ErrorKind::VarlinkError {
                    error: reply.error.unwrap_or_default().into(),
                    parameters: ::serde_json::to_string_pretty(
                        &reply.parameters.unwrap_or_default(),
                    ).unwrap_or_default(),
                }).into()
            }
            kind => e.context(ErrorKind::Varlink(kind)).into(),
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
