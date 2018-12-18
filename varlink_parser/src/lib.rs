/*!
 varlink_parser crate for parsing [varlink](http://varlink.org) interface definition files.

 # Examples

 ```rust
 use varlink_parser::Varlink;
 let v = Varlink::from_string("
 ## The Varlink Service Interface is provided by every varlink service. It
 ## describes the service and the interfaces it implements.
 interface org.varlink.service

 ## Get a list of all the interfaces a service provides and information
 ## about the implementation.
 method GetInfo() -> (
 vendor: string,
 product: string,
 version: string,
 url: string,
 interfaces: []string
 )

 ## Get the description of an interface that is implemented by this service.
 method GetInterfaceDescription(interface: string) -> (description: string)

 ## The requested interface was not found.
 error InterfaceNotFound (interface: string)

 ## The requested method was not found
 error MethodNotFound (method: string)

 ## The interface defines the requested method, but the service does not
 ## implement it.
 error MethodNotImplemented (method: string)

 ## One of the passed parameters is invalid.
 error InvalidParameter (parameter: string)
 ").unwrap();
    assert_eq!(v.interface.name, "org.varlink.service");
 ```
!*/

#![doc(
    html_logo_url = "https://varlink.org/images/varlink.png",
    html_favicon_url = "https://varlink.org/images/varlink-small.png"
)]

use self::varlink_grammar::VInterface;
use ansi_term::Colour;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;

use chainerror::*;

#[cfg(test)]
mod test;

#[cfg(feature = "peg")]
mod varlink_grammar {
    include!(concat!(env!("OUT_DIR"), "/varlink_grammar.rs"));
}

#[cfg(not(feature = "peg"))]
mod varlink_grammar;

derive_str_cherr!(Error);

pub enum VType<'a> {
    Bool,
    Int,
    Float,
    String,
    Object,
    Typename(&'a str),
    Struct(Box<VStruct<'a>>),
    Enum(Box<VEnum<'a>>),
}

pub enum VTypeExt<'a> {
    Array(Box<VTypeExt<'a>>),
    Dict(Box<VTypeExt<'a>>),
    Option(Box<VTypeExt<'a>>),
    Plain(VType<'a>),
}

pub struct Argument<'a> {
    pub name: &'a str,
    pub vtype: VTypeExt<'a>,
}

pub struct VStruct<'a> {
    pub elts: Vec<Argument<'a>>,
}

pub struct VEnum<'a> {
    pub elts: Vec<&'a str>,
}

pub struct VError<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub parm: VStruct<'a>,
}

pub enum VStructOrEnum<'a> {
    VStruct(Box<VStruct<'a>>),
    VEnum(Box<VEnum<'a>>),
}

pub struct Typedef<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub elt: VStructOrEnum<'a>,
}

pub struct Method<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub input: VStruct<'a>,
    pub output: VStruct<'a>,
}

