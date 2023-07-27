use crate::*;

#[test]
fn test_standard() {
    let v = IDL::try_from(
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
    )
    .unwrap();
    assert_eq!(v.name, "org.varlink.service");
    assert_eq!(
        v.doc,
        "\
         # The Varlink Service Interface is provided by every varlink service. It\n\
         # describes the service and the interfaces it implements.\
         "
    );
    assert_eq!(
        v.methods.get("GetInterfaceDescription").unwrap().doc,
        "# Get the description of an interface that is implemented by this service."
    );
    //println!("{}", v.to_string());
    assert_eq!(
        v.to_string(),
        "\
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
"
    );
}

#[test]
fn test_complex() {
    let v = IDL::try_from(
        "interface org.example.complex
type TypeEnum ( a, b, c )

type TypeFoo (
bool: bool,
int: int,
float: float,
string: string,
array: ?[]string,
enum: ( foo, bar, baz ),
type: TypeEnum,
anon: ( foo: bool, bar: int, baz: ( a: int, b: int) )
)

method Foo(a: (b: bool, c: int), foo: TypeFoo) -> (a: (b: bool, c: int), foo: TypeFoo)

error ErrorFoo (a: (b: bool, c: int), foo: TypeFoo)
",
    )
    .unwrap();
    assert_eq!(v.name, "org.example.complex");
    println!("{}", v);
    assert_eq!(
        v.to_string(),
        "\
interface org.example.complex

type TypeEnum (a, b, c)

type TypeFoo (
  bool: bool,
  int: int,
  float: float,
  string: string,
  array: ?[]string,
  enum: (foo, bar, baz),
  type: TypeEnum,
  anon: (foo: bool, bar: int, baz: (a: int, b: int))
)

method Foo(a: (b: bool, c: int), foo: TypeFoo) -> (
  a: (b: bool, c: int),
  foo: TypeFoo
)

error ErrorFoo (a: (b: bool, c: int), foo: TypeFoo)
"
    );
}

#[test]
fn test_formatted() {
    let v = IDL::try_from(
        "\
# 345678901234567890123456789012345678901234567890123456789012345678901234567890
interface org.example.format

type TypeFoo (enum: (foo, bar, sdfsdfsdfsdf, sdfsdfsefaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa),
enum2: (foo, bar, sdfsdfsdfsdf, sdfsdfsefaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa),
anon: (baz: (a: (foo: bool, bar: int, baz: (a: int, b: int), baz1: (a: int, bee: int)),
b: (foo: bool, bar: int, baz: (a: int, b: int), baz1: (a: int, beee: int)))))

method Foo(a: (b: bool, c: int), foo: bool) -> (a: (b: bool, c: int), foo: bool)

error ErrorFoo (a: (foo: bool, bar: int, baz: (a: int, b: int), b: (beee: int)))

error ErrorFoo1 (a: (foo: bool, bar: int, baz: (a: int, b: int), b: (beee: int)))
",
    )
    .unwrap();
    assert_eq!(v.name, "org.example.format");
    println!("{}", v.get_oneline());
    println!("{}", v);
    assert_eq!(
        v.to_string(),
        "\
# 345678901234567890123456789012345678901234567890123456789012345678901234567890
interface org.example.format

type TypeFoo (
  enum: (foo, bar, sdfsdfsdfsdf, sdfsdfsefaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa),
  enum2: (
    foo,
    bar,
    sdfsdfsdfsdf,
    sdfsdfsefaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  ),
  anon: (
    baz: (
      a: (foo: bool, bar: int, baz: (a: int, b: int), baz1: (a: int, bee: int)),
      b: (
        foo: bool,
        bar: int,
        baz: (a: int, b: int),
        baz1: (a: int, beee: int)
      )
    )
  )
)

method Foo(a: (b: bool, c: int), foo: bool) -> (a: (b: bool, c: int), foo: bool)

error ErrorFoo (a: (foo: bool, bar: int, baz: (a: int, b: int), b: (beee: int)))

error ErrorFoo1 (
  a: (foo: bool, bar: int, baz: (a: int, b: int), b: (beee: int))
)
"
    );
}

