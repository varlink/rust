mod varlink_grammar {
    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
}

use std::collections::BTreeMap;
use std::borrow::Cow;
use std::collections::HashSet;
use itertools::Itertools;
use self::varlink_grammar::Interfaces;

pub enum VType<'a> {
    Bool(Option<bool>),
    Int8(Option<i8>),
    UInt8(Option<u8>),
    Int16(Option<i16>),
    UInt16(Option<u16>),
    Int32(Option<i32>),
    UInt32(Option<u32>),
    Int64(Option<i64>),
    UInt64(Option<u64>),
    Float32(Option<f32>),
    Float64(Option<f64>),
    VString(Option<&'a str>),
    VTypename(&'a str),
    VStruct(Box<VStruct<'a>>),
}

pub struct VTypeExt<'a> {
    vtype: VType<'a>,
    nullable: bool,
    isarray: Option<usize>,
}

pub struct Argument<'a> {
    pub name: &'a str,
    pub vtypes: Vec<VTypeExt<'a>>,
}

pub struct VStruct<'a> {
    pub elts: Vec<Argument<'a>>,
}

pub struct Typedef<'a> {
    pub name: &'a str,
    pub vstruct: VStruct<'a>,
}

pub struct Method<'a> {
    pub name: &'a str,
    pub input: VStruct<'a>,
    pub output: VStruct<'a>,
    pub stream: bool,
}

enum MethodOrTypedef<'a> {
    Typedef(Typedef<'a>),
    Method(Method<'a>),
}

pub struct Interface<'a> {
    pub name: &'a str,
    pub methods: BTreeMap<&'a str, Method<'a>>,
    pub typedefs: BTreeMap<&'a str, Typedef<'a>>,
    pub error: HashSet<Cow<'static, str>>,
}

use std::fmt;
macro_rules! printVTypeExt {
	($s:ident, $f:ident, $t:expr) => {{
                write!($f, "{}", $t)?;
                if let Some(t) = $s.isarray {
                    match t {
                        0 => write!($f, "[]")?,
                        _ => write!($f, "[{}]", t)?,
                    }
                };
                if $s.nullable {
                    write!($f, "?")?
                };
	}};
	($s:ident, $f:ident, $v:ident, $t:expr) => {{
                write!($f, "{}", $t)?;
                if let Some(t) = $s.isarray {
                    match t {
                        0 => write!($f, "[]")?,
                        _ => write!($f, "[{}]", t)?,
                    }
                };
                if $s.nullable {
                    write!($f, "?")?
                };
                if let Some(val) = *$v {
                    write!($f, " = {}", val)?;
                }
	}};
	($s:ident, $f:ident, $v:ident, $t:expr, $k:expr) => {{
                write!($f, "{}", $t)?;
                if let Some(t) = $s.isarray {
                    match t {
                        0 => write!($f, "[]")?,
                        _ => write!($f, "[{}]", t)?,
                    }
                };
                if $s.nullable {
                    write!($f, "?")?
                };
                if let Some(val) = *$v {
                    write!($f, " = {s}{}{s}", val, s=$k)?;
                }
	}};
}

impl<'a> fmt::Display for VTypeExt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.vtype {
            VType::Bool(ref v) => printVTypeExt!(self, f, v, "bool"),
            VType::Int8(ref v) => printVTypeExt!(self, f, v, "int8"),
            VType::UInt8(ref v) => printVTypeExt!(self, f, v, "uint8"),
            VType::Int16(ref v) => printVTypeExt!(self, f, v, "int16"),
            VType::UInt16(ref v) => printVTypeExt!(self, f, v, "uint16"),
            VType::Int32(ref v) => printVTypeExt!(self, f, v, "int32"),
            VType::UInt32(ref v) => printVTypeExt!(self, f, v, "uint32"),
            VType::Int64(ref v) => printVTypeExt!(self, f, v, "int64"),
            VType::UInt64(ref v) => printVTypeExt!(self, f, v, "uint64"),
            VType::Float32(ref v) => printVTypeExt!(self, f, v, "float32"),
            VType::Float64(ref v) => printVTypeExt!(self, f, v, "float64"),
            VType::VString(ref v) => printVTypeExt!(self, f, v, "string", "\""),
            VType::VTypename(ref v) => printVTypeExt!(self, f, v),
            VType::VStruct(ref v) => printVTypeExt!(self, f, v),
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Argument<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.vtypes.len() == 1 {
            write!(f, "{}: {}", self.name, self.vtypes[0])?;
        } else {
            let mut iter = self.vtypes.iter();
            if let Some(fst) = iter.next() {
                write!(f, "{}: {}", self.name, fst)?;
                for elt in iter {
                    write!(f, " | {}", elt)?;
                }
            }
        }
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