enum MethodOrTypedefOrError<'a> {
    Error(VError<'a>),
    Typedef(Typedef<'a>),
    Method(Method<'a>),
}

pub struct Interface<'a> {
    pub name: &'a str,
    pub doc: &'a str,
    pub methods: BTreeMap<&'a str, Method<'a>>,
    pub method_keys: Vec<&'a str>,
    pub typedefs: BTreeMap<&'a str, Typedef<'a>>,
    pub typedef_keys: Vec<&'a str>,
    pub errors: BTreeMap<&'a str, VError<'a>>,
    pub error_keys: Vec<&'a str>,
    pub error: HashSet<String>,
}

pub trait Format {
    fn get_oneline(&self) -> String;
    fn get_multiline(&self, indent: usize, max: usize) -> String;
}

pub trait FormatColored {
    fn get_oneline_colored(&self) -> String;
    fn get_multiline_colored(&self, indent: usize, max: usize) -> String;
}

impl<'a> Format for VTypeExt<'a> {
    fn get_oneline(&self) -> String {
        match *self {
            VTypeExt::Plain(VType::Bool) => "bool".into(),
            VTypeExt::Plain(VType::Int) => "int".into(),
            VTypeExt::Plain(VType::Float) => "float".into(),
            VTypeExt::Plain(VType::String) => "string".into(),
            VTypeExt::Plain(VType::Object) => "object".into(),
            VTypeExt::Plain(VType::Typename(v)) => v.into(),
            VTypeExt::Plain(VType::Struct(ref v)) => v.get_oneline(),
            VTypeExt::Plain(VType::Enum(ref v)) => v.get_oneline(),
            VTypeExt::Array(ref v) => format!("[]{}", v.get_oneline()),
            VTypeExt::Dict(ref v) => format!("[{}]{}", "string", v.get_oneline()),
            VTypeExt::Option(ref v) => format!("?{}", v.get_oneline()),
        }
    }
    fn get_multiline(&self, indent: usize, max: usize) -> String {
        match *self {
            VTypeExt::Plain(VType::Bool) => "bool".into(),
            VTypeExt::Plain(VType::Int) => "int".into(),
            VTypeExt::Plain(VType::Float) => "float".into(),
            VTypeExt::Plain(VType::String) => "string".into(),
            VTypeExt::Plain(VType::Object) => "object".into(),
            VTypeExt::Plain(VType::Typename(v)) => v.into(),
            VTypeExt::Plain(VType::Struct(ref v)) => v.get_multiline(indent, max),
            VTypeExt::Plain(VType::Enum(ref v)) => v.get_multiline(indent, max),
            VTypeExt::Array(ref v) => format!("[]{}", v.get_multiline(indent, max)),
            VTypeExt::Dict(ref v) => format!("[{}]{}", "string", v.get_multiline(indent, max)),
            VTypeExt::Option(ref v) => format!("?{}", v.get_multiline(indent, max)),
        }
    }
}

impl<'a> FormatColored for VTypeExt<'a> {
    fn get_oneline_colored(&self) -> String {
        match *self {
            VTypeExt::Plain(VType::Bool) => Colour::Cyan.paint("bool").to_string(),
            VTypeExt::Plain(VType::Int) => Colour::Cyan.paint("int").to_string(),
            VTypeExt::Plain(VType::Float) => Colour::Cyan.paint("float").to_string(),
            VTypeExt::Plain(VType::String) => Colour::Cyan.paint("string").to_string(),
            VTypeExt::Plain(VType::Object) => Colour::Cyan.paint("object").to_string(),
            VTypeExt::Plain(VType::Typename(ref v)) => {
                Colour::Cyan.paint(v.to_string()).to_string()
            }
            VTypeExt::Plain(VType::Struct(ref v)) => v.get_oneline_colored(),
            VTypeExt::Plain(VType::Enum(ref v)) => v.get_oneline_colored(),
            VTypeExt::Array(ref v) => format!("[]{}", v.get_oneline_colored()),
            VTypeExt::Dict(ref v) => format!(
                "[{}]{}",
                Colour::Cyan.paint("string"),
                v.get_oneline_colored()
            ),
            VTypeExt::Option(ref v) => format!("?{}", v.get_oneline_colored()),
        }
    }
    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        match *self {
            VTypeExt::Plain(VType::Bool) => Colour::Cyan.paint("bool").to_string(),
            VTypeExt::Plain(VType::Int) => Colour::Cyan.paint("int").to_string(),
            VTypeExt::Plain(VType::Float) => Colour::Cyan.paint("float").to_string(),
            VTypeExt::Plain(VType::String) => Colour::Cyan.paint("string").to_string(),
            VTypeExt::Plain(VType::Object) => Colour::Cyan.paint("object").to_string(),
            VTypeExt::Plain(VType::Typename(ref v)) => {
                Colour::Cyan.paint(v.to_string()).to_string()
            }
            VTypeExt::Plain(VType::Struct(ref v)) => v.get_multiline_colored(indent, max),
            VTypeExt::Plain(VType::Enum(ref v)) => v.get_multiline_colored(indent, max),
            VTypeExt::Array(ref v) => format!("[]{}", v.get_multiline_colored(indent, max)),
            VTypeExt::Dict(ref v) => format!(
                "[{}]{}",
                Colour::Cyan.paint("string"),
                v.get_multiline_colored(indent, max)
            ),
            VTypeExt::Option(ref v) => format!("?{}", v.get_multiline_colored(indent, max)),
        }
    }
}

impl<'a> fmt::Display for VTypeExt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_oneline())
    }
}

impl<'a> Format for VStructOrEnum<'a> {
    fn get_oneline(&self) -> String {
        match *self {
            VStructOrEnum::VStruct(ref v) => v.get_oneline(),
            VStructOrEnum::VEnum(ref v) => v.get_oneline(),
        }
    }

