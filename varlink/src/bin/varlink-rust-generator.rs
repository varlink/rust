//! varlink-rust-generator generates rust code from a varlink interface
//! definition file
//!
//!# Usage
//! $ varlink-rust-generator `[<varlink_file>]`
//!
//! If <varlink_file> is omitted, input is expected to come from stdin.
//!
//! Output is sent to stdout.

extern crate varlink;

use std::env;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use varlink::generator::{generate, Result};

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let mut reader: Box<Read> = match args.len() {
        0 | 1 => Box::new(io::stdin()),
        _ => {
            if args[1] == "-" {
                Box::new(io::stdin())
            } else {
                Box::new(File::open(Path::new(&args[1]))?)
            }
        }
    };
    let writer: &mut Write = &mut io::stdout();
    generate(&mut reader, writer)
}

