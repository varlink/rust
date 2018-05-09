//! varlink_parser crate for parsing varlink interface definition files.

extern crate bytes;
extern crate itertools;

use self::varlink_grammar::VInterface;
use itertools::Itertools;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;
use std::io::{self, Error, ErrorKind};

#[cfg(test)]
mod test;

mod varlink_grammar {
    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
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

pub struct Interface<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub methods: BTreeMap<&'a str, Method<'a>>,
    pub typedefs: BTreeMap<&'a str, Typedef<'a>>,
    pub errors: BTreeMap<&'a str, VError<'a>>,
    pub error: HashSet<Cow<'static, str>>,
}

macro_rules! printVTypeExt {
    ($s:ident, $f:ident, $t:expr) => {{
        write!($f, "{}", $t)?;
    }};
}

impl<'a> fmt::Display for VTypeExt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &VTypeExt::Plain(VType::Bool) => printVTypeExt!(self, f, "bool"),
            &VTypeExt::Plain(VType::Int) => printVTypeExt!(self, f, "int"),
            &VTypeExt::Plain(VType::Float) => printVTypeExt!(self, f, "float"),
            &VTypeExt::Plain(VType::String) => printVTypeExt!(self, f, "string"),
            &VTypeExt::Plain(VType::Object) => printVTypeExt!(self, f, "object"),
            &VTypeExt::Plain(VType::Typename(ref v)) => printVTypeExt!(self, f, v),
            &VTypeExt::Plain(VType::Struct(ref v)) => printVTypeExt!(self, f, v),
            &VTypeExt::Plain(VType::Enum(ref v)) => printVTypeExt!(self, f, v),
            &VTypeExt::Array(ref v) => write!(f, "[]{}", v)?,
            &VTypeExt::Dict(ref v) => write!(f, "[dict]{}", v)?,
            &VTypeExt::Option(ref v) => write!(f, "?{}", v)?,
        }
        Ok(())
    }
}

impl<'a> fmt::Display for VStructOrEnum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VStructOrEnum::VStruct(ref v) => write!(f, "{}", v)?,
            VStructOrEnum::VEnum(ref v) => write!(f, "{}", v)?,
        }
        Ok(())
    }
}

impl<'a> fmt::Display for Argument<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.vtype)?;
        Ok(())
    }
}

impl<'a> fmt::Display for VStruct<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            write!(f, "{}", fst)?;
            for elt in iter {
                write!(f, ", {}", elt)?;
            }
        }
        write!(f, ")")
    }
}

impl<'a> fmt::Display for VEnum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            write!(f, "{}", fst)?;
            for elt in iter {
                write!(f, ", {}", elt)?;
            }
        }
        write!(f, ")")
    }
}

impl<'a> fmt::Display for Interface<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.doc.len() > 0 {
            write!(f, "{}\n", self.doc)?;
        }
        write!(f, "interface {}\n", self.name)?;

        for t in self.typedefs.values() {
            write!(f, "\n")?;
            if t.doc.len() > 0 {
                write!(f, "{}\n", t.doc)?;
            }
            write!(f, "type {} {}\n", t.name, t.elt)?;
        }

        for m in self.methods.values() {
            write!(f, "\n")?;
            if m.doc.len() > 0 {
                write!(f, "{}\n", m.doc)?;
            }
            write!(f, "method {}{} -> {}\n", m.name, m.input, m.output)?;
        }

        for e in self.errors.values() {
            write!(f, "\n")?;
            if e.doc.len() > 0 {
                write!(f, "{}\n", e.doc)?;
            }
            write!(f, "error {} {}\n", e.name, e.parm)?;
        }
        Ok(())
    }
}

pub fn trim_doc(s: &str) -> &str {
    s.trim_matches(&[
        ' ', '\n', '\r', '\u{00A0}', '\u{FEFF}', '\u{1680}', '\u{180E}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}', '\u{2028}', '\u{2029}',
    ] as &[_])
}

impl<'a> Interface<'a> {
    fn from_token(n: &'a str, mt: Vec<MethodOrTypedefOrError<'a>>, doc: &'a str) -> Interface<'a> {
        let mut i = Interface {
            name: n,
            doc: doc,
            methods: BTreeMap::new(),
            typedefs: BTreeMap::new(),
            errors: BTreeMap::new(),
            error: HashSet::new(),
        };

        for o in mt {
            match o {
                MethodOrTypedefOrError::Method(m) => {
                    if let Some(d) = i.methods.insert(m.name, m) {
                        i.error.insert(
                            format!(
                                "Interface `{}`: multiple definitions of type `{}`!",
                                i.name, d.name
                            ).into(),
                        );
                    };
                }
                MethodOrTypedefOrError::Typedef(t) => {
                    if let Some(d) = i.typedefs.insert(t.name, t) {
                        i.error.insert(
                            format!(
                                "Interface `{}`: multiple definitions of type `{}`!",
                                i.name, d.name
                            ).into(),
                        );
                    };
                }
                MethodOrTypedefOrError::Error(e) => {
                    if let Some(d) = i.errors.insert(e.name, e) {
                        i.error.insert(
                            format!(
                                "Interface `{}`: multiple definitions of error `{}`!",
                                i.name, d.name
                            ).into(),
                        );
                    };
                }
            };
        }
        if i.methods.len() == 0 {
            i.error
                .insert(format!("Interface `{}`: no method defined!", i.name).into());
        }

        i
    }
}

pub struct Varlink<'a> {
    pub string: &'a str,
    pub interface: Interface<'a>,
}

impl<'a> Varlink<'a> {
    pub fn from_string(s: &'a str) -> io::Result<Varlink> {
        let iface = match VInterface(s) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, e));
            }
        };

        if iface.error.len() != 0 {
            Err(Error::new(
                ErrorKind::Other,
                iface.error.into_iter().sorted().join("\n"),
            ))
        } else {
            Ok(Varlink {
                string: s,
                interface: iface,
            })
        }
    }
}
