//! Generate rust code from varlink interface definition files

use failure::{Backtrace, Context, Fail};
use std::borrow::Cow;
use std::env;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{self, Read, Write};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use varlink_parser::{self, VStruct, VStructOrEnum, VType, VTypeExt, Varlink};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "IO error")]
    Io,
    #[fail(display = "Parse Error")]
    Parser,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        self.inner.get_context().clone()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Error {
        e.context(ErrorKind::Io).into()
    }
}

impl From<varlink_parser::Error> for Error {
    fn from(e: varlink_parser::Error) -> Error {
        e.context(ErrorKind::Parser).into()
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

type EnumVec<'a> = Vec<(String, Vec<String>)>;
type StructVec<'a> = Vec<(String, &'a VStruct<'a>)>;

trait ToRust<'short, 'long: 'short> {
    fn to_rust(
        &'long self,
        parent: &str,
        enumvec: &mut EnumVec,
        structvec: &mut StructVec<'short>,
        options: &'long GeneratorOptions,
    ) -> Result<Cow<'long, str>>;
}

#[derive(Default)]
pub struct GeneratorOptions {
    pub bool_type: Option<&'static str>,
    pub int_type: Option<&'static str>,
    pub float_type: Option<&'static str>,
    pub string_type: Option<&'static str>,
    pub preamble: Option<&'static str>,
}

impl<'short, 'long: 'short> ToRust<'short, 'long> for VType<'long> {
    fn to_rust(
        &'long self,
        parent: &str,
        enumvec: &mut EnumVec,
        structvec: &mut StructVec<'short>,
        options: &'long GeneratorOptions,
    ) -> Result<Cow<'long, str>> {
        match *self {
            VType::Bool => Ok(options.bool_type.unwrap_or("bool").into()),
            VType::Int => Ok(options.int_type.unwrap_or("i64").into()),
            VType::Float => Ok(options.float_type.unwrap_or("f64").into()),
            VType::String => Ok(options.string_type.unwrap_or("String").into()),
            VType::Object => Ok("Value".into()),
            VType::Typename(v) => Ok(v.into()),
            VType::Enum(ref v) => {
                enumvec.push((
                    parent.into(),
                    Vec::from_iter(v.elts.iter().map(|s| String::from(*s))),
                ));
                Ok(Cow::Owned(parent.to_string()))
            }
            VType::Struct(ref v) => {
                structvec.push((String::from(parent), v.as_ref()));
                Ok(Cow::Owned(parent.to_string()))
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
        options: &'long GeneratorOptions,
    ) -> Result<Cow<'long, str>> {
        match *self {
            VTypeExt::Plain(ref vtype) => vtype.to_rust(parent, enumvec, structvec, options),
            VTypeExt::Array(ref v) => {
                Ok(format!("Vec<{}>", v.to_rust(parent, enumvec, structvec, options)?).into())
            }
            VTypeExt::Dict(ref v) => match *v.as_ref() {
                VTypeExt::Plain(VType::Struct(ref s)) if s.elts.is_empty() => {
                    Ok("varlink::StringHashSet".into())
                }
                _ => Ok(format!(
                    "varlink::StringHashMap<{}>",
                    v.to_rust(parent, enumvec, structvec, options)?
                ).into()),
            },
            VTypeExt::Option(ref v) => Ok(format!(
                "Option<{}>",
                v.to_rust(parent, enumvec, structvec, options)?
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

fn varlink_to_rust(varlink: &Varlink, w: &mut Write, options: &GeneratorOptions) -> Result<()> {
    let mut enumvec = EnumVec::new();
    let mut structvec = StructVec::new();
    let iface = &varlink.interface;

    // FIXME: use the quote crate with quote! ??

    write!(
        w,
        r#"//! DO NOT EDIT
//! This file is automatically generated by the varlink rust generator

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use failure::{{Backtrace, Context, Fail, ResultExt}};
use serde_json::{{self, Value}};
use std::io::{{self, BufRead}};
use std::sync::{{Arc, RwLock}};
use varlink::{{self, CallTrait}};

"#
    )?;

    if options.preamble.is_some() {
        write!(w, "{}", options.preamble.unwrap())?;
    }

    for t in iface.typedefs.values() {
        match t.elt {
            VStructOrEnum::VStruct(ref v) => {
                write!(
                    w,
                    "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
                )?;
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
                            &mut structvec,
                            options
                        )?
                    )?;
                }
            }
            VStructOrEnum::VEnum(ref v) => {
                write!(
                    w,
                    "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
                )?;
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

    for t in iface.methods.values() {
        write!(
            w,
            "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
        )?;
        write!(w, "pub struct {}_Reply {{\n", t.name)?;
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
                    format!("{}_Reply_{}", t.name, e.name).as_ref(),
                    &mut enumvec,
                    &mut structvec,
                    options
                )?
            )?;
        }
        write!(w, "}}\n\n")?;
        write!(
            w,
            "impl varlink::VarlinkReply for {}_Reply {{}}\n\n",
            t.name
        )?;
        write!(
            w,
            "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
        )?;
        write!(w, "pub struct {}_Args {{\n", t.name)?;
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
                    format!("{}_Args_{}", t.name, e.name).as_ref(),
                    &mut enumvec,
                    &mut structvec,
                    options
                )?
            )?;
        }
        write!(w, "}}\n\n")?;
    }