    fn get_multiline(&self, indent: usize, max: usize) -> String {
        match *self {
            VStructOrEnum::VStruct(ref v) => v.get_multiline(indent, max),
            VStructOrEnum::VEnum(ref v) => v.get_multiline(indent, max),
        }
    }
}

impl<'a> FormatColored for VStructOrEnum<'a> {
    fn get_oneline_colored(&self) -> String {
        match *self {
            VStructOrEnum::VStruct(ref v) => v.get_oneline_colored(),
            VStructOrEnum::VEnum(ref v) => v.get_oneline_colored(),
        }
    }

    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        match *self {
            VStructOrEnum::VStruct(ref v) => v.get_multiline_colored(indent, max),
            VStructOrEnum::VEnum(ref v) => v.get_multiline_colored(indent, max),
        }
    }
}

impl<'a> fmt::Display for VStructOrEnum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_oneline())
    }
}

impl<'a> Format for Argument<'a> {
    fn get_oneline(&self) -> String {
        format!("{}: {}", self.name, self.vtype)
    }

    fn get_multiline(&self, indent: usize, max: usize) -> String {
        format!("{}: {}", self.name, self.vtype.get_multiline(indent, max))
    }
}

impl<'a> FormatColored for Argument<'a> {
    fn get_oneline_colored(&self) -> String {
        format!("{}: {}", self.name, self.vtype.get_oneline_colored())
    }

    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        format!(
            "{}: {}",
            self.name,
            self.vtype.get_multiline_colored(indent, max)
        )
    }
}

impl<'a> fmt::Display for Argument<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_oneline())
    }
}

impl<'a> fmt::Display for VStruct<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            write!(f, "{}", fst)?;
            for elt in iter {
                write!(f, ", {}", elt)?;
            }
        }
        write!(f, ")")
    }
}

impl<'a> Format for VStruct<'a> {
    fn get_oneline(&self) -> String {
        let mut f = String::new();

        f += "(";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            f += &fst.get_oneline();
            for elt in iter {
                f += &format!(", {}", elt.get_oneline());
            }
        }
        f += ")";
        f
    }

    fn get_multiline(&self, indent: usize, max: usize) -> String {
        let mut f = String::new();

        f += "(\n";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            let line = fst.get_oneline();
            if line.len() + indent + 2 < max {
                f += &format!("{:indent$}{}", "", line, indent = indent + 2);
            } else {
                f += &format!(
                    "{:indent$}{}",
                    "",
                    fst.get_multiline(indent + 2, max),
                    indent = indent + 2
                );
            }
            for elt in iter {
                f += ",\n";
                let line = elt.get_oneline();
                if line.len() + indent + 2 < max {
                    f += &format!("{:indent$}{}", "", line, indent = indent + 2);
                } else {
                    f += &format!(
                        "{:indent$}{}",
                        "",
                        elt.get_multiline(indent + 2, max),
                        indent = indent + 2
                    );
                }
            }
        }
        f += &format!("\n{:indent$})", "", indent = indent);
        f
    }
}

impl<'a> FormatColored for VStruct<'a> {
    fn get_oneline_colored(&self) -> String {
        let mut f = String::new();

        f += "(";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            f += &fst.get_oneline_colored();
            for elt in iter {
                f += &format!(", {}", elt.get_oneline_colored());
            }
        }
        f += ")";
        f
    }

    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        let mut f = String::new();

        f += "(\n";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            let line = fst.get_oneline();
            if line.len() + indent + 2 < max {
                f += &format!(
                    "{:indent$}{}",
                    "",
                    fst.get_oneline_colored(),
                    indent = indent + 2
                );
            } else {
                f += &format!(
                    "{:indent$}{}",
                    "",
                    fst.get_multiline_colored(indent + 2, max),
                    indent = indent + 2
                );
            }
            for elt in iter {
                f += ",\n";
                let line = elt.get_oneline();
                if line.len() + indent + 2 < max {
                    f += &format!(
                        "{:indent$}{}",
                        "",
                        elt.get_oneline_colored(),
                        indent = indent + 2
                    );
                } else {
                    f += &format!(
                        "{:indent$}{}",
                        "",
                        elt.get_multiline_colored(indent + 2, max),
                        indent = indent + 2
                    );
                }
            }
        }
        f += &format!("\n{:indent$})", "", indent = indent);
        f
    }
}

impl<'a> fmt::Display for VEnum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_oneline())
    }
}

