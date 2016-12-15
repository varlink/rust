#![allow(dead_code)]

mod varlink_grammar {
    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
}

#[cfg(test)]
use self::varlink_grammar::*;

#[test]
fn test_standard() {
    assert!(interfaces("
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

  Help() -> (
    description: string,
    properties: Property[],
    interfaces: InterfaceDescription[]
  )
}
")
        .is_ok());
}

#[test]
fn test_one_method() {
    assert!(interfaces("foo.bar{ Foo()->() }").is_ok());
}

#[test]
fn test_one_method_no_type() {
    assert!(interfaces("foo.bar{ Foo()->(b:) }").is_err());
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
    assert!(interfaces("foo.bar{ type I () F()->() }").is_err());
}

#[test]
fn test_type_one_arg() {
    assert!(interfaces("foo.bar{ type I (b:bool) F()->() }").is_ok());
}

#[test]
fn test_type_one_array() {
    assert!(interfaces("foo.bar{ type I (b:bool[]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[ ]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[1]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[ 1 ]) F()->() }").is_ok());
    assert!(interfaces("foo.bar{ type I (b:bool[ 1 1 ]) F()->() }").is_err());
}

// REALLY???
#[test]
fn test_method_struct_optional() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)? )->()}").is_ok());
}

// REALLY???
#[test]
fn test_method_struct_array_optional() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)[]? )->()}").is_ok());
}

#[test]
fn test_method_struct_array_optional_wrong() {
    assert!(interfaces("foo.bar{ Foo(foo: (i: int64, b: bool)?[]) -> ()}").is_err());
}
