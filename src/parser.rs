#![allow(dead_code)]

mod varlink_grammar {

    pub enum VType<'a> {
        Bool,
        Int8,
        UInt8,
        Int16,
        UInt16,
        Int32,
        UInt32,
        Int64,
        UInt64,
        Float32,
        Float64,
        VString,
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
        pub methods: Vec<Method<'a>>,
        pub typedefs: Vec<Typedef<'a>>,
    }

    use std::fmt;

    impl<'a> fmt::Display for VType<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                VType::Bool => write!(f, "bool"),
                VType::Int8 => write!(f, "int8"),
                VType::UInt8 => write!(f, "uint8"),
                VType::Int16 => write!(f, "int16"),
                VType::UInt16 => write!(f, "uint16"),
                VType::Int32 => write!(f, "int32"),
                VType::UInt32 => write!(f, "uint32"),
                VType::Int64 => write!(f, "int64"),
                VType::UInt64 => write!(f, "uint64"),
                VType::Float32 => write!(f, "float32"),
                VType::Float64 => write!(f, "float64"),
                VType::VString => write!(f, "string"),
                VType::VTypename(ref s) => write!(f, "{}", s),
                VType::VStruct(ref v) => write!(f, "{}", v),
            }
        }
    }

    impl<'a> fmt::Display for VTypeExt<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.vtype)?;
            if let Some(t) = self.isarray {
                match t {
                    0 => write!(f, "[]")?,
                    _ => write!(f, "[{}]", t)?,
                }
            }
            if self.nullable {
                write!(f, "?")
            } else {
                Ok(())
            }
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
            for t in &self.typedefs {
                write!(f, "  type {} {};\n", t.name, t.vstruct)?;
            }
            for m in &self.methods {
                write!(f, "  {}{} -> {};\n", m.name, m.input, m.output)?;
            }
            write!(f, "}}\n")
        }
    }

    impl<'a> Interface<'a> {
        fn from_token(n: &'a str,
                      ts: Vec<Typedef<'a>>,
                      m: Method<'a>,
                      mt: Vec<MethodOrTypedef<'a>>)
                      -> Interface<'a> {
            let mut i = Interface {
                name: n,
                methods: Vec::new(),
                typedefs: Vec::new(),
            };

            i.methods.push(m);

            for o in mt {
                match o {
                    MethodOrTypedef::Method(m) => i.methods.push(m),
                    MethodOrTypedef::Typedef(t) => i.typedefs.push(t),
                };
            }

            for t in ts {
                i.typedefs.push(t);
            }

            return i;
        }
    }

    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
}

#[cfg(test)]
use self::varlink_grammar::*;

#[test]
fn test_standard() {
    let ifaces = interfaces("
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
    assert_eq!(ifaces[0].name, "org.varlink.service");
    assert_eq!(ifaces[0].to_string(),
               "org.varlink.service {
  type Type (name: string, typestring: string);
  type \
                Method (name: string, monitor: bool, type_in: string, type_out: string);
  type \
                Interface (name: string, types: Type[], methods: Method[]);
  type Property \
                (key: string, value: string);
  type InterfaceDescription (description: string, \
                types: string[], methods: string[]);
  Introspect(version: uint64) -> (name: \
                string, interfaces: Interface[]);
  Help() -> (description: string, properties: \
                Property[], interfaces: InterfaceDescription[]);
}
");

}

#[test]
fn test_one_method() {
    let ifaces = interfaces("/* comment */ foo.bar{ Foo()->() }").unwrap();
    assert!(ifaces[0].methods[0].stream == false);
}

#[test]
fn test_one_method_no_type() {
    assert!(interfaces("foo.bar{ Foo()->(b:) }").is_err());
}

#[test]
fn test_one_method_stream() {
    let ifaces = interfaces("foo.bar{ Foo()=>() }").unwrap();
    assert!(ifaces[0].methods[0].stream);
}

#[test]
fn test_domainnames() {
    assert!(interfaces("org.varlink.service {F()->()}").is_ok());
    assert!(interfaces("com.example.0example {F()->()}").is_ok());
    assert!(interfaces("com.example.example-dash {F()->()}").is_ok());
    assert!(interfaces("xn--lgbbat1ad8j.example.algeria {F()->()}").is_ok());
    assert!(interfaces("com.-example.leadinghyphen {F()->()}").is_err());
    assert!(interfaces("com.example-.danglinghyphen- {F()->()}").is_err());
    assert!(interfaces("Com.example.uppercase-toplevel {F()->()}").is_err());
    assert!(interfaces("Co9.exmaple.number-toplevel {F()->()}").is_err());
    assert!(interfaces("com.Example {F()->()}").is_err());
}

#[test]
fn test_no_method() {
    assert!(interfaces("
org.varlink.service {
  type Interface (name: string, types: Type[], methods: Method[])
  type Property (key: string, value: string)
}
")
        .is_err());
}

#[test]
fn test_type_no_args() {
    assert!(interfaces("foo.bar{ type I () F()->() }").is_ok());
}

#[test]
fn test_type_one_arg() {
    assert!(interfaces("foo.bar{ type I (b:bool) F()->() }").is_ok());
}

#[test]
fn test_type_one_array() {
    assert!(interfaces("foo.bar{ type I (b:bool[]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[ ]) F()->() }").is_err());
    assert!(interfaces("foo.bar{ type I (b:bool[1]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[ 1 ]) F()->() }").is_err());
    assert!(interfaces("foo.bar{ type I (b:bool[ 1 1 ]) F()->() }").is_err());
}

#[test]
fn test_method_struct_optional() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)? )->()}").is_ok());
}

#[test]
fn test_method_struct_array_optional() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)[]? )->()}").is_ok());
}

#[test]
fn test_method_struct_array_optional_wrong() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)?[]) -> ()}").is_err());
}

#[test]
fn test_format() {
    let i = interfaces("foo.bar{ type I (b:bool[18446744073709551615]) F()->() }").unwrap();
    assert_eq!(i[0].to_string(),
               "foo.bar {
  type I (b: bool[18446744073709551615]);
  F() -> ();
}
");
}

#[test]
fn test_union() {
    let i = interfaces("foo.bar{ F()->(s: (a: bool, b: int64), u: bool|int64|(foo: bool, bar: \
                        bool)) }")
        .unwrap();
    println!("{}", i[0]);
    assert_eq!(i[0].to_string(),
               "foo.bar {
  F() -> (s: (a: bool, b: int64), u: bool | int64 | (foo: bool, bar: \
                bool));
}
");
}

#[test]
fn print_size() {
    use std::mem::size_of;
    println!("Sizeof enum VType {}", size_of::<VType>());
    println!("usize max {}", usize::max_value());
}