impl<'a> fmt::Display for Interface<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {{\n", self.name)?;

        for t in self.typedefs.values() {
            write!(f, "  type {} {};\n", t.name, t.vstruct)?;
        }

        for m in self.methods.values() {
            write!(f,
                   "  {}{} {}> {};\n",
                   m.name,
                   m.input,
                   match m.stream {
                       true => '=',
                       false => '-',
                   },
                   m.output)?;
        }
        write!(f, "}}\n")
    }
}

impl<'a> Interface<'a> {
    fn from_token(n: &'a str, mt: Vec<MethodOrTypedef<'a>>) -> Interface<'a> {
        let mut i = Interface {
            name: n,
            methods: BTreeMap::new(),
            typedefs: BTreeMap::new(),
            error: HashSet::new(),
        };

        for o in mt {
            match o {
                MethodOrTypedef::Method(m) => {
                    if let Some(d) = i.methods.insert(m.name, m) {
                        i.error
                            .insert(format!("Interface `{}`: multiple definitions of type `{}`!",
                                            i.name,
                                            d.name)
                                .into());
                    };
                }
                MethodOrTypedef::Typedef(t) => {
                    if let Some(d) = i.typedefs.insert(t.name, t) {
                        i.error
                            .insert(format!("Interface `{}`: multiple definitions of type `{}`!",
                                            i.name,
                                            d.name)
                                .into());
                    };
                }
            };
        }
        if i.methods.len() == 0 {
            i.error.insert(format!("Interface `{}`: no method defined!", i.name).into());
        }

        i
    }
}

pub struct Varlink<'a> {
    string: &'a str,
    pub interfaces: BTreeMap<String, Interface<'a>>,
}

impl<'a> Varlink<'a> {
    pub fn from_string(s: &'a str) -> Result<Varlink, String> {

        let mut v = Varlink {
            string: s,
            interfaces: BTreeMap::new(),
        };

        let ifaces = match Interfaces(v.string) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let mut log: HashSet<Cow<'static, str>> = HashSet::new();

        for i in ifaces {

            if v.interfaces.contains_key(i.name.into()) {
                log.insert(format!("Multiple definitions of interface `{}`!", i.name).into());
            }

            log.extend(i.error.clone().into_iter());
            v.interfaces.insert(i.name.into(), i);
        }

        if log.len() != 0 {
            Err(log.into_iter().sorted().join("\n"))
        } else {
            Ok(v)
        }
    }
}