impl<'a> Format for VEnum<'a> {
    fn get_oneline(&self) -> String {
        let mut f = String::new();

        f += "(";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            f += fst;
            for elt in iter {
                f += &format!(", {}", elt);
            }
        }
        f += ")";
        f
    }

    fn get_multiline(&self, indent: usize, _max: usize) -> String {
        let mut f = String::new();

        f += "(\n";
        let mut iter = self.elts.iter();
        if let Some(fst) = iter.next() {
            f += &format!("{:indent$}{}", "", fst, indent = indent + 2);
            for elt in iter {
                f += &format!(",\n{:indent$}{}", "", elt, indent = indent + 2);
            }
        }
        f += &format!("\n{:indent$})", "", indent = indent);
        f
    }
}

impl<'a> FormatColored for VEnum<'a> {
    fn get_oneline_colored(&self) -> String {
        self.get_oneline()
    }

    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        self.get_multiline(indent, max)
    }
}

fn trim_doc(s: &str) -> &str {
    s.trim_matches(&[
        ' ', '\n', '\r', '\u{00A0}', '\u{FEFF}', '\u{1680}', '\u{180E}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}', '\u{2028}', '\u{2029}',
    ] as &[_])
}

impl<'a> fmt::Display for Interface<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_multiline(0, 80))
    }
}

impl<'a> Format for Interface<'a> {
    fn get_oneline(&self) -> String {
        let mut f = String::new();

        if !self.doc.is_empty() {
            f += &self.doc;
            f += "\n";
        }
        f += &format!("{} {}\n", "interface", self.name);

        for t in self.typedef_keys.iter().map(|k| &self.typedefs[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &t.doc;
                f += "\n";
            }

            f += &format!("{} {} {}\n", "type", t.name, t.elt.get_oneline());
        }

        for m in self.method_keys.iter().map(|k| &self.methods[k]) {
            f += "\n";
            if !m.doc.is_empty() {
                f += &m.doc;
                f += "\n";
            }

            f += &format!(
                "{} {}{} {} {}\n",
                "method",
                m.name,
                m.input.get_oneline(),
                "->",
                m.output.get_oneline()
            );
        }

        for t in self.error_keys.iter().map(|k| &self.errors[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &t.doc;
                f += "\n";
            }

            f += &format!("{} {} {}\n", "error", t.name, t.parm.get_oneline());
        }
        f
    }

    fn get_multiline(&self, indent: usize, max: usize) -> String {
        let mut f = String::new();

        if !self.doc.is_empty() {
            f += &self
                .doc
                .split('\n')
                .map(|s| format!("{:indent$}{}", "", s, indent = indent))
                .collect::<Vec<String>>()
                .join("\n");

            f += "\n";
        }
        f += &format!(
            "{:indent$}{} {}\n",
            "",
            "interface",
            self.name,
            indent = indent
        );

        for t in self.typedef_keys.iter().map(|k| &self.typedefs[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &format!("{:indent$}{}", "", t.doc, indent = indent);
                f += "\n";
            }

            let line = format!("{:indent$}type {} ", "", t.name, indent = indent);
            let elt_line = t.elt.get_oneline();
            if line.len() + elt_line.len() <= max {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    "type",
                    t.name,
                    t.elt.get_oneline(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    "type",
                    t.name,
                    t.elt.get_multiline(indent, max),
                    indent = indent
                );
            }
        }

        for m in self.method_keys.iter().map(|k| &self.methods[k]) {
            f += "\n";
            if !m.doc.is_empty() {
                f += &m
                    .doc
                    .split('\n')
                    .map(|s| format!("{:indent$}{}", "", s, indent = indent))
                    .collect::<Vec<String>>()
                    .join("\n");
                f += "\n";
            }

            let m_line = format!("method {}", m.name);
            let m_input = m.input.get_oneline();
            let m_output = m.output.get_oneline();
            if (m_line.len() + m_input.len() + m_output.len() + 4 <= max)
                || (m_input.len() + m_output.len() == 4)
            {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    "method",
                    m.name,
                    m.input.get_oneline(),
                    "->",
                    m.output.get_oneline(),
                    indent = indent
                );
            } else if (m_line.len() + m_input.len() + 6 <= max) || (m_input.len() == 2) {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    "method",
                    m.name,
                    m.input.get_oneline(),
                    "->",
                    m.output.get_multiline(indent, max),
                    indent = indent
                );
            } else if m_output.len() + 7 <= max {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    "method",
                    m.name,
                    m.input.get_multiline(indent, max),
                    "->",
                    m.output.get_oneline(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    "method",
                    m.name,
                    m.input.get_multiline(indent, max),
                    "->",
                    m.output.get_multiline(indent, max),
                    indent = indent
                );
            }
        }
        for t in self.error_keys.iter().map(|k| &self.errors[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &t
                    .doc
                    .split('\n')
                    .map(|s| format!("{:indent$}{}", "", s, indent = indent))
                    .collect::<Vec<String>>()
                    .join("\n");
                f += "\n";
            }

            let line = format!("{:indent$}error {} ", "", t.name, indent = indent);
            let elt_line = t.parm.get_oneline();
            if line.len() + elt_line.len() <= max {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    "error",
                    t.name,
                    t.parm.get_oneline(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    "error",
                    t.name,
                    t.parm.get_multiline(indent, max),
                    indent = indent
                );
            }
        }
        f
    }
}

