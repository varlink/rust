extern crate varlink;

use std::io;
use std::io::prelude::*;
use varlink::parser::Varlink;
use std::process::exit;
use std::path::Path;
use std::fs::File;
use std::env;

use std::io::Error as IOError;
use std::error::Error;

use std::result::Result;
use varlink::parser::*;
use std::fmt;

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

trait ToRust {
    fn to_rust(&self) -> Result<String, ToRustError>;
}

#[derive(Debug)]
enum ToRustError {
    BadStruct,
    IoError(IOError),
}

impl Error for ToRustError {
    fn description(&self) -> &str {
        match *self {
            ToRustError::BadStruct => "Anonymous struct",
            ToRustError::IoError(_) => "an I/O error occurred",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &ToRustError::IoError(ref err) => Some(&*err as &Error),
            _ => None,
        }
    }
}

impl From<IOError> for ToRustError {
    fn from(err: IOError) -> ToRustError {
        ToRustError::IoError(err)
    }
}


impl fmt::Display for ToRustError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())?;
        Ok(())
    }
}

impl<'a> ToRust for VType<'a> {
    fn to_rust(&self) -> Result<String, ToRustError> {
        match *self {
            VType::Bool(_) => Ok("bool".into()),
            VType::Int(_) => Ok("i64".into()),
            VType::Float(_) => Ok("f64".into()),
            VType::VString(_) => Ok("String".into()),
            VType::VData(_) => Ok("String".into()),
            VType::VTypename(v) => Ok(v.into()),
            VType::VStruct(_) => Err(ToRustError::BadStruct),
        }
    }
}

impl<'a> ToRust for VTypeExt<'a> {
    fn to_rust(&self) -> Result<String, ToRustError> {
        let v = self.vtype.to_rust()?;

        if self.isarray {
            Ok(format!("Vec<{}>", v).into())
        } else {
            Ok(v.into())
        }
    }
}

impl<'a> ToRust for Interface<'a> {
    fn to_rust(&self) -> Result<String, ToRustError> {
        let mut out: String = "".to_owned();

        for t in self.typedefs.values() {
            out += "#[derive(Serialize, Deserialize, Debug)]\n";
            out += format!("pub struct {} {{\n", t.name).as_ref();
            for e in &t.vstruct.elts {
                let v = &e.vtypes[0];
                out += format!("    pub {} : {},\n", e.name, v.to_rust()?).as_ref();
            }
            out += "}\n\n";
        }

        for t in self.methods.values() {
            out += "#[derive(Serialize, Deserialize, Debug)]\n";
            out += format!("pub struct {}Reply {{\n", t.name).as_ref();
            for e in &t.output.elts {
                let v = &e.vtypes[0];
                out += format!("    pub {} : {},\n", e.name, v.to_rust()?).as_ref();
            }
            out += "}\n\n";

            if t.input.elts.len() > 0 {
                out += "#[derive(Serialize, Deserialize, Debug)]\n";
                out += format!("pub struct {}Args {{\n", t.name).as_ref();
                for e in &t.input.elts {
                    let v = &e.vtypes[0];
                    out += format!("    pub {} : {},\n", e.name, v.to_rust()?).as_ref();
                }
                out += "}\n\n";
            }
        }
        Ok(out)
    }
}

fn do_main() -> Result<(), ToRustError> {
    let mut buffer = String::new();

    let args: Vec<_> = env::args().collect();
    match args.len() {
        0 | 1 => io::stdin().read_to_string(&mut buffer)?,
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

    println!("{}", vr.unwrap().interface.to_rust()?);

    Ok(())
}

fn main() {
    exit(do_main().into_error_code());
}
