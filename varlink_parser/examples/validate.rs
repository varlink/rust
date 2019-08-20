extern crate varlink_parser;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use std::result::Result;
use varlink_parser::{FormatColored, IDL};

fn main() -> Result<(), Box<dyn Error>> {
    let mut buffer = String::new();
    let args: Vec<_> = env::args().collect();

    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
        _ => File::open(Path::new(&args[1]))?.read_to_string(&mut buffer)?,
    };

    let v = IDL::from_string(&buffer)?;
    println!("{}", v.get_multiline_colored(0, 80));
    exit(0);
}
