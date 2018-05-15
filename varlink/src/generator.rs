//! Generate rust code from varlink interface definition files

extern crate varlink_parser;

use std::env;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::exit;
use varlink_parser::{Interface, Varlink, VStruct, VStructOrEnum, VType, VTypeExt};
use std::borrow::Cow;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    links {
        Parser(self::varlink_parser::Error, self::varlink_parser::ErrorKind);
    }
}

type EnumVec<'a> = Vec<(String, Vec<String>)>;
type StructVec<'a> = Vec<(String, &'a VStruct<'a>)>;

trait ToRust<'short, 'long: 'short> {
    fn to_rust(
        &'long self,
        parent: &str,
        enumvec: &mut EnumVec,
        structvec: &mut StructVec<'short>,
    ) -> Result<Cow<'long, str>>;
}

impl<'short, 'long: 'short> ToRust<'short, 'long> for VType<'long> {
    fn to_rust(
        &'long self,
        parent: &str,
        enumvec: &mut EnumVec,
        structvec: &mut StructVec<'short>,
    ) -> Result<Cow<'long, str>> {
        match self {
            &VType::Bool => Ok("bool".into()),
            &VType::Int => Ok("i64".into()),
            &VType::Float => Ok("f64".into()),
            &VType::String => Ok("String".into()),
            &VType::Object => Ok("Value".into()),
            &VType::Typename(v) => Ok(v.into()),
            &VType::Enum(ref v) => {
                enumvec.push((
                    parent.into(),
                    Vec::from_iter(v.elts.iter().map(|s| String::from(*s))),
                ));
                Ok(format!("{}", parent).into())
            }
            &VType::Struct(ref v) => {
                structvec.push((String::from(parent), v.as_ref()));
                Ok(format!("{}", parent).into())
            }
        }
    }
}

impl<'short, 'long: 'short> ToRust<'short, 'long> for VTypeExt<'long> {
    fn to_rust(
        &'long self,
        parent: &str,
        enumvec: &mut EnumVec,
        structvec: &mut StructVec<'short>,
    ) -> Result<Cow<'long, str>> {
        match self {
            &VTypeExt::Plain(ref vtype) => vtype.to_rust(parent, enumvec, structvec),
            &VTypeExt::Array(ref v) => {
                Ok(format!("Vec<{}>", v.to_rust(parent, enumvec, structvec)?).into())
            }
            &VTypeExt::Dict(ref v) => match v.as_ref() {
                &VTypeExt::Plain(VType::Struct(ref s)) if s.elts.len() == 0 => {
                    Ok("varlink::StringHashSet".into())
                }
                _ => Ok(format!(
                    "varlink::StringHashMap<{}>",
                    v.to_rust(parent, enumvec, structvec)?
                ).into()),
            },
            &VTypeExt::Option(ref v) => Ok(format!(
                "Option<{}>",
                v.to_rust(parent, enumvec, structvec)?
            ).into()),
        }
    }
}

fn to_snake_case(mut str: &str) -> String {
    let mut words = vec![];
    // Preserve leading underscores
    str = str.trim_left_matches(|c: char| {
        if c == '_' {
            words.push(String::new());
            true
        } else {
            false
        }
    });
    for s in str.split('_') {
        let mut last_upper = false;
        let mut buf = String::new();
        if s.is_empty() {
            continue;
        }
        for ch in s.chars() {
            if !buf.is_empty() && buf != "'" && ch.is_uppercase() && !last_upper {
                words.push(buf);
                buf = String::new();
            }
            last_upper = ch.is_uppercase();
            buf.extend(ch.to_lowercase());
        }
        words.push(buf);
    }
    words.join("_")
}

fn is_rust_keyword(v: &str) -> bool {
    match v {
        "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" | "continue"
        | "crate" | "do" | "else" | "enum" | "extern" | "false" | "final" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut"
        | "offsetof" | "override" | "priv" | "proc" | "pub" | "pure" | "ref" | "return"
        | "Self" | "self" | "sizeof" | "static" | "struct" | "super" | "trait" | "true"
        | "type" | "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
        | "yield" => true,
        _ => false,
    }
}

