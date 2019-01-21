use ansi_term::Colour;
use std::fmt;

use crate::*;

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

impl<'a> fmt::Display for IDL<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.get_multiline(0, 80))
    }
}

impl<'a> Format for IDL<'a> {
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

impl<'a> FormatColored for IDL<'a> {
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
