extern crate varlink;

use std::io;
use std::io::prelude::*;
use varlink::parser::Varlink;
use std::process::exit;

fn main() {
    let mut buffer = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
        println!("Error {:?}", e);
        exit(1);
    }

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