impl<'a> FormatColored for Interface<'a> {
    fn get_oneline_colored(&self) -> String {
        let mut f = String::new();

        if !self.doc.is_empty() {
            f += &Colour::Blue.paint(self.doc);
            f += "\n";
        }
        f += &format!("{} {}\n", Colour::Purple.paint("interface"), self.name);

        for t in self.typedef_keys.iter().map(|k| &self.typedefs[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &Colour::Blue.paint(t.doc);
                f += "\n";
            }

            f += &format!(
                "{} {} {}\n",
                Colour::Purple.paint("type"),
                Colour::Cyan.paint(t.name),
                t.elt.get_oneline_colored()
            );
        }

        for m in self.method_keys.iter().map(|k| &self.methods[k]) {
            f += "\n";
            if !m.doc.is_empty() {
                f += &Colour::Blue.paint(m.doc);
                f += "\n";
            }

            f += &format!(
                "{} {}{} {} {}\n",
                Colour::Purple.paint("method"),
                Colour::Green.paint(m.name),
                m.input.get_oneline_colored(),
                Colour::Purple.paint("->"),
                m.output.get_oneline_colored()
            );
        }
        for t in self.error_keys.iter().map(|k| &self.errors[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &Colour::Blue.paint(t.doc);
                f += "\n";
            }

            f += &format!(
                "{} {} {}\n",
                Colour::Purple.paint("error"),
                Colour::Cyan.paint(t.name),
                t.parm.get_oneline_colored()
            );
        }
        f
    }

