extern crate varlink_parser;

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use varlink_parser::{FormatColored, Varlink};
use std::result::Result;
use std::error::Error;

fn main() ->  Result<(), Box<Error>> {
    let mut buffer = String::new();
    let args: Vec<_> = env::args().collect();

    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
        _ => File::open(Path::new(&args[1]))?.read_to_string(&mut buffer)?,
    };

    let v = Varlink::from_string(&buffer)?;
    println!("{}", v.interface.get_multiline_colored(0, 80));
    exit(0);
}
