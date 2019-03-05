//! varlink_parser crate for parsing [varlink](http://varlink.org) interface definition files.
//!
//! # Examples
//!
//! ```rust
//! use varlink_parser::IDL;
//! let interface = IDL::from_string("
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
#![deny(
    warnings,
    unsafe_code,
    absolute_paths_not_starting_with_crate,
    deprecated_in_future,
    keyword_idents,
    macro_use_extern_crate,
    missing_debug_implementations,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    unused_labels,
    unused_lifetimes,
    unstable_features,
    unreachable_pub,
    future_incompatible,
    missing_copy_implementations,
    missing_doc_code_examples,
    rust_2018_idioms,
    rust_2018_compatibility
)]
#![allow(elided_lifetimes_in_paths, missing_docs)]

use std::collections::BTreeMap;
use std::collections::HashSet;

use chainerror::*;
use itertools::Itertools;

pub use crate::format::{Format, FormatColored};

pub use self::varlink_grammar::ParseError;
use self::varlink_grammar::ParseInterface;

mod format;

#[cfg(test)]
mod test;

#[cfg(feature = "peg")]
mod varlink_grammar {
    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
}

#[cfg(not(feature = "peg"))]
mod varlink_grammar;

derive_str_cherr!(Error);

#[derive(Debug)]
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

#[derive(Debug)]
pub enum VTypeExt<'a> {
    Array(Box<VTypeExt<'a>>),
    Dict(Box<VTypeExt<'a>>),
    Option(Box<VTypeExt<'a>>),
    Plain(VType<'a>),
}

#[derive(Debug)]
pub struct Argument<'a> {
    pub name: &'a str,
    pub vtype: VTypeExt<'a>,
}

#[derive(Debug)]
pub struct VStruct<'a> {
    pub elts: Vec<Argument<'a>>,
}

#[derive(Debug)]
pub struct VEnum<'a> {
    pub elts: Vec<&'a str>,
}

#[derive(Debug)]
pub struct VError<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub parm: VStruct<'a>,
}

#[derive(Debug)]
pub enum VStructOrEnum<'a> {
    VStruct(Box<VStruct<'a>>),
    VEnum(Box<VEnum<'a>>),
}

#[derive(Debug)]
pub struct Typedef<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub elt: VStructOrEnum<'a>,
}

#[derive(Debug)]
pub struct Method<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub input: VStruct<'a>,
    pub output: VStruct<'a>,
}

#[derive(Debug)]
enum MethodOrTypedefOrError<'a> {
    Error(VError<'a>),
    Typedef(Typedef<'a>),
    Method(Method<'a>),
}

#[derive(Debug)]
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
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, m.name
                        ));
                    }

                    i.method_keys.push(m.name);
                    if let Some(d) = i.methods.insert(m.name, m) {
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of method `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Typedef(t) => {
                    if i.error_keys.contains(&t.name) || i.method_keys.contains(&t.name) {
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, t.name
                        ));
                    }
                    i.typedef_keys.push(t.name);
                    if let Some(d) = i.typedefs.insert(t.name, t) {
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of type `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Error(e) => {
                    if i.typedef_keys.contains(&e.name) || i.method_keys.contains(&e.name) {
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of `{}`!",
                            i.name, e.name
                        ));
                    }
                    i.error_keys.push(e.name);
                    if let Some(d) = i.errors.insert(e.name, e) {
                        let _ = i.error.insert(format!(
                            "Interface `{}`: multiple definitions of error `{}`!",
                            i.name, d.name
                        ));
                    };
                }
            };
        }

        i
    }
}

impl<'a> IDL<'a> {
    pub fn from_string(s: &'a str) -> ChainResult<Self, Error> {
        let interface = ParseInterface(s).map_err(|e| {
            let line = s.split("\n").nth(e.line - 1).unwrap();
            cherr!(
                e,
                Error(format!(
                    "Varlink parse error\n{}\n{marker:>col$}",
                    line,
                    marker = "^",
                    col = e.column
                ))
            )
        })?;
        if !interface.error.is_empty() {
            Err(strerr!(
                Error,
                "Interface definition error: '{}'\n",
                interface.error.into_iter().sorted().join("\n")
            ))
        } else {
            Ok(interface)
        }
    }
}
