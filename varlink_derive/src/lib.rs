//! Macro for generating modules from a varlink interface definition
//!
//! It has the drawback, that most IDEs don't execute this and thus
//! offer no code completion.
//!
//! Examples:
//!
//! ```rust,ignore
//! use varlink_derive;
//!
//! varlink_derive::varlink!(org_example_ping, r#"
//! # Example service
//! interface org.example.ping
//!
//! # Returns the same string
//! method Ping(ping: string) -> (pong: string)
//! "#);
//!
//! use crate::org_example_ping::VarlinkClientInterface;
//! /* ... */
//! ```
//!
//! ```rust,ignore
//! use varlink_derive;
//!
//! varlink_derive::varlink_file!(
//!    org_example_network,
//!    "examples/example/src/org.example.network.varlink"
//! );
//!
//! use crate::org_example_network::VarlinkClientInterface;
//! /* ... */
//! ```

extern crate proc_macro;
extern crate varlink_generator;

use proc_macro::{Span, TokenStream, TokenTree};
use std::io::Read;

/// Generates a module from a varlink interface definition
///
/// `varlink!(<modulename>, r#"<varlink interface definition>"#)`
///
/// Examples:
///
/// ```rust,ignore
/// use varlink_derive;
//
/// varlink_derive::varlink!(org_example_ping, r#"
/// # Example service
/// interface org.example.ping
///
/// # Returns the same string
/// method Ping(ping: string) -> (pong: string)
/// "#);
///
/// use crate::org_example_ping::VarlinkClientInterface;
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink(input: TokenStream) -> TokenStream {
    let (name, source, _) = parse_varlink_args(input);
    expand_varlink(name, source)
}

/// Generates a module from a varlink interface definition file
///
/// `varlink!(<modulename>, "<varlink interface definition file path relative to the workspace>")`
///
/// Examples:
///
/// ```rust,ignore
/// use varlink_derive;
///
/// varlink_derive::varlink_file!(
///    org_example_network,
///    "examples/example/src/org.example.network.varlink"
///);
///
/// use crate::org_example_network::VarlinkClientInterface;
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink_file(input: TokenStream) -> TokenStream {
    let (name, filename, _) = parse_varlink_filename_args(input);
    let mut source = Vec::<u8>::new();
    std::fs::File::open(filename)
        .unwrap()
        .read_to_end(&mut source)
        .unwrap();
    expand_varlink(name, String::from_utf8_lossy(&source).to_string())
}

// Parse a TokenStream of the form `name r#""#`
fn parse_varlink_filename_args(input: TokenStream) -> (String, String, Span) {
    let mut iter = input.into_iter();
    let name = match iter.next() {
        Some(TokenTree::Ident(i)) => i.to_string(),
        Some(other) => panic!("Expected module name, found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    match iter.next() {
        Some(TokenTree::Punct(ref p)) if p.as_char() == ',' => {}
        Some(other) => panic!("Expected ',', found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    let (body_literal, span) = match iter.next() {
        Some(TokenTree::Literal(l)) => (l.to_string(), l.span()),
        Some(other) => panic!("Expected raw string literal, found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    if !body_literal.starts_with("\"") || !body_literal.ends_with("\"") {
        panic!("Expected raw string literal (`r#\"...\"#`)");
    }
    let body_string = body_literal[1..body_literal.len() - 1].to_string();
    match iter.next() {
        None => {}
        Some(_) => panic!("Unexpected trailing tokens in macro"),
    }
    (name, body_string, span)
}

// Parse a TokenStream of the form `name r#""#`
fn parse_varlink_args(input: TokenStream) -> (String, String, Span) {
    let mut iter = input.into_iter();
    let name = match iter.next() {
        Some(TokenTree::Ident(i)) => i.to_string(),
        Some(other) => panic!("Expected module name, found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    match iter.next() {
        Some(TokenTree::Punct(ref p)) if p.as_char() == ',' => {}
        Some(other) => panic!("Expected ',', found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    let (body_literal, span) = match iter.next() {
        Some(TokenTree::Literal(l)) => (l.to_string(), l.span()),
        Some(other) => panic!("Expected raw string literal, found {}", other),
        None => panic!("Unexpected end of macro input"),
    };
    if !body_literal.starts_with("r#\"") || !body_literal.ends_with("\"#") {
        panic!("Expected raw string literal (`r#\"...\"#`)");
    }
    let body_string = body_literal[3..body_literal.len() - 2].to_string();
    match iter.next() {
        None => {}
        Some(_) => panic!("Unexpected trailing tokens in macro"),
    }
    (name, body_string, span)
}

fn expand_varlink(name: String, source: String) -> TokenStream {
    let code = match varlink_generator::compile(source) {
        Ok(code) => code,
        Err(e) => {
            let mut s = String::new();
            for i in e.iter() {
                s += &i.to_string();
                s += "\n";
            }
            panic!(s)
        }
    };

    format!("mod {} {{ {} }}", name, code).parse().unwrap()
}
