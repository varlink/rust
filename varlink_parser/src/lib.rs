//! varlink_parser crate for parsing varlink interface definition files.

extern crate bytes;
extern crate itertools;

use itertools::Itertools;
use self::varlink_grammar::VInterface;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;
use std::io::{self, Error, ErrorKind};

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
    pub parm: VStruct<'a>,
}

pub enum VStructOrEnum<'a> {
    VStruct(Box<VStruct<'a>>),
    VEnum(Box<VEnum<'a>>),
}

pub struct Typedef<'a> {
    pub name: &'a str,
    pub elt: VStructOrEnum<'a>,
}

pub struct Method<'a> {
    pub name: &'a str,
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
        write!(f, "interface {}\n", self.name)?;

        for t in self.typedefs.values() {
            write!(f, "type {} {}\n", t.name, t.elt)?;
        }

        for m in self.methods.values() {
            write!(f, "method {}{} -> {}\n", m.name, m.input, m.output)?;
        }

        for e in self.errors.values() {
            write!(f, "error {} {}\n", e.name, e.parm)?;
        }
        Ok(())
    }
}

impl<'a> Interface<'a> {
    fn from_token(n: &'a str, mt: Vec<MethodOrTypedefOrError<'a>>) -> Interface<'a> {
        let mut i = Interface {
            name: n,
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

#[test]
fn test_standard() {
    let v = Varlink::from_string(
        "
# The Varlink Service Interface is provided by every varlink service. It
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
",
    ).unwrap();
    assert_eq!(v.interface.name, "org.varlink.service");
    //println!("{}", v.interface.to_string());
    assert_eq!(
        v.interface.to_string(),
        "interface org.varlink.service\n\
         method GetInfo() -> (vendor: string, product: string, \
         version: string, url: string, interfaces: []string)\n\
         method GetInterfaceDescription(interface: string) \
         -> (description: string)\n\
         error InterfaceNotFound (interface: string)\n\
         error InvalidParameter (parameter: string)\n\
         error MethodNotFound (method: string)\n\
         error MethodNotImplemented (method: string)\n"
    );
}

#[test]
fn test_complex() {
    let v = Varlink::from_string(
        "interface org.example.complex
type TypeEnum ( a, b, c )

type TypeFoo (
    bool: bool,
    int: int,
    float: float,
    string: string,
    enum: ( foo, bar, baz ),
    type: TypeEnum,
    anon: ( foo: bool, bar: int, baz: ( a: int, b: int) )
)

method Foo(a: (b: bool, c: int), foo: TypeFoo) -> (a: (b: bool, c: int), foo: TypeFoo)

error ErrorFoo (a: (b: bool, c: int), foo: TypeFoo)
",
    ).unwrap();
    assert_eq!(v.interface.name, "org.example.complex");
    //println!("{}", v.interface.to_string());
    assert_eq!(
        v.interface.to_string(),
        "interface org.example.complex\n\
         type TypeEnum (a, b, c)\n\
         type TypeFoo (bool: bool, int: int, float: float, \
         string: string, enum: (foo, bar, baz), \
         type: TypeEnum, anon: (foo: bool, bar: int, baz: (a: int, b: int)))\n\
         method Foo(a: (b: bool, c: int), foo: TypeFoo) \
         -> (a: (b: bool, c: int), foo: TypeFoo)\n\
         error ErrorFoo (a: (b: bool, c: int), foo: TypeFoo)\n"
    );
}

#[test]
fn test_one_method() {
    let v = Varlink::from_string("interface foo.bar\nmethod Foo()->()");
    assert!(v.is_ok());
}

#[test]
fn test_one_method_no_type() {
    assert!(VInterface("interface foo.bar\nmethod Foo()->(b:)").is_err());
}

#[test]
fn test_domainnames() {
    assert!(Varlink::from_string("interface org.varlink.service\nmethod F()->()").is_ok());
    assert!(Varlink::from_string("interface com.example.0example\nmethod F()->()").is_ok());
    assert!(Varlink::from_string("interface com.example.example-dash\nmethod F()->()").is_ok());
    assert!(
        Varlink::from_string("interface xn--lgbbat1ad8j.example.algeria\nmethod F()->()").is_ok()
    );
    assert!(Varlink::from_string("interface com.-example.leadinghyphen\nmethod F()->()").is_err());
    assert!(
        Varlink::from_string("interface com.example-.danglinghyphen-\nmethod F()->()").is_err()
    );
    assert!(
        Varlink::from_string("interface Com.example.uppercase-toplevel\nmethod F()->()").is_err()
    );
    assert!(Varlink::from_string("interface Co9.example.number-toplevel\nmethod F()->()").is_err());
    assert!(Varlink::from_string("interface 1om.example.number-toplevel\nmethod F()->()").is_err());
    assert!(Varlink::from_string("interface com.Example\nmethod F()->()").is_err());
}

#[test]
fn test_no_method() {
    assert!(
        Varlink::from_string(
            "
interface org.varlink.service
  type Interface (name: string, types: []Type, methods: []Method)
  type Property (key: string, value: string)
",
        ).is_err()
    );
}

#[test]
fn test_type_no_args() {
    assert!(Varlink::from_string("interface foo.bar\n type I ()\nmethod F()->()").is_ok());
}

#[test]
fn test_type_one_arg() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b:bool)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_enum() {
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b: (foo, bar, baz))\nmethod F()->()")
            .is_ok()
    );
}

#[test]
fn test_type_string() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b: string)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_object() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b: object)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_int() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b: int)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_float() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b: float)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_one_array() {
    assert!(Varlink::from_string("interface foo.bar\n type I (b:[]bool)\nmethod  F()->()").is_ok());
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b:bool[ ])\nmethod  F()->()").is_err()
    );
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b:[ ]bool)\nmethod  F()->()").is_err()
    );
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b:[1]bool)\nmethod  F()->()").is_err()
    );
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b:[ 1 ]bool)\nmethod  F()->()").is_err()
    );
    assert!(
        Varlink::from_string("interface foo.bar\n type I (b:[ 1 1 ]bool)\nmethod  F()->()")
            .is_err()
    );
}

#[test]
fn test_format() {
    let v = Varlink::from_string("interface foo.bar\ntype I(b:[]bool)\nmethod  F()->()").unwrap();
    assert_eq!(
        v.interface.to_string(),
        "\
interface foo.bar
type I (b: []bool)
method F() -> ()
"
    );
}

#[test]
fn test_duplicate() {
    let e = Varlink::from_string(
        "
interface foo.example
	type Device()
	type Device()
	type T()
	type T()
	method F() -> ()
	method F() -> ()
",
    ).err()
        .unwrap();
    assert_eq!(
        e.to_string(),
        "\
Interface `foo.example`: multiple definitions of type `Device`!
Interface `foo.example`: multiple definitions of type `F`!
Interface `foo.example`: multiple definitions of type `T`!"
    );
}