#[test]
fn test_one_method() {
    let v = IDL::try_from("interface foo.bar\nmethod Foo()->()");
    assert!(v.is_ok());
}

#[test]
fn test_one_method_no_type() {
    assert!(ParseInterface("interface foo.bar\nmethod Foo()->(b:)").is_err());
}

#[test]
fn test_domainnames() {
    assert!(IDL::try_from("interface org.varlink.service\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface com.example.0example\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface com.example.example-dash\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface com.-example.leadinghyphen\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface xn--lgbbat1ad8j.example.algeria\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface com.example-.danglinghyphen-\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface 1om.example.number-toplevel\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface com.Example\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface Com.example.uppercase-toplevel\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a.b\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a.b.c\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface co9.example.number-toplevel\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a1.b1.c1\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a1.b--1.c--1\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a--1.b--1.c--1\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a.21.c\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a.1\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface a.0.0\nmethod F()->()").is_ok());
    assert!(IDL::try_from("interface ab\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface .a.b.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.b.c.\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a..b.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface 1.b.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface 8a.0.0\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface -a.b.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.b.c-\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.b-.c-\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.-b.c-\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.-.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.*.c\nmethod F()->()").is_err());
    assert!(IDL::try_from("interface a.?\nmethod F()->()").is_err());
}

#[test]
fn test_type_no_args() {
    assert!(IDL::try_from("interface foo.bar\n type I ()\nmethod F()->()").is_ok());
}

#[test]
fn test_type_one_arg() {
    assert!(IDL::try_from("interface foo.bar\n type I (b:bool)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_enum() {
    assert!(
        IDL::try_from("interface foo.bar\n type I (b: (foo, bar, baz))\nmethod F()->()").is_ok()
    );
}

#[test]
fn test_type_string() {
    assert!(IDL::try_from("interface foo.bar\n type I (b: string)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_stringmap() {
    assert!(
        IDL::try_from("interface foo.bar\n type I (b: [string]string)\nmethod F()->()").is_ok()
    );
}

#[test]
fn test_type_stringmap_set() {
    assert!(IDL::try_from("interface foo.bar\n type I (b: [string]())\nmethod F()->()").is_ok());
}

#[test]
fn test_type_object() {
    assert!(IDL::try_from("interface foo.bar\n type I (b: object)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_int() {
    assert!(IDL::try_from("interface foo.bar\n type I (b: int)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_float() {
    assert!(IDL::try_from("interface foo.bar\n type I (b: float)\nmethod F()->()").is_ok());
}

#[test]
fn test_type_one_array() {
    assert!(IDL::try_from("interface foo.bar\n type I (b:[]bool)\nmethod  F()->()").is_ok());
    assert!(IDL::try_from("interface foo.bar\n type I (b:bool[ ])\nmethod  F()->()").is_err());
    assert!(IDL::try_from("interface foo.bar\n type I (b:[ ]bool)\nmethod  F()->()").is_err());
    assert!(IDL::try_from("interface foo.bar\n type I (b:[1]bool)\nmethod  F()->()").is_err());
    assert!(IDL::try_from("interface foo.bar\n type I (b:[ 1 ]bool)\nmethod  F()->()").is_err());
    assert!(IDL::try_from("interface foo.bar\n type I (b:[ 1 1 ]bool)\nmethod  F()->()").is_err());
}

#[test]
fn test_format() {
    let v = IDL::try_from("interface foo.bar\ntype I(b:[]bool)\nmethod  F()->()").unwrap();
    assert_eq!(
        v.to_string(),
        "interface foo.bar\n\ntype I (b: []bool)\n\nmethod F() -> ()\n"
    );
}

#[test]
fn test_duplicate() {
    let e = IDL::try_from(
        "
interface foo.example
type Device()
type Device()
type T()
type T()
method F() -> ()
method F() -> ()
",
    )
    .err()
    .unwrap();
    assert_eq!(
        e.to_string(),
        "Interface definition error: '\
Interface `foo.example`: multiple definitions of method `F`!
Interface `foo.example`: multiple definitions of type `Device`!
Interface `foo.example`: multiple definitions of type `T`!'
"
    );
}
