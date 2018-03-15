extern crate varlink_parser;

use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::io::Error as IOError;
use std::error::Error;
use std::io::{Read, Write};
use std::result::Result;
use std::fmt;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::io;
use std::fs::File;

use varlink_parser::{Interface, VStructOrEnum, VType, VTypeExt, Varlink};

type EnumHash<'a> = HashMap<String, Vec<String>>;

trait ToRust {
    fn to_rust(&self, parent: &str, enumhash: &mut EnumHash) -> Result<String, ToRustError>;
}

#[derive(Debug)]
pub enum ToRustError {
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
                enumhash.insert(
                    parent.into(),
                    Vec::from_iter(v.elts.iter().map(|s| String::from(*s))),
                );
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
                        out += format!(
                            "    pub {}: Option<{}>,\n",
                            e.name,
                            e.vtype
                                .to_rust(format!("{}_{}", t.name, e.name).as_ref(), &mut enumhash)?
                        ).as_ref();
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
                    out += format!(
                        "    pub {}: Option<{}>,\n",
                        e.name,
                        e.vtype.to_rust(self.name, &mut enumhash)?
                    ).as_ref();
                }
                out += "}\n\n";
            }

            if t.input.elts.len() > 0 {
                out += "#[derive(Serialize, Deserialize, Debug)]\n";
                out += format!("pub struct {}Args {{\n", t.name).as_ref();
                for e in &t.input.elts {
                    out += format!(
                        "    pub {}: Option<{}>,\n",
                        e.name,
                        e.vtype.to_rust(self.name, &mut enumhash)?
                    ).as_ref();
                }
                out += "}\n\n";
            }
        }

        for t in self.errors.values() {
            if t.parm.elts.len() > 0 {
                out += "#[derive(Serialize, Deserialize, Debug)]\n";
                out += format!("pub struct {}Args {{\n", t.name).as_ref();
                for e in &t.parm.elts {
                    out += format!(
                        "    pub {}: Option<{}>,\n",
                        e.name,
                        e.vtype.to_rust(self.name, &mut enumhash)?
                    ).as_ref();
                }
                out += "}\n\n";
            }
        }

        if self.errors.len() > 0 {
            out += "#[derive(Debug)]\n";
            out += "pub enum Error {\n";
            for t in self.errors.values() {
                if t.parm.elts.len() > 0 {
                    out += format!("    {}(Option<{}Args>),\n", t.name, t.name).as_ref();
                } else {
                    out += format!("    {},\n", t.name).as_ref();
                }
            }
            out += "}\n";

            out += r#"
impl From<Error> for varlink::server::Error {
    fn from(e: Error) -> Self {
        varlink::server::Error {
            error: match e {
"#;
            for t in self.errors.values() {
                out += format!(
                    r#"                Error::{}{} => "io.systemd.network.{}".into(),
"#,
                    t.name,
                    {
                        if t.parm.elts.len() > 0 {
                            "(_)"
                        } else {
                            ""
                        }
                    },
                    t.name
                ).as_ref();
            }

            out += r#"            },
            parameters: match e {
"#;
            for t in self.errors.values() {
                out += format!(
                    r#"                Error::{}{} => {},
"#,
                    t.name,
                    {
                        if t.parm.elts.len() > 0 {
                            "(args)"
                        } else {
                            ""
                        }
                    },
                    {
                        if t.parm.elts.len() > 0 {
                            "Some(serde_json::to_value(args).unwrap())"
                        } else {
                            "None"
                        }
                    }
                ).as_ref();
            }
            out += r#"            },
        }
    }
}
"#;
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

        out += "pub trait Interface: varlink::server::Interface {\n";
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(
                        ", {}: Option<{}>",
                        e.name,
                        e.vtype.to_rust(self.name, &mut enumhash)?
                    ).as_ref();
                }
            }
            let mut c = t.name.chars();
            let fname = match c.next() {
                None => String::from(t.name),
                Some(f) => f.to_lowercase().chain(c).collect(),
            };

            out += format!(
                "    fn {}(&self{}) -> Result<{}Reply, Error>;\n",
                fname, inparms, t.name
            ).as_ref();
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

        out += concat!(
            "    fn call(&self, req: varlink::server::Request) -> ",
            "Result<serde_json::Value, varlink::server::Error> {\n",
            "        match req.method.as_ref() {\n"
        );
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                let ref e = t.input.elts[0];
                inparms += format!("args.{}", e.name).as_ref();
                for e in &t.input.elts[1..] {
                    inparms += format!(", args.{}, ", e.name).as_ref();
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
                    concat!("\n                if let Some(args) = req.parameters {{\n",
"                    let args: {}Args = serde_json::from_value(args)?;\n",
"                    return Ok(serde_json::to_value(self.{}({})?)?);\n",
"                }} else {{\n",
"                    return Err(varlink::server::VarlinkError::InvalidParameter(None).into());\n",
"                }}\n",
"            }}\n"),
                    t.name,
                    fname,
                    inparms
                ).as_ref();
            } else {
                out +=
                    format!(" return Ok(serde_json::to_value(self.{}()?)?); }}\n", fname).as_ref();
            }
        }
        out += concat!(
            "            m => {\n",
            "                let method: String = m.clone().into();\n",
            "                return Err(varlink::server::VarlinkError::",
            "MethodNotFound(Some(method.into())).into());\n",
            "            }\n",
            "        }\n",
            "    }\n",
            "}\n};\n}"
        );

        Ok(out)
    }
}

pub fn generate(mut reader: Box<Read>, mut writer: Box<Write>) -> io::Result<()> {
    let mut buffer = String::new();

    reader.read_to_string(&mut buffer)?;

    let vr = Varlink::from_string(&buffer);

    if let Err(e) = vr {
        eprintln!("{}", e);
        exit(1);
    }

    match vr.unwrap().interface.to_rust(&buffer) {
        Ok(out) => {
            writeln!(
                writer,
                r#"// This file is automatically generated by the varlink rust generator
use std::result::Result;
use std::convert::From;

use varlink;
use serde_json;

{}"#,
                out
            )?;
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }

    Ok(())
}

/// Errors are emitted to stderr and terminate the process.
pub fn cargo_build<T: AsRef<Path> + ?Sized>(input_path: &T) {
    let mut stderr = io::stderr();
    let input_path = input_path.as_ref();

    let reader: Box<Read>;

    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let rust_path = out_dir
        .join(input_path.file_name().unwrap())
        .with_extension("rs");

    let writer: Box<Write> = Box::new(File::create(&rust_path).unwrap());

    match File::open(input_path) {
        Ok(r) => reader = Box::new(r),
        Err(e) => {
            writeln!(
                stderr,
                "Could not read varlink input file `{}`: {}",
                input_path.display(),
                e
            ).unwrap();
            exit(1);
        }
    }
    if let Err(e) = generate(reader, writer) {
        writeln!(
            stderr,
            "Could not generate rust code from varlink file `{}`: {}",
            input_path.display(),
            e
        ).unwrap();
        exit(1);
    }

    println!("cargo:rerun-if-changed={}", input_path.display());
}