    for t in iface.errors.values() {
        write!(
            w,
            "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
        )?;
        write!(w, "pub struct {}_Args {{\n", t.name)?;
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
                    format!("{}_Args_{}", t.name, e.name).as_ref(),
                    &mut enumvec,
                    &mut structvec,
                    options
                )?
            )?;
        }
        write!(w, "}}\n\n")?;
    }

    loop {
        let mut nstructvec = StructVec::new();
        for (name, v) in structvec.drain(..) {
            write!(
                w,
                "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n"
            )?;
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
                            &mut nstructvec,
                            options
                        )
                        .unwrap()
                )?;
            }
            write!(w, "}}\n\n")?;
        }
        for (name, v) in enumvec.drain(..) {
            write!(
                w,
                "#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]\n\
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

        if nstructvec.is_empty() {
            break;
        }
        structvec = nstructvec;
    }

    write!(w, "pub trait VarlinkCallError: varlink::CallTrait {{\n")?;
    for t in iface.errors.values() {
        let mut inparms = String::new();
        let mut innames = String::new();
        if !t.parm.elts.is_empty() {
            for e in &t.parm.elts {
                inparms += format!(
                    ", {}: {}",
                    replace_if_rust_keyword(e.name),
                    e.vtype.to_rust(
                        format!("{}_Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
                innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
            }
            innames.pop();
            innames.pop();
        }
        write!(
            w,
            r#"    fn reply_{sname}(&mut self{inparms}) -> varlink::Result<()> {{
        self.reply_struct(varlink::Reply::error(
            "{iname}.{ename}",
"#,
            sname = to_snake_case(t.name),
            inparms = inparms,
            iname = iface.name,
            ename = t.name,
        )?;
        if !t.parm.elts.is_empty() {
            write!(
                w,
                "            Some(serde_json::to_value({}_Args {{ {} }})?),",
                t.name, innames
            )?;
        } else {
            write!(w, "        None,\n")?;
        }

        write!(
            w,
            r#"
        ))
    }}
"#
        )?;
    }
    write!(
        w,
        "}}\n\nimpl<'a> VarlinkCallError for varlink::Call<'a> {{}}\n\n"
    )?;

    write!(
        w,
        r#"
#[derive(Debug)]
pub struct Error {{
    inner: Context<ErrorKind>,
}}

#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {{
    #[fail(display = "IO error")]
    Io_Error(::std::io::ErrorKind),
    #[fail(display = "(De)Serialization Error")]
    SerdeJson_Error(serde_json::error::Category),
    #[fail(display = "Varlink Error")]
    Varlink_Error(varlink::ErrorKind),
    #[fail(display = "Unknown error reply: '{{:#?}}'", _0)]
    VarlinkReply_Error(varlink::Reply),
"#
    )?;
    for t in iface.errors.values() {
        write!(
            w,
            "    \
             #[fail(display = \"{iname}.{ename}: {{:#?}}\", _0)]\n    \
             {ename}(Option<{ename}_Args>),\n",
            ename = t.name,
            iname = iface.name,
        )?;
    }
    write!(
        w,
        r#"}}

impl Fail for Error {{
    fn cause(&self) -> Option<&Fail> {{
        self.inner.cause()
    }}

    fn backtrace(&self) -> Option<&Backtrace> {{
        self.inner.backtrace()
    }}
}}

impl ::std::fmt::Display for Error {{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {{
        ::std::fmt::Display::fmt(&self.inner, f)
    }}
}}

impl Error {{
    pub fn kind(&self) -> ErrorKind {{
        self.inner.get_context().clone()
    }}
}}

impl From<ErrorKind> for Error {{
    fn from(kind: ErrorKind) -> Error {{
        Error {{
            inner: Context::new(kind),
        }}
    }}
}}

impl From<Context<ErrorKind>> for Error {{
    fn from(inner: Context<ErrorKind>) -> Error {{
        Error {{ inner }}
    }}
}}

impl From<::std::io::Error> for Error {{
    fn from(e: ::std::io::Error) -> Error {{
        let kind = e.kind();
        e.context(ErrorKind::Io_Error(kind)).into()
    }}
}}

impl From<serde_json::Error> for Error {{
    fn from(e: serde_json::Error) -> Error {{
        let cat = e.classify();
        e.context(ErrorKind::SerdeJson_Error(cat)).into()
    }}
}}

pub type Result<T> = ::std::result::Result<T, Error>;

impl From<varlink::Error> for Error {{
    fn from(e: varlink::Error) -> Self {{
        let kind = e.kind();
        match kind {{
            varlink::ErrorKind::Io(kind) => e.context(ErrorKind::Io_Error(kind)).into(),
            varlink::ErrorKind::SerdeJsonSer(cat) => e.context(ErrorKind::SerdeJson_Error(cat)).into(),
            kind => e.context(ErrorKind::Varlink_Error(kind)).into(),
        }}
    }}
}}

impl From<varlink::Reply> for Error {{
    fn from(e: varlink::Reply) -> Self {{
        if varlink::Error::is_error(&e) {{
            return varlink::Error::from(e).into();
        }}

        match e {{
"#
    )?;

    for t in iface.errors.values() {
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
            iname = iface.name,
            ename = t.name
        )?;
    }

    write!(
        w,
        r#"            _ => ErrorKind::VarlinkReply_Error(e).into(),
        }}
    }}
}}
"#
    )?;

    for t in iface.methods.values() {
        let mut inparms = String::new();
        let mut innames = String::new();
        if !t.output.elts.is_empty() {
            for e in &t.output.elts {
                inparms += format!(
                    ", {}: {}",
                    replace_if_rust_keyword(e.name),
                    e.vtype.to_rust(
                        format!("{}_Reply_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
                innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
            }
            innames.pop();
            innames.pop();
        }
        write!(w, "pub trait Call_{}: VarlinkCallError {{\n", t.name)?;
        write!(
            w,
            "    fn reply(&mut self{}) -> varlink::Result<()> {{\n",
            inparms
        )?;
        if !t.output.elts.is_empty() {
            write!(
                w,
                "        self.reply_struct({}_Reply {{ {} }}.into())\n",
                t.name, innames
            )?;
        } else {
            write!(
                w,
                "        self.reply_struct(varlink::Reply::parameters(None))\n"
            )?;
        }
        write!(
            w,
            "    }}\n}}\n\nimpl<'a> Call_{} for varlink::Call<'a> {{}}\n\n",
            t.name
        )?;
    }

    write!(w, "pub trait VarlinkInterface {{\n")?;
    for t in iface.methods.values() {
        let mut inparms = String::new();
        if !t.input.elts.is_empty() {
            for e in &t.input.elts {
                inparms += format!(
                    ", {}: {}",
                    replace_if_rust_keyword(e.name),
                    e.vtype.to_rust(
                        format!("{}_Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
            }
        }

        write!(
            w,
            "    fn {}(&self, call: &mut Call_{}{}) -> varlink::Result<()>;\n",
            to_snake_case(t.name),
            t.name,
            inparms
        )?;
    }

    write!(
        w,
        r#"    fn call_upgraded(&self, _call: &mut varlink::Call, _bufreader: &mut BufRead) ->
        varlink::Result<Vec<u8>> {{
        Ok(Vec::new())
    }}
}}

"#
    )?;

    write!(w, "pub trait VarlinkClientInterface {{\n")?;
    for t in iface.methods.values() {
        let mut inparms = String::new();
        let mut outparms = String::new();
        if !t.input.elts.is_empty() {
            for e in &t.input.elts {
                inparms += format!(
                    ", {}: {}",
                    replace_if_rust_keyword(e.name),
                    e.vtype.to_rust(
                        format!("{}_Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
            }
        }
        if !t.output.elts.is_empty() {
            for e in &t.output.elts {
                outparms += format!(
                    "{}, ",
                    e.vtype.to_rust(
                        format!("{}_Reply_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
            }
            outparms.pop();
            outparms.pop();
        }

        write!(
            w,
            "    fn {sname}(&mut self{inparms}) -> varlink::MethodCall<{mname}_Args, \
             {mname}_Reply, Error>;\
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
    for t in iface.methods.values() {
        let mut inparms = String::new();
        let mut innames = String::new();
        if !t.input.elts.is_empty() {
            for e in &t.input.elts {
                inparms += format!(
                    ", {}: {}",
                    replace_if_rust_keyword(e.name),
                    e.vtype.to_rust(
                        format!("{}_Args_{}", t.name, e.name).as_ref(),
                        &mut enumvec,
                        &mut structvec,
                        options
                    )?
                ).as_ref();
                innames += format!("{}, ", replace_if_rust_keyword(e.name)).as_ref();
            }
            innames.pop();
            innames.pop();
        }
        write!(
            w,
            "    fn {sname}(&mut self{inparms}) -> varlink::MethodCall<{mname}_Args, \
             {mname}_Reply, \
             Error> \
             {{\n",
            sname = to_snake_case(t.name),
            inparms = inparms,
            mname = t.name
        )?;

        write!(
            w,
            "            \
             varlink::MethodCall::<{mname}_Args, {mname}_Reply, Error>::new(\n            \
             self.connection.clone(),\n            \
             \"{iname}.{mname}\",\n            \
             {mname}_Args {{ {innames} }},\n        \
             )\n",
            mname = t.name,
            iname = iface.name,
            innames = innames
        )?;
        write!(w, "    }}\n")?;
    }
    write!(w, "}}\n")?;

    write!(
        w,
        r########################################################################################"
pub struct VarlinkInterfaceProxy {{
    inner: Box<VarlinkInterface + Send + Sync>,
}}

pub fn new(inner: Box<VarlinkInterface + Send + Sync>) -> VarlinkInterfaceProxy {{
    VarlinkInterfaceProxy {{ inner }}
}}

impl varlink::Interface for VarlinkInterfaceProxy {{
    fn get_description(&self) -> &'static str {{
        r#####################################"{description}"#####################################
    }}

    fn get_name(&self) -> &'static str {{
        "{iname}"
    }}

"########################################################################################,
        description = varlink.description,
        iname = iface.name
    )?;

    write!(
        w,
        r#"    fn call_upgraded(&self, call: &mut varlink::Call, bufreader: &mut BufRead) ->
        varlink::Result<Vec<u8>> {{
        self.inner.call_upgraded(call, bufreader)
    }}

    fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {{
        let req = call.request.unwrap();
        match req.method.as_ref() {{
"#
    )?;

    for t in iface.methods.values() {
        let mut inparms = String::new();
        for e in &t.input.elts {
            inparms += format!(", args.{}", replace_if_rust_keyword(e.name)).as_ref();
        }

        write!(
            w,
            "            \"{iname}.{mname}\" => {{",
            iname = iface.name,
            mname = t.name
        )?;
        if !t.input.elts.is_empty() {
            write!(
                w,
                concat!(
                    "\n",
                    "                if let Some(args) = req.parameters.clone() {{\n",
                    "                    let args: {mname}_Args = serde_json::from_value(args)?;\n",
                    "                    self.inner.{sname}(call as &mut Call_{mname}{inparms})\n",
                    "                }} else {{\n",
                    "                    call.reply_invalid_parameter(\"parameters\".into())\
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
                    "                self.inner.{sname}(call as &mut Call_{mname})\n",
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
            "                call.reply_method_not_found(String::from(m))\n",
            "            }}\n",
            "        }}\n",
            "    }}\n",
            "}}"
        )
    )?;

    Ok(())
}

/// `generate` reads a varlink interface definition from `reader` and writes
/// the rust code to `writer`.
pub fn generate(reader: &mut Read, writer: &mut Write) -> Result<()> {
    generate_with_options(
        reader,
        writer,
        &GeneratorOptions {
            ..Default::default()
        },
    )
}

/// `generate_with_options` reads a varlink interface definition from `reader` and writes
/// the rust code to `writer`.
pub fn generate_with_options(
    reader: &mut Read,
    writer: &mut Write,
    options: &GeneratorOptions,
) -> Result<()> {
    let mut buffer = String::new();

    reader.read_to_string(&mut buffer)?;

    let vr = Varlink::from_string(&buffer)?;

    varlink_to_rust(&vr, writer, options)?;

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
    cargo_build_options(
        input_path,
        &GeneratorOptions {
            ..Default::default()
        },
    )
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
///    varlink::generator::cargo_build_options("src/org.example.ping.varlink",
///       &varlink::generator::GeneratorOptions {
///           int_type: Some("i128"),
///            ..Default::default()
///        });
///}
///```
///
pub fn cargo_build_options<T: AsRef<Path> + ?Sized>(input_path: &T, options: &GeneratorOptions) {
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

    if let Err(e) = generate_with_options(reader, writer, options) {
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
    cargo_build_tosource_options(
        input_path,
        rustfmt,
        &GeneratorOptions {
            ..Default::default()
        },
    )
}

/// cargo build helper function
///
/// `cargo_build_tosource_options` is used in a `build.rs` program to build the rust code
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
///    varlink::generator::cargo_build_tosource_options("src/org.example.ping.varlink", true,
///        &varlink::generator::GeneratorOptions {
///           int_type: Some("i128"),
///            ..Default::default()
///        }
///    );
///}
///```
///
pub fn cargo_build_tosource_options<T: AsRef<Path> + ?Sized>(
    input_path: &T,
    rustfmt: bool,
    options: &GeneratorOptions,
) {
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

    if let Err(e) = generate_with_options(reader, writer, options) {
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
