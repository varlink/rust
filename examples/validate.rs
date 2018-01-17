extern crate varlink;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Error as IOError;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use std::result::Result;
use varlink::parser::Varlink;

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

fn do_main() -> Result<(), IOError> {
    let mut buffer = String::new();
    let args: Vec<_> = env::args().collect();

    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
        _ => {
            File::open(Path::new(&args[1]))?
                .read_to_string(&mut buffer)?
        }
    };

    match Varlink::from_string(&buffer) {
        Ok(v) => {
            println!("Syntax check passed!\n");
            println!("{}", v.interface);
            exit(0);
        }
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

}

fn main() {
    exit(do_main().into_error_code());
}
