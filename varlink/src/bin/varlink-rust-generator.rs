//! varlink-rust-generator

extern crate varlink;

use std::io;
use std::process::exit;
use std::path::Path;
use std::fs::File;
use std::env;
use std::io::{Read, Write};
use std::error::Error;
use std::result::Result;

use varlink::generator::generate;

trait MainReturn {
    fn into_error_code(self) -> i32;
}

impl<E: Error> MainReturn for Result<(), E> {
    fn into_error_code(self) -> i32 {
        if let Err(e) = self {
            write!(io::stderr(), "{}\n", e).unwrap();
            1
        } else {
            0
        }
    }
}
fn do_main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    {
        let mut reader: Box<Read> = match args.len() {
            0 | 1 => Box::new(io::stdin()),
            _ => Box::new(File::open(Path::new(&args[1]))?),
        };
        let writer: &mut Write = &mut io::stdout();
        generate(&mut reader, writer)
    }
}

fn main() {
    exit(do_main().into_error_code());
}
