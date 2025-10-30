//! varlink_parser crate for parsing [varlink](http://varlink.org) interface definition files.
//!
//! # Examples
//!
//! ```rust
//! use varlink_parser::IDL;
//! use std::convert::TryFrom;
//! let interface = IDL::try_from("
//! ## The Varlink Service Interface is provided by every varlink service. It
//! ## describes the service and the interfaces it implements.
//! interface org.varlink.service
//!
//! ## Get a list of all the interfaces a service provides and information
//! ## about the implementation.
//! method GetInfo() -> (
//! vendor: string,
//! product: string,
//! version: string,
//! url: string,
//! interfaces: []string
//! )
//!
//! ## Get the description of an interface that is implemented by this service.
//! method GetInterfaceDescription(interface: string) -> (description: string)
//!
//! ## The requested interface was not found.
//! error InterfaceNotFound (interface: string)
//!
//! ## The requested method was not found
//! error MethodNotFound (method: string)
//!
//! ## The interface defines the requested method, but the service does not
//! ## implement it.
//! error MethodNotImplemented (method: string)
//!
//! ## One of the passed parameters is invalid.
//! error InvalidParameter (parameter: string)
//! ").unwrap();
//!    assert_eq!(interface.name, "org.varlink.service");
//! ```

#![doc(
    html_logo_url = "https://varlink.org/images/varlink.png",
    html_favicon_url = "https://varlink.org/images/varlink-small.png"
)]

use self::varlink_grammar::ParseInterface;
use std::collections::BTreeMap;
use std::collections::HashSet;

mod format;

pub use crate::format::{Format, FormatColored};
use std::convert::TryFrom;

#[cfg(test)]
mod test;

mod varlink_grammar;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Varlink parse error\n{line}\n{marker:>column$}", marker = "^")]
    Parse { line: String, column: usize },
    #[error("Interface definition error: {0}")]
    Idl(String),
}

pub enum VType<'a> {
    Bool,
    Int,
    Float,
    String,
    Object,
    Typename(&'a str),
    Struct(Box<VStruct<'a>>),
    Enum(Box<VEnum<'a>>),
}

pub enum VTypeExt<'a> {
    Array(Box<VTypeExt<'a>>),
    Dict(Box<VTypeExt<'a>>),
    Option(Box<VTypeExt<'a>>),
    Plain(VType<'a>),
}

pub struct Argument<'a> {
    pub name: &'a str,
    pub vtype: VTypeExt<'a>,
}

pub struct VStruct<'a> {
    pub elts: Vec<Argument<'a>>,
}

pub struct VEnum<'a> {
    pub elts: Vec<&'a str>,
}

pub struct VError<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub parm: VStruct<'a>,
}

pub enum VStructOrEnum<'a> {
    VStruct(Box<VStruct<'a>>),
    VEnum(Box<VEnum<'a>>),
}

pub struct Typedef<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub elt: VStructOrEnum<'a>,
}

pub struct Method<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub input: VStruct<'a>,
    pub output: VStruct<'a>,
}

enum MethodOrTypedefOrError<'a> {
    Error(VError<'a>),
    Typedef(Typedef<'a>),
    Method(Method<'a>),
}

pub struct IDL<'a> {
    pub description: &'a str,
    pub name: &'a str,
    pub doc: &'a str,
    pub methods: BTreeMap<&'a str, Method<'a>>,
    pub method_keys: Vec<&'a str>,
    pub typedefs: BTreeMap<&'a str, Typedef<'a>>,
    pub typedef_keys: Vec<&'a str>,
    pub errors: BTreeMap<&'a str, VError<'a>>,
    pub error_keys: Vec<&'a str>,
    pub error: HashSet<String>,
}

fn trim_doc(s: &str) -> &str {
    s.trim_matches(&[
        ' ', '\n', '\r', '\u{00A0}', '\u{FEFF}', '\u{1680}', '\u{180E}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}', '\u{2028}', '\u{2029}',
    ] as &[_])
}

impl<'a> IDL<'a> {
    fn from_token(
        description: &'a str,
        name: &'a str,
        mt: Vec<MethodOrTypedefOrError<'a>>,
        doc: &'a str,
    ) -> IDL<'a> {
        let mut i = IDL {
            description,
            name,
            doc,
            methods: BTreeMap::new(),
            method_keys: Vec::new(),
            typedefs: BTreeMap::new(),
            typedef_keys: Vec::new(),
            errors: BTreeMap::new(),
            error_keys: Vec::new(),
            error: HashSet::new(),
        };

        for o in mt {
            match o {
                MethodOrTypedefOrError::Method(m) => {
                    if i.error_keys.contains(&m.name) || i.typedef_keys.contains(&m.name) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, m.name
                        ));
                    }

                    i.method_keys.push(m.name);
                    if let Some(d) = i.methods.insert(m.name, m) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of method `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Typedef(t) => {
                    if i.error_keys.contains(&t.name) || i.method_keys.contains(&t.name) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, t.name
                        ));
                    }
                    i.typedef_keys.push(t.name);
                    if let Some(d) = i.typedefs.insert(t.name, t) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of type `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Error(e) => {
                    if i.typedef_keys.contains(&e.name) || i.method_keys.contains(&e.name) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, e.name
                        ));
                    }
                    i.error_keys.push(e.name);
                    if let Some(d) = i.errors.insert(e.name, e) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of error `{}`!",
                            i.name, d.name
                        ));
                    };
                }
            };
        }

        i
    }

    #[deprecated(since = "4.1.0", note = "please use `IDL::try_from` instead")]
    pub fn from_string(s: &'a str) -> Result<Self, Error> {
        IDL::try_from(s)
    }
}

impl<'a> TryFrom<&'a str> for IDL<'a> {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let interface = ParseInterface(value, value).map_err(|e| {
            let line = value.split('\n').nth(e.location.line - 1).unwrap();
            Error::Parse {
                line: line.to_string(),
                column: e.location.column,
            }
        })?;

        if !interface.error.is_empty() {
            let mut v: Vec<_> = interface.error.into_iter().collect();
            v.sort();
            let mut s = v.join("\n");
            s.push('\n');

            Err(Error::Idl(s))
        } else {
            Ok(interface)
        }
    }
}
