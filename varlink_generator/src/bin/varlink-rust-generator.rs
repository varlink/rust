/*!
 varlink-rust-generator is a CLI, that generates rust code from a varlink
 interface definition file

 # Usage
 $ varlink-rust-generator `[<varlink_file>]`

 If <varlink_file> is omitted, input is expected to come from stdin.

 Output is sent to stdout.
!*/
extern crate varlink_generator;

use std::env;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;

use chainerror::*;
use varlink_generator::{generate, Error, Result};

fn print_usage(program: &str, opts: &getopts::Options) {
    let brief = format!("Usage: {} [VARLINK FILE]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "nosource", "don't print doc header and allow");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            print_usage(&program, &opts);
            return Err(strerr!(Error, "Invalid Arguments"));
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return Ok(());
    }

    let tosource = !matches.opt_present("nosource");

    let mut reader: Box<Read> = match matches.free.len() {
        0 => Box::new(io::stdin()),
        _ => {
            if matches.free[0] == "-" {
                Box::new(io::stdin())
            } else {
                Box::new(
                    File::open(Path::new(&matches.free[0])).map_err(mstrerr!(
                        Error,
                        "Failed to open '{}'",
                        &matches.free[0]
                    ))?,
                )
            }
        }
    };
    let writer: &mut Write = &mut io::stdout();
    generate(&mut reader, writer, tosource)
}
