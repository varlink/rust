extern crate failure;
extern crate failure_derive;
extern crate varlink_parser;

use failure::{Backtrace, Context, Fail};
use std::env;
use std::fmt::{self, Display};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use varlink_parser::{Varlink, FormatColored};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "IO error")]
    Io,
    #[fail(display = "Parse Error")]
    Parser,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
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
        e.context(ErrorKind::Io).into()
    }
}

impl From<varlink_parser::Error> for Error {
    fn from(e: varlink_parser::Error) -> Error {
        e.context(ErrorKind::Parser).into()
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

fn main() -> Result<()> {
    let mut buffer = String::new();
    let args: Vec<_> = env::args().collect();

    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
        _ => File::open(Path::new(&args[1]))?.read_to_string(&mut buffer)?,
    };

    let v = Varlink::from_string(&buffer)?;
    println!("{}", v.interface.get_multiline_colored(0, 80));
    exit(0);
}