#[test]
fn test_standard() {
    let v = Varlink::from_string("
/**
 * The Varlink Service Interface is added to every varlink service. It provides
 * the Introspect method to be called by a client to retrieve the bootstrap
 * information from a service.
 */
org.varlink.service {
  type Type (name: string, typestring: string)
  type Method (
    name: string,
    monitor: bool,
    type_in: string,
    type_out: string
  )
  type Interface (name: string, types: Type[], methods: Method[])
  type Property (key: string, value: string)
  type InterfaceDescription (
    description: string,
    types: string[],
    methods: string[]
  )

  /**
   * Returns the machine readable information about a service. It contains the service
   * name, all available interfaces with their defined method calls and types.
   */
  Introspect(version: uint64) -> (name: string, interfaces: Interface[])

  /**
   * Returns the human readable description of a service.
   */
  Help() -> (
    description: string,
    properties: Property[],
    interfaces: InterfaceDescription[]
  )
}
")
        .unwrap();
    assert!(v.interfaces.contains_key("org.varlink.service"));
    println!("{}", v.interfaces["org.varlink.service"].to_string());
    assert_eq!(v.interfaces["org.varlink.service"].to_string(),
               "\
org.varlink.service {
  type Interface (name: string, types: Type[], methods: Method[]);
  type InterfaceDescription (description: string, types: string[], methods: string[]);
  type Method (name: string, monitor: bool, type_in: string, type_out: string);
  type Property (key: string, value: string);
  type Type (name: string, typestring: string);
  Help() -> (description: string, properties: Property[], interfaces: InterfaceDescription[]);
  Introspect(version: uint64) -> (name: string, interfaces: Interface[]);
}
");

}

#[test]
fn test_one_method() {
    let v = Varlink::from_string("/* comment */ foo.bar{ Foo()->() }").unwrap();
    assert!(v.interfaces["foo.bar"].methods["Foo"].stream == false);
}

#[test]
fn test_one_method_stream() {
    let v = Varlink::from_string("foo.bar{ Foo()=>() }").unwrap();
    assert!(v.interfaces["foo.bar"].methods["Foo"].stream);
}

#[test]
fn test_one_method_no_type() {
    assert!(Interfaces("foo.bar{ Foo()->(b:) }").is_err());
}

#[test]
fn test_domainnames() {
    assert!(Varlink::from_string("org.varlink.service {F()->()}").is_ok());
    assert!(Varlink::from_string("com.example.0example {F()->()}").is_ok());
    assert!(Varlink::from_string("com.example.example-dash {F()->()}").is_ok());
    assert!(Varlink::from_string("xn--lgbbat1ad8j.example.algeria {F()->()}").is_ok());
    assert!(Varlink::from_string("com.-example.leadinghyphen {F()->()}").is_err());
    assert!(Varlink::from_string("com.example-.danglinghyphen- {F()->()}").is_err());
    assert!(Varlink::from_string("Com.example.uppercase-toplevel {F()->()}").is_err());
    assert!(Varlink::from_string("Co9.example.number-toplevel {F()->()}").is_err());
    assert!(Varlink::from_string("1om.example.number-toplevel {F()->()}").is_err());
    assert!(Varlink::from_string("com.Example {F()->()}").is_err());
}

#[test]
fn test_no_method() {
    assert!(Varlink::from_string("
org.varlink.service {
  type Interface (name: string, types: Type[], methods: Method[])
  type Property (key: string, value: string)
}
")
        .is_err());
}

#[test]
fn test_type_no_args() {
    assert!(Varlink::from_string("foo.bar{ type I () F()->() }").is_ok());
}

#[test]
fn test_type_one_arg() {
    assert!(Varlink::from_string("foo.bar{ type I (b:bool) F()->() }").is_ok());
}

#[test]
fn test_default_values() {
    assert!(Varlink::from_string("foo.bar{ type I (b:bool = true) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:int8 = 127) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:int8 = -127) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:int8 = 1-27) F()->() }").is_err());
    assert!(Varlink::from_string("foo.bar{ type I (b:int8 = 128) F()->() }").is_err());
    assert!(Varlink::from_string("foo.bar{ type I (b:uint8 = 255) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:uint8 = 256) F()->() }").is_err());
    assert!(Varlink::from_string("foo.bar{ type I (b:float32 = 1.0) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:float32 = +1.0e10) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:float32 = -1.0e10) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:float64 = +1.0e10) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:float64 = -1.0e10) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:string = \"drgjdkhg\") F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:string = \"dr\\\"gj\\\"dkhg\") F()->() }")
        .is_ok());
}

#[test]
fn test_type_one_array() {
    assert!(Varlink::from_string("foo.bar{ type I (b:bool[]) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:bool[ ]) F()->() }").is_err());
    let ifaces = Varlink::from_string("foo.bar{ type I (b:bool[ ]) F()->() }");
    assert_eq!(ifaces.err().unwrap().to_string(),
               "error at 1:25: expected `[0-9]`");
    assert!(Varlink::from_string("foo.bar{ type I (b:bool[1]) F()->() }").is_ok());
    assert!(Varlink::from_string("foo.bar{ type I (b:bool[ 1 ]) F()->() }").is_err());
    assert!(Varlink::from_string("foo.bar{ type I (b:bool[ 1 1 ]) F()->() }").is_err());
}

#[test]
fn test_method_struct_optional() {
    assert!(Varlink::from_string("foo.bar{ Foo(foo: (i: int64, b: bool)? )->()}").is_ok());
}

#[test]
fn test_method_struct_array_optional() {
    assert!(Varlink::from_string("foo.bar{ Foo(foo: (i: int64, b: bool)[]? )->()}").is_ok());
}

#[test]
fn test_method_struct_array_optional_wrong() {
    assert!(Varlink::from_string("foo.bar{ Foo(foo: (i: int64, b: bool)?[]) -> ()}").is_err());
}

#[test]
fn test_format() {
    let v = Varlink::from_string("foo.bar{ type I (b:bool[18446744073709551615]) F()->()}")
        .unwrap();
    assert_eq!(v.interfaces["foo.bar"].to_string(),
               "\
foo.bar {
  type I (b: bool[18446744073709551615]);
  F() -> ();
}
");
}

#[test]
fn test_max_array_size() {
    let v = Varlink::from_string("foo.bar{ type I (b:bool[18446744073709551616]) F()->()}");
    assert!(v.is_err());
    assert_eq!(v.err().unwrap(),
               "error at 1:46: expected `number 1..18446744073709551615`");
}

#[test]
fn test_union() {
    let v = Varlink::from_string("
    foo.bar{ F()->(s: (a: bool, b: int64), u: bool|int64|(foo: bool, bar: bool))}")
        .unwrap();
    assert_eq!(v.interfaces["foo.bar"].to_string(),
               "\
foo.bar {
  F() -> (s: (a: bool, b: int64), u: bool | int64 | (foo: bool, bar: \
                bool));
}
");
}

#[test]
fn test_duplicate() {
    let e = Varlink::from_string("
foo.example {
	type Device()
	type Device()
	type T()
	type T()
	F() -> ()
}

foo.example {
    F() -> ()
    F() -> ()
	type T()
	type T()
}

foo.example {
    E() -> ()
}

bar.example {
    F() -> ()
    F() -> ()
}

")
        .err()
        .unwrap();
    assert_eq!(e,
               "\
Interface `bar.example`: multiple definitions of type `F`!
Interface `foo.example`: multiple definitions of type `Device`!
Interface `foo.example`: multiple definitions of type `F`!
Interface `foo.example`: multiple definitions of type `T`!
Multiple definitions of interface `foo.example`!");
}
