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
use std::collections::HashMap;
use std::iter::FromIterator;

type EnumHash<'a> = HashMap<String, Vec<String>>;

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
    fn to_rust(&self, parent: &str, enumhash: &mut EnumHash) -> Result<String, ToRustError>;
}

#[derive(Debug)]
enum ToRustError {
    IoError(IOError),
}

impl Error for ToRustError {
    fn description(&self) -> &str {
        match *self {
            ToRustError::IoError(_) => "an I/O error occurred",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &ToRustError::IoError(ref err) => Some(&*err as &Error),
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
    fn to_rust(&self, parent: &str, enumhash: &mut EnumHash) -> Result<String, ToRustError> {
        match *self {
            VType::Bool(_) => Ok("bool".into()),
            VType::Int(_) => Ok("i64".into()),
            VType::Float(_) => Ok("f64".into()),
            VType::VString(_) => Ok("String".into()),
            VType::VData(_) => Ok("String".into()),
            VType::VTypename(v) => Ok(v.into()),
            VType::VEnum(ref v) => {
                enumhash.insert(parent.into(),
                                Vec::from_iter(v.elts.iter().map(|s| String::from(*s))));
                Ok(format!("{}", parent).into())
            }
            VType::VStruct(_) => Ok(format!("{}", parent).into()),
        }
    }
}

impl<'a> ToRust for VTypeExt<'a> {
    fn to_rust(&self, parent: &str, enumhash: &mut EnumHash) -> Result<String, ToRustError> {
        let v = self.vtype.to_rust(parent, enumhash)?;

        if self.isarray {
            Ok(format!("Vec<{}>", v).into())
        } else {
            Ok(v.into())
        }
    }
}

fn dotted_to_camel_case(s: &str) -> String {
    s.split('.')
        .map(|piece| piece.chars())
        .flat_map(|mut chars| {
                      chars
                          .nth(0)
                          .expect("empty section between dots!")
                          .to_uppercase()
                          .chain(chars)
                  })
        .collect()
}

trait InterfaceToRust {
    fn to_rust(&self, description: &String) -> Result<String, ToRustError>;
}

impl<'a> InterfaceToRust for Interface<'a> {
    fn to_rust(&self, description: &String) -> Result<String, ToRustError> {
        let mut out: String = "".to_owned();
        let mut enumhash = EnumHash::new();

        for t in self.typedefs.values() {
            out += "#[derive(Serialize, Deserialize, Debug)]\n";
            match t.elt {
                VStructOrEnum::VStruct(ref v) => {
                    out += format!("pub struct {} {{\n", t.name).as_ref();
                    for e in &v.elts {
                        out += format!("    pub {}: Option<{}>,\n",
                                       e.name,
                                       e.vtype
                                           .to_rust(format!("{}_{}", t.name, e.name).as_ref(),
                                                    &mut enumhash)?)
                            .as_ref();
                    }
                }
                VStructOrEnum::VEnum(ref v) => {
                    out += format!("pub enum {} {{\n", t.name).as_ref();
                    let mut iter = v.elts.iter();
                    if let Some(fst) = iter.next() {
                        out += format!("    {}", fst).as_ref();
                        for elt in iter {
                            out += format!(",\n    {}", elt).as_ref();
                        }
                    }
                    out += "\n";
                }
            }
            out += "}\n\n";
        }

        for t in self.methods.values() {
            if t.output.elts.len() > 0 {
                out += "#[derive(Serialize, Deserialize, Debug)]\n";
                out += format!("pub struct {}Reply {{\n", t.name).as_ref();
                for e in &t.output.elts {
                    out += format!("    pub {}: Option<{}>,\n",
                                   e.name,
                                   e.vtype.to_rust(self.name, &mut enumhash)?)
                        .as_ref();
                }
                out += "}\n\n";
            }

            if t.input.elts.len() > 0 {
                out += "#[derive(Serialize, Deserialize, Debug)]\n";
                out += format!("pub struct {}Args {{\n", t.name).as_ref();
                for e in &t.input.elts {
                    out += format!("    pub {}: Option<{}>,\n",
                                   e.name,
                                   e.vtype.to_rust(self.name, &mut enumhash)?)
                        .as_ref();
                }
                out += "}\n\n";
            }

        }

        for (name, v) in &enumhash {
            out += format!("pub enum {} {{\n", name).as_ref();
            let mut iter = v.iter();
            if let Some(fst) = iter.next() {
                out += format!("    {}", fst).as_ref();
                for elt in iter {
                    out += format!(",\n    {}", elt).as_ref();
                }
            }
            out += "\n}\n\n";
        }

        out += "pub trait Interface: VarlinkInterface {\n";
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(", {} : {}",
                                       e.name,
                                       e.vtype.to_rust(self.name, &mut enumhash)?)
                        .as_ref();
                }
            }
            let mut c = t.name.chars();
            let fname = match c.next() {
                None => String::from(t.name),
                Some(f) => f.to_lowercase().chain(c).collect(),
            };

            out += format!("    fn {}(&self{}) -> Result<{}Reply, Error>;\n",
                           fname,
                           inparms,
                           t.name)
                .as_ref();
        }
        out += "}\n\n";

        out += format!(
            r####"
#[macro_export]
macro_rules! {} {{
	(
		()
		$(pub)* struct $name:ident $($_tail:tt)*
	) => {{

impl varlink::server::Interface for $name {{
    fn get_description(&self) -> &'static str {{
        r#"
{}
"#
    }}

    fn get_name(&self) -> &'static str {{
        "{}"
    }}

"####,
            dotted_to_camel_case(self.name),
            description,
            self.name
        ).as_ref();

        out += r#"    fn call(&self, req: varlink::server::Request) -> Result<serde_json::Value, varlink::server::Error> {
        match req.method.as_ref() {
"#;
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(", args.{}", e.name).as_ref();
                }
            }
            let mut c = t.name.chars();
            let fname = match c.next() {
                None => String::from(t.name),
                Some(f) => f.to_lowercase().chain(c).collect(),
            };

            out += format!("            \"{}.{}\" => {{", self.name, t.name).as_ref();
            if t.input.elts.len() > 0 {
                out += format!(
                    r#"
                if let Some(args) = req.parameters {{
                    let args: {}Args = serde_json::from_value(args)?;
                    return Ok(serde_json::to_value(self.{}(&self{})?)?);
                }} else {{
                    return Err(varlink::server::VarlinkError::InvalidParameter(None).into());
                }}
"#,
                    t.name,
                    fname,
                    inparms
                ).as_ref();
            } else {
                out += format!("return Ok(serde_json::to_value(self.{}()?)?); }}", fname).as_ref();

            }
        }
        out += r#"
            m => {
                let method: String = m.clone().into();
                return Err(varlink::server::VarlinkError::MethodNotFound(Some(method.into())).into());
            }
"#;
        out += "        }\n";
        out += "    }\n";
        out += "}\n}\n";

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

    println!(
        r#"
use std::result::Result;
use std::convert::From;

use varlink;

{}"#,
        vr.unwrap().interface.to_rust(&buffer)?
    );

    Ok(())
}

fn main() {
    exit(do_main().into_error_code());
}
