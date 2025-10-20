//! Macros for generating modules from a varlink interface definition
//!
//! This crate provides two procedural macros for generating a Rust module out of an interface
//! definition:
//!
//! - The [`varlink!`] macro takes as argument the interface as a string literal.
//! - The [`varlink_file!`] macro takes as argument the path to the varlink interface definition
//!   _relative_ to the directory containing the manifest of your package.
//!
//! They have the drawback that most IDEs don't execute this and thus
//! offer no code completion.
//!
//! # Examples
//!
//! ```rust,no_run
//! use varlink_derive;
//! extern crate serde_derive;
//!
//! varlink_derive::varlink!(org_example_ping, r#"
//! ## Example service
//! interface org.example.ping
//!
//! ## Returns the same string
//! method Ping(ping: string) -> (pong: string)
//! "#);
//!
//! use org_example_ping::VarlinkClientInterface;
//! /* ... */
//! ```
//!
//! ```rust,no_run
//! use varlink_derive;
//! extern crate serde_derive;
//!
//! varlink_derive::varlink_file!(
//!    org_example_network,
//!    "../examples/example/src/org.example.network.varlink"
//! );
//!
//! use org_example_network::VarlinkClientInterface;
//! /* ... */
//! ```

extern crate proc_macro;
extern crate varlink_generator;

use proc_macro::{Span, TokenStream, TokenTree};
use std::io::Read;

/// Generates a module from a varlink interface definition
///
/// # Usage
///
/// The macro takes two arguments:
///
/// 1. The module name that will be generated. It must be a valid Rust identifier.
/// 2. A string literal containing the the varlink interface definition.
///
/// # Examples
///
/// ```rust,no_run
/// use varlink_derive;
/// extern crate serde_derive;
///
/// varlink_derive::varlink!(org_example_ping, r#"
/// ## Example service
/// interface org.example.ping
///
/// ## Returns the same string
/// method Ping(ping: string) -> (pong: string)
/// "#);
///
/// use org_example_ping::VarlinkClientInterface;
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink(input: TokenStream) -> TokenStream {
    let (name, source, _) = parse_varlink_args(input);
    expand_varlink(name, source)
}

/// Generates a module from a varlink interface definition file
///
/// # Usage
///
/// The macro takes two arguments:
///
/// 1. The module name that will be generated. It must be a valid Rust identifier.
/// 2. A string literal containing the file path of the varlink interface definition. The path
///    **must** be relative to the directory containing the manifest of your package.
///
/// # Examples
///
/// ```rust,no_run
/// use varlink_derive;
/// extern crate serde_derive;
///
/// varlink_derive::varlink_file!(
///    org_example_network,
///    "../examples/example/src/org.example.network.varlink"
///);
///
/// use org_example_network::VarlinkClientInterface;
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink_file(input: TokenStream) -> TokenStream {
    let (name, filename, _) = parse_varlink_filename_args(input);
    let mut source = Vec::<u8>::new();

    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        std::borrow::Cow::Owned(std::path::Path::new(&manifest_dir).join(filename))
    } else {
        std::borrow::Cow::Borrowed(std::path::Path::new(&filename))
    };

    std::fs::File::open(&path)
        .unwrap_or_else(|err| {
            panic!(
                "varlink_file! expansion failed. Could not open file {}: {}",
                path.display(),
                err
            )
        })
        .read_to_end(&mut source)
        .unwrap_or_else(|err| {
            panic!(
                "varlink_file! expansion failed. Could not read file {}: {}",
                path.display(),
                err
            )
        });

    expand_varlink(name, String::from_utf8_lossy(&source).to_string())
}

/// Generates a module with async client support from a varlink interface definition
///
/// # Usage
///
/// The macro takes two arguments:
///
/// 1. The module name that will be generated. It must be a valid Rust identifier.
/// 2. A string literal containing the the varlink interface definition.
///
/// This macro generates ONLY async client code (exclusive mode). The generated code requires
/// the `tokio` feature to be enabled in the varlink crate. For sync code, use `varlink!` instead.
///
/// # Examples
///
/// ```rust,ignore
/// use varlink_derive;
/// extern crate serde_derive;
///
/// varlink_derive::varlink_async!(org_example_ping, r#"
/// ## Example service
/// interface org.example.ping
///
/// ## Returns the same string
/// method Ping(ping: string) -> (pong: string)
/// "#);
///
/// use org_example_ping::VarlinkClientInterface;
/// // Uses async client API with .call().await
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink_async(input: TokenStream) -> TokenStream {
    let (name, source, _) = parse_varlink_args(input);
    expand_varlink_async(name, source)
}

/// Generates a module with async client support from a varlink interface definition file
///
/// # Usage
///
/// The macro takes two arguments:
///
/// 1. The module name that will be generated. It must be a valid Rust identifier.
/// 2. A string literal containing the file path of the varlink interface definition. The path
///    **must** be relative to the directory containing the manifest of your package.
///
/// This macro generates ONLY async client code (exclusive mode). The generated code requires
/// the `tokio` feature to be enabled in the varlink crate. For sync code, use `varlink_file!` instead.
///
/// # Examples
///
/// ```rust,ignore
/// use varlink_derive;
/// extern crate serde_derive;
///
/// varlink_derive::varlink_file_async!(
///    org_example_network,
///    "../examples/example/src/org.example.network.varlink"
///);
///
/// use org_example_network::VarlinkClientInterface;
/// // Uses async client API with .call().await
/// /* ... */
/// ```
#[proc_macro]
pub fn varlink_file_async(input: TokenStream) -> TokenStream {
    let (name, filename, _) = parse_varlink_filename_args(input);
    let mut source = Vec::<u8>::new();

    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        std::borrow::Cow::Owned(std::path::Path::new(&manifest_dir).join(filename))
    } else {
        std::borrow::Cow::Borrowed(std::path::Path::new(&filename))
    };

    std::fs::File::open(&path)
        .unwrap_or_else(|err| {
            panic!(
                "varlink_file_async! expansion failed. Could not open file {}: {}",
                path.display(),
                err
            )
        })
        .read_to_end(&mut source)
        .unwrap_or_else(|err| {
            panic!(
                "varlink_file_async! expansion failed. Could not read file {}: {}",
                path.display(),
                err
            )
        });

    expand_varlink_async(name, String::from_utf8_lossy(&source).to_string())
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
    if !body_literal.starts_with('\"') || !body_literal.ends_with('\"') {
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
    let code = varlink_generator::compile(source).unwrap();

    format!("mod {} {{ {} }}", name, code).parse().unwrap()
}

fn expand_varlink_async(name: String, source: String) -> TokenStream {
    let code = varlink_generator::compile_with_options(
        source,
        &varlink_generator::GeneratorOptions {
            generate_async: true,
            ..Default::default()
        },
    )
    .unwrap();

    format!("mod {} {{ {} }}", name, code).parse().unwrap()
}
