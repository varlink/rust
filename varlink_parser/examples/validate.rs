#[macro_use]
extern crate error_chain;
extern crate varlink_parser;

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use varlink_parser::Varlink;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    links {
        Parser(self::varlink_parser::Error, self::varlink_parser::ErrorKind);
    }
}

fn main() -> Result<()> {
    let mut buffer = String::new();
    let args: Vec<_> = env::args().collect();

    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
        _ => File::open(Path::new(&args[1]))?.read_to_string(&mut buffer)?,
    };

    let v = Varlink::from_string(&buffer)?;
    println!("{}", v.interface);
    exit(0);
}