    fn get_multiline_colored(&self, indent: usize, max: usize) -> String {
        let mut f = String::new();

        if !self.doc.is_empty() {
            f += &self
                .doc
                .split('\n')
                .map(|s| format!("{:indent$}{}", "", Colour::Blue.paint(s), indent = indent))
                .collect::<Vec<String>>()
                .join("\n");
            f += "\n";
        }
        f += &format!(
            "{:indent$}{} {}\n",
            "",
            Colour::Purple.paint("interface"),
            self.name,
            indent = indent
        );

        for t in self.typedef_keys.iter().map(|k| &self.typedefs[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &t
                    .doc
                    .to_string()
                    .split('\n')
                    .map(|s| format!("{:indent$}{}", "", Colour::Blue.paint(s), indent = indent))
                    .collect::<Vec<String>>()
                    .join("\n");
                f += "\n";
            }

            let line = format!("{:indent$}type {} ", "", t.name, indent = indent);
            let elt_line = t.elt.get_oneline();
            if line.len() + elt_line.len() <= max {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    Colour::Purple.paint("type"),
                    Colour::Cyan.paint(t.name),
                    t.elt.get_oneline_colored(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    Colour::Purple.paint("type"),
                    Colour::Cyan.paint(t.name),
                    t.elt.get_multiline_colored(indent, max),
                    indent = indent
                );
            }
        }

        for m in self.method_keys.iter().map(|k| &self.methods[k]) {
            f += "\n";
            if !m.doc.is_empty() {
                f += &m
                    .doc
                    .to_string()
                    .split('\n')
                    .map(|s| format!("{:indent$}{}", "", Colour::Blue.paint(s), indent = indent))
                    .collect::<Vec<String>>()
                    .join("\n");

                f += "\n";
            }

            let m_line = format!("method {}", m.name);
            let m_input = m.input.get_oneline();
            let m_output = m.output.get_oneline();
            if (m_line.len() + m_input.len() + m_output.len() + 4 <= max)
                || (m_input.len() + m_output.len() == 4)
            {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    Colour::Purple.paint("method"),
                    Colour::Green.paint(m.name),
                    m.input.get_oneline_colored(),
                    Colour::Purple.paint("->"),
                    m.output.get_oneline_colored(),
                    indent = indent
                );
            } else if (m_line.len() + m_input.len() + 6 <= max) || (m_input.len() == 2) {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    Colour::Purple.paint("method"),
                    Colour::Green.paint(m.name),
                    m.input.get_oneline_colored(),
                    Colour::Purple.paint("->"),
                    m.output.get_multiline_colored(indent, max),
                    indent = indent
                );
            } else if m_output.len() + 7 <= max {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    Colour::Purple.paint("method"),
                    Colour::Green.paint(m.name),
                    m.input.get_multiline_colored(indent, max),
                    Colour::Purple.paint("->"),
                    m.output.get_oneline_colored(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {}{} {} {}\n",
                    "",
                    Colour::Purple.paint("method"),
                    Colour::Green.paint(m.name),
                    m.input.get_multiline_colored(indent, max),
                    Colour::Purple.paint("->"),
                    m.output.get_multiline_colored(indent, max),
                    indent = indent
                );
            }
        }
        for t in self.error_keys.iter().map(|k| &self.errors[k]) {
            f += "\n";
            if !t.doc.is_empty() {
                f += &t
                    .doc
                    .to_string()
                    .split('\n')
                    .map(|s| format!("{:indent$}{}", "", Colour::Blue.paint(s), indent = indent))
                    .collect::<Vec<String>>()
                    .join("\n");

                f += "\n";
            }

            let line = format!("error {} ", t.name);
            let elt_line = t.parm.get_oneline();
            if line.len() + elt_line.len() <= max {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    Colour::Purple.paint("error"),
                    Colour::Cyan.paint(t.name),
                    t.parm.get_oneline_colored(),
                    indent = indent
                );
            } else {
                f += &format!(
                    "{:indent$}{} {} {}\n",
                    "",
                    Colour::Purple.paint("error"),
                    Colour::Cyan.paint(t.name),
                    t.parm.get_multiline_colored(indent, max),
                    indent = indent
                );
            }
        }
        f
    }
}

impl<'a> Interface<'a> {
    fn from_token(
        name: &'a str,
        mt: Vec<MethodOrTypedefOrError<'a>>,
        doc: &'a str,
    ) -> Interface<'a> {
        let mut i = Interface {
            name,
            doc,
            methods: BTreeMap::new(),
            method_keys: Vec::new(),
            typedefs: BTreeMap::new(),
            typedef_keys: Vec::new(),
            errors: BTreeMap::new(),
            error_keys: Vec::new(),
            error: HashSet::new(),
        };

        for o in mt {
            match o {
                MethodOrTypedefOrError::Method(m) => {
                    i.method_keys.push(m.name);
                    if let Some(d) = i.methods.insert(m.name, m) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of type `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Typedef(t) => {
                    i.typedef_keys.push(t.name);
                    if let Some(d) = i.typedefs.insert(t.name, t) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of type `{}`!",
                            i.name, d.name
                        ));
                    };
                }
                MethodOrTypedefOrError::Error(e) => {
                    i.error_keys.push(e.name);
                    if let Some(d) = i.errors.insert(e.name, e) {
                        i.error.insert(format!(
                            "Interface `{}`: multiple definitions of error `{}`!",
                            i.name, d.name
                        ));
                    };
                }
            };
        }

        i
    }
}

pub struct Varlink<'short, 'long: 'short> {
    pub description: &'long str,
    pub interface: Interface<'short>,
}

impl<'short, 'long: 'short> Varlink<'short, 'long> {
    pub fn from_string<S: ?Sized + AsRef<str>>(s: &'long S) -> ChainResult<Self, Error> {
        let s = s.as_ref();
        let iface = VInterface(s).map_err(mstrerr!(Error, "Could not parse: {}", s))?;
        if !iface.error.is_empty() {
            Err(strerr!(
                Error,
                "Interface definition error: '{}'\n",
                iface.error.into_iter().sorted().join("\n")
            ))
        } else {
            Ok(Varlink {
                description: s,
                interface: iface,
            })
        }
    }
}
