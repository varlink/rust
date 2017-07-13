extern crate varlink;

use std::io;
use std::io::prelude::*;
use varlink::parser::Varlink;
use std::process::exit;
use std::path::Path;
use std::fs::File;
use std::env;
use std::io::{Error as IOError, ErrorKind};
use std::error::Error;
use std::result::Result;
use varlink::parser::VType;

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
    let mut buffer = String::new();

    let args: Vec<_> = env::args().collect();
    match args.len() {
        0 => io::stdin().read_to_string(&mut buffer)?,
        _ => {
            File::open(Path::new(&args[1]))?
                .read_to_string(&mut buffer)?
        }
    };

    let vr = Varlink::from_string(&buffer);

    if let Err(e) = vr {
        println!("{}", e);
        exit(1);
    }

    let v = vr.unwrap();

    for t in v.interface.typedefs.values() {
        println!("#[derive(Serialize, Deserialize, Debug)]");
        println!("pub struct {} {{", t.name);
        for e in t.vstruct.elts.iter() {
            let v = &e.vtypes[0];
            println!("    pub {} : {};", e.name, match v.vtype {
                VType::Bool(_) => "bool",
                VType::Int(_) => "i64",
                VType::Float(_) => "f64",
                VType::VString(_) => "String",
                VType::VData(_) => "String",
                VType::VTypename(ref v) => v,
                VType::VStruct(_) => return Err(IOError::new(ErrorKind::Other, "oh no!")),

            });
        }
        println!("}}\n");
    }

    Ok(())
}

fn main() {
    exit(do_main().into_error_code());
}