fn replace_if_rust_keyword(v: &str) -> String {
    if is_rust_keyword(v) {
        String::from(v) + "_"
    } else {
        String::from(v)
    }
}

fn replace_if_rust_keyword_annotate(v: &str, w: &mut Write) -> io::Result<(String)> {
    if is_rust_keyword(v) {
        write!(w, " #[serde(rename = \"{}\")]", v)?;
        Ok(String::from(v) + "_")
    } else {
        Ok(String::from(v))
    }
}

trait InterfaceToRust {
    fn to_rust(&self, description: &String, writer: &mut Write) -> Result<()>;
}

impl<'a> InterfaceToRust for Interface<'a> {
    fn to_rust(&self, description: &String, w: &mut Write) -> Result<()> {
        let mut enumvec = EnumVec::new();
        let mut structvec = StructVec::new();

        // FIXME: use the quote crate with quote! ??

        write!(
            w,
            r#"//! DO NOT EDIT
//! This file is automatically generated by the varlink rust generator

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use error_chain::ChainedError;
use serde_json::{{self, Value}};
use std::io;
use std::sync::{{Arc, RwLock}};
use varlink;
use varlink::CallTrait;

"#
        )?;

        for t in self.typedefs.values() {
            match t.elt {
                VStructOrEnum::VStruct(ref v) => {
                    write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
                    write!(w, "pub struct {} {{\n", replace_if_rust_keyword(t.name))?;
                    for e in &v.elts {
                        if let VTypeExt::Option(_) = e.vtype {
                            write!(w, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
                        }
                        let ename = replace_if_rust_keyword_annotate(e.name, w)?;
                        write!(
                            w,
                            " pub {}: {},\n",
                            ename,
                            e.vtype.to_rust(
                                format!("{}_{}", t.name, e.name).as_ref(),
                                &mut enumvec,
                                &mut structvec
                            )?
                        )?;
                    }
                }
                VStructOrEnum::VEnum(ref v) => {
                    write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
                    write!(w, "pub enum {} {{\n", t.name)?;
                    let mut iter = v.elts.iter();
                    for elt in iter {
                        let eltname = replace_if_rust_keyword_annotate(elt, w)?;
                        write!(w, "   {},\n", eltname)?;
                    }
                    write!(w, "\n")?;
                }
            }
            write!(w, "}}\n\n")?;
        }

        for t in self.methods.values() {
            write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
            write!(w, "pub struct {}Reply_ {{\n", t.name)?;
            for e in &t.output.elts {
                if let VTypeExt::Option(_) = e.vtype {
                    write!(w, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
                }
                let ename = replace_if_rust_keyword_annotate(e.name, w)?;
                write!(
                    w,
                    " pub {}: {},\n",
                    ename,
                    e.vtype.to_rust(
                        format!("{}Reply_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec
                    )?
                )?;
            }
            write!(w, "}}\n\n")?;
            write!(
                w,
                "impl varlink::VarlinkReply for {}Reply_ {{}}\n\n",
                t.name
            )?;
            write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
            write!(w, "pub struct {}Args_ {{\n", t.name)?;
            for e in &t.input.elts {
                if let VTypeExt::Option(_) = e.vtype {
                    write!(w, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
                }
                let ename = replace_if_rust_keyword_annotate(e.name, w)?;
                write!(
                    w,
                    " pub {}: {},\n",
                    ename,
                    e.vtype.to_rust(
                        format!("{}Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec
                    )?
                )?;
            }
            write!(w, "}}\n\n")?;
        }

        for t in self.errors.values() {
            write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
            write!(w, "pub struct {}Args_ {{\n", t.name)?;
            for e in &t.parm.elts {
                if let VTypeExt::Option(_) = e.vtype {
                    write!(w, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
                }
                let ename = replace_if_rust_keyword_annotate(e.name, w)?;
                write!(
                    w,
                    " pub {}: {},\n",
                    ename,
                    e.vtype.to_rust(
                        format!("{}Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec
                    )?
                )?;
            }
            write!(w, "}}\n\n")?;
        }

        loop {
            let mut nstructvec = StructVec::new();
            for (name, v) in structvec.drain(..) {
                write!(w, "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n")?;
                write!(w, "pub struct {} {{\n", replace_if_rust_keyword(&name))?;
                for e in &v.elts {
                    if let VTypeExt::Option(_) = e.vtype {
                        write!(w, "    #[serde(skip_serializing_if = \"Option::is_none\")]")?;
                    }
                    let ename = replace_if_rust_keyword_annotate(e.name, w)?;
                    write!(
                        w,
                        " pub {}: {},\n",
                        ename,
                        e.vtype
                            .to_rust(
                                format!("{}_{}", name, e.name).as_ref(),
                                &mut enumvec,
                                &mut nstructvec
                            )
                            .unwrap()
                    )?;
                }
                write!(w, "}}\n\n")?;
            }
            for (name, v) in enumvec.drain(..) {
                write!(
                    w,
                    "#[derive(Serialize, Deserialize, Debug, PartialEq)]\n\
                     pub enum {} {{\n",
                    replace_if_rust_keyword(name.as_str())
                )?;
                let mut iter = v.iter();
                for elt in iter {
                    let eltname = replace_if_rust_keyword_annotate(elt, w)?;
                    write!(w, "   {},\n", eltname)?;
                }
                write!(w, "\n}}\n\n")?;
            }

            if nstructvec.len() == 0 {
                break;
            }
            structvec = nstructvec;
        }

        write!(w, "pub trait _CallErr: varlink::CallTrait {{\n")?;
        for t in self.errors.values() {
            let mut inparms: String = "".to_owned();
            let mut innames: String = "".to_owned();
            if t.parm.elts.len() > 0 {
                for e in &t.parm.elts {
                    inparms += format!(
                        ", {}: {}",
                        replace_if_rust_keyword(e.name),
                        e.vtype.to_rust(
                            format!("{}Args_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                    innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
                }
                innames.pop();
                innames.pop();
            }
            write!(
                w,
                r#"    fn reply_{sname}(&mut self{inparms}) -> Result<()> {{
        self.reply_struct(varlink::Reply::error(
            "{iname}.{ename}",
"#,
                sname = to_snake_case(t.name),
                inparms = inparms,
                iname = self.name,
                ename = t.name,
            )?;
            if t.parm.elts.len() > 0 {
                write!(
                    w,
                    "            Some(serde_json::to_value({}Args_ {{ {} }}).unwrap()),",
                    t.name, innames
                )?;
            } else {
                write!(w, "        None,\n")?;
            }

            write!(
                w,
                r#"
        )).map_err(|e| e.into())
    }}
"#
            )?;
        }
        write!(w, "}}\n\nimpl<'a> _CallErr for varlink::Call<'a> {{}}\n\n")?;

        write!(w, "\nerror_chain! {{\n    errors {{\n")?;
        for t in self.errors.values() {
            write!(
                w,
                "        {ename}(t: Option<{ename}Args_>) {{\n",
                ename = t.name
            )?;
            write!(
                w,
                "            display(\"{ename}: '{{:?}}'\", t)\n        }}\n",
                ename = t.name
            )?;
        }
        write!(
            w,
            "    }}\n    \
             foreign_links {{\n        \
             Io(::std::io::Error);\n        \
             Fmt(::std::fmt::Error);\n        \
             SerdeJson(::serde_json::Error);\n        \
             }}\n"
        )?;
        write!(
            w,
            "    \
             links {{\n        \
             Varlink(::varlink::Error, ::varlink::ErrorKind);\n    \
             }}\n}}\n"
        )?;
        write!(
            w,
            r#"
impl From<varlink::Reply> for Error {{
    fn from(e: varlink::Reply) -> Self {{
        if varlink::Error::is_error(&e) {{
            return varlink::Error::from(e).into();
        }}

        match e {{
"#
        )?;

        for t in self.errors.values() {
            write!(
                w,
                r#"            varlink::Reply {{
                     error: Some(ref t), ..
                }} if t == "{iname}.{ename}" =>
                {{
                   match e {{
                       varlink::Reply {{
                           parameters: Some(p),
                           ..
                       }} => match serde_json::from_value(p) {{
                           Ok(v) => ErrorKind::{ename}(v).into(),
                           Err(_) => ErrorKind::{ename}(None).into(),
                       }},
                       _ => ErrorKind::{ename}(None).into(),
                   }}
               }}
"#,
                iname = self.name,
                ename = t.name
            )?;
        }

        write!(
            w,
            r#"            _ => return varlink::Error::from(varlink::ErrorKind::UnknownError(e)).into(),
        }}
    }}
}}
"#
        )?;

        write!(
            w,
            r#"
#[derive(Serialize)]
struct internal_error {{
    message: String
}}

impl From<Error> for varlink::Error {{
    fn from(e: Error) -> Self {{
        match e {{
"#
        )?;

        for t in self.errors.values() {
            write!(
                w,
                r#"         Error(ErrorKind::{ename}(t), _) => {{
                varlink::Error::from(varlink::ErrorKind::UnknownError(varlink::Reply {{
                    error: Some("{iname}.{ename}".into()),
                    parameters: Some(serde_json::to_value(t).unwrap()),
                    ..Default::default()
                }}))
            }}"#,
                ename = t.name,
                iname = self.name
            )?;
        }

        write!(
            w,
            r#"
            e => {{
                varlink::Error::from(varlink::ErrorKind::UnknownError(varlink::Reply {{
                    error: Some("org.example.more.InternalError".into()),
                    parameters: Some(serde_json::to_value(internal_error{{ message: e.display_chain
                    ().to_string()}}).unwrap()),
                    ..Default::default()
                }}))
            }}
        }}
    }}
}}
"#
        )?;

        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            let mut innames: String = "".to_owned();
            if t.output.elts.len() > 0 {
                for e in &t.output.elts {
                    inparms += format!(
                        ", {}: {}",
                        replace_if_rust_keyword(e.name),
                        e.vtype.to_rust(
                            format!("{}Reply_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                    innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
                }
                innames.pop();
                innames.pop();
            }
            write!(w, "pub trait _Call{}: _CallErr {{\n", t.name)?;
            write!(w, "    fn reply(&mut self{}) -> Result<()> {{\n", inparms)?;
            if t.output.elts.len() > 0 {
                write!(
                    w,
                    "        self.reply_struct({}Reply_ {{ {} }}.into()).map_err(|e| e.into())\n",
                    t.name, innames
                )?;
            } else {
                write!(
                    w,
                    "        self.reply_struct(varlink::Reply::parameters(None)).map_err(|e| e.into())\n"
                )?;
            }
            write!(
                w,
                "    }}\n}}\n\nimpl<'a> _Call{} for varlink::Call<'a> {{}}\n\n",
                t.name
            )?;
        }

        write!(w, "pub trait VarlinkInterface {{\n")?;
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(
                        ", {}: {}",
                        replace_if_rust_keyword(e.name),
                        e.vtype.to_rust(
                            format!("{}Args_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                }
            }

            write!(
                w,
                "    fn {}(&self, call: &mut _Call{}{}) -> Result<()>;\n",
                to_snake_case(t.name),
                t.name,
                inparms
            )?;
        }

        write!(
            w,
            r#"    fn call_upgraded(&self, _call: &mut varlink::Call) -> varlink::Result<()> {{
        Ok(())
    }}
}}

"#
        )?;

        write!(w, "pub trait VarlinkClientInterface {{\n")?;
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            let mut outparms: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(
                        ", {}: {}",
                        replace_if_rust_keyword(e.name),
                        e.vtype.to_rust(
                            format!("{}Args_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                }
            }
            if t.output.elts.len() > 0 {
                for e in &t.output.elts {
                    outparms += format!(
                        "{}, ",
                        e.vtype.to_rust(
                            format!("{}Reply_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                }
                outparms.pop();
                outparms.pop();
            }

            write!(
                w,
                "    fn {sname}(&mut self{inparms}) -> varlink::MethodCall<{mname}Args_, \
                 {mname}Reply_, Error>;\
                 \n",
                sname = to_snake_case(t.name),
                inparms = inparms,
                mname = t.name
            )?;
        }

        write!(w, "}}\n")?;

        write!(
            w,
            r#"
pub struct VarlinkClient {{
    connection: Arc<RwLock<varlink::Connection>>,
    more: bool,
    oneway: bool,
}}

impl VarlinkClient {{
    pub fn new(connection: Arc<RwLock<varlink::Connection>>) -> Self {{
        VarlinkClient {{
            connection,
            more: false,
            oneway: false,
        }}
    }}
    pub fn more(&self) -> Self {{
        VarlinkClient {{
            connection: self.connection.clone(),
            more: true,
            oneway: false,
        }}
    }}
    pub fn oneway(&self) -> Self {{
        VarlinkClient {{
            connection: self.connection.clone(),
            more: false,
            oneway: true,
        }}
    }}
}}

impl VarlinkClientInterface for VarlinkClient {{
"#
        )?;
        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            let mut innames: String = "".to_owned();
            if t.input.elts.len() > 0 {
                for e in &t.input.elts {
                    inparms += format!(
                        ", {}: {}",
                        replace_if_rust_keyword(e.name),
                        e.vtype.to_rust(
                            format!("{}Args_{}", t.name, e.name).as_ref(),
                            &mut enumvec,
                            &mut structvec
                        )?
                    ).as_ref();
                    innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
                }
                innames.pop();
                innames.pop();
            }
            write!(
                w,
                "    fn {sname}(&mut self{inparms}) -> varlink::MethodCall<{mname}Args_, \
                 {mname}Reply_, \
                 Error> \
                 {{\n",
                sname = to_snake_case(t.name),
                inparms = inparms,
                mname = t.name
            )?;

            write!(
                w,
                "            \
                 varlink::MethodCall::<{mname}Args_, {mname}Reply_, Error>::new(\n            \
                 self.connection.clone(),\n            \
                 \"{iname}.{mname}\",\n            \
                 {mname}Args_ {{ {innames} }},\n        \
                 )\n",
                mname = t.name,
                iname = self.name,
                innames = innames
            )?;
            write!(w, "    }}\n")?;
        }
        write!(w, "}}\n")?;

        write!(
            w,
            r########################################################################################"
pub struct _InterfaceProxy {{
    inner: Box<VarlinkInterface + Send + Sync>,
}}

pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> _InterfaceProxy {{
    _InterfaceProxy {{ inner }}
}}

impl varlink::Interface for _InterfaceProxy {{
    fn get_description(&self) -> &'static str {{
        r#####################################"{description}"#####################################
    }}

    fn get_name(&self) -> &'static str {{
        "{iname}"
    }}

"########################################################################################,
            description = description,
            iname = self.name
        )?;

        write!(
            w,
            r#"    fn call_upgraded(&self, call: &mut varlink::Call) -> varlink::Result<()> {{
        self.inner.call_upgraded(call)
    }}

    fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {{
        let req = call.request.unwrap();
        match req.method.as_ref() {{
"#
        )?;

        for t in self.methods.values() {
            let mut inparms: String = "".to_owned();
            for e in &t.input.elts {
                inparms += format!(", args.{}", replace_if_rust_keyword(e.name)).as_ref();
            }

            write!(
                w,
                "            \"{iname}.{mname}\" => {{",
                iname = self.name,
                mname = t.name
            )?;
            if t.input.elts.len() > 0 {
                write!(
                    w,
                    concat!(
                        "\n",
                        "                if let Some(args) = req.parameters.clone() {{\n",
                        "                    let args: {mname}Args_ = serde_json::from_value(args)?;\n",
                        "                    return self.inner.{sname}(call as &mut \
                        _Call{mname}{inparms}).map_err(|e| e.into());\n",
                        "                }} else {{\n",
                        "                    return call.reply_invalid_parameter(\"parameters\".into());\
                        \n",
                        "                }}\n",
                        "            }}\n"
                    ),
                    mname = t.name,
                    sname = to_snake_case(t.name),
                    inparms = inparms
                )?;
            } else {
                write!(
                    w,
                    concat!(
                        "\n",
                        "                return self.inner.{sname}(call as &mut _Call{mname}).map_err(|e| e.into());\n",
                        "            }}\n"
                    ),
                    sname = to_snake_case(t.name),
                    mname = t.name
                )?;
            }
        }
        write!(
            w,
            concat!(
                "\n",
                "            m => {{\n",
                "                return call.reply_method_not_found(String::from(m));\n",
                "            }}\n",
                "        }}\n",
                "    }}\n",
                "}}"
            )
        )?;

        Ok(())
    }
}

/// `generate` reads a varlink interface definition from `reader` and writes
/// the rust code to `writer`.
pub fn generate(reader: &mut Read, writer: &mut Write) -> Result<()> {
    let mut buffer = String::new();

    reader.read_to_string(&mut buffer)?;

    let vr = Varlink::from_string(&buffer)?;

    vr.interface.to_rust(&buffer, writer)?;

    Ok(())
}

/// cargo build helper function
///
/// `cargo_build` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition.
///
/// Errors are emitted to stderr and terminate the process.
///
///# Examples
///
///```rust,no_run
///extern crate varlink;
///
///fn main() {
///    varlink::generator::cargo_build("src/org.example.ping.varlink");
///}
///```
///
pub fn cargo_build<T: AsRef<Path> + ?Sized>(input_path: &T) {
    let input_path = input_path.as_ref();

    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let rust_path = out_dir
        .join(input_path.file_name().unwrap())
        .with_extension("rs");

    let writer: &mut Write = &mut (File::create(&rust_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not open varlink output file `{}`: {}",
            rust_path.display(),
            e
        );
        exit(1);
    }));

    let reader: &mut Read = &mut (File::open(input_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not read varlink input file `{}`: {}",
            input_path.display(),
            e
        );
        exit(1);
    }));

    if let Err(e) = generate(reader, writer) {
        eprintln!(
            "Could not generate rust code from varlink file `{}`: {}",
            input_path.display(),
            e
        );
        exit(1);
    }

    println!("cargo:rerun-if-changed={}", input_path.display());
}

/// cargo build helper function
///
/// `cargo_build_tosource` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition. This function saves the rust code
/// in the same directory as the varlink file. The name is the name of the varlink file
/// and "." replaced with "_" and of course ending with ".rs".
///
/// Use this, if you are using an IDE with code completion, as most cannot cope with
/// `include!(concat!(env!("OUT_DIR"), "<varlink_file>"));`
///
/// Set `rustfmt` to `true`, if you want the generator to run rustfmt on the generated
/// code. This might be good practice to avoid large changes after a global `cargo fmt` run.
///
/// Errors are emitted to stderr and terminate the process.
///
///# Examples
///
///```rust,no_run
///extern crate varlink;
///
///fn main() {
///    varlink::generator::cargo_build_tosource("src/org.example.ping.varlink", true);
///}
///```
///
pub fn cargo_build_tosource<T: AsRef<Path> + ?Sized>(input_path: &T, rustfmt: bool) {
    let input_path = input_path.as_ref();
    let noextension = input_path.with_extension("");
    let newfilename = noextension
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .replace(".", "_");
    let rust_path = input_path
        .parent()
        .unwrap()
        .join(Path::new(&newfilename).with_extension("rs"));

    let writer: &mut Write = &mut (File::create(&rust_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not open varlink output file `{}`: {}",
            rust_path.display(),
            e
        );
        exit(1);
    }));

    let reader: &mut Read = &mut (File::open(input_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not read varlink input file `{}`: {}",
            input_path.display(),
            e
        );
        exit(1);
    }));

    if let Err(e) = generate(reader, writer) {
        eprintln!(
            "Could not generate rust code from varlink file `{}`: {}",
            input_path.display(),
            e
        );
        exit(1);
    }

    if rustfmt {
        if let Err(e) = Command::new("rustfmt")
            .arg(rust_path.to_str().unwrap())
            .output()
        {
            eprintln!(
                "Could not run rustfmt on file `{}` {}",
                rust_path.display(),
                e
            );
            exit(1);
        }
    }

    println!("cargo:rerun-if-changed={}", input_path.display());
}
