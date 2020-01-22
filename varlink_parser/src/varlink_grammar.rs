pub use grammar::*;

peg::parser! {
    grammar grammar() for str {
        /* Modeled after ECMA-262, 5th ed., 7.2. */
        rule whitespace() -> &'input str
          = quiet!{$([' ' | '\t' | '\u{00A0}' | '\u{FEFF}' | '\u{1680}' | '\u{180E}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' ])}
          / expected!("whitespace")

        /* Modeled after ECMA-262, 5th ed., 7.3. */
        rule eol_r() -> &'input str
          = $( "\n" )
          / $( "\r\n" )
          / $( "\r" )
          / $( "\u{2028}" )
          / $( "\u{2029}" )

        rule comment() -> &'input str
            = quiet!{
                $( "#" (!['\n' | '\r' | '\u{2028}' | '\u{2029}' ][_])* eol_r() )
            }
            / expected!("<comment>")

        rule eol() -> &'input str
            = quiet!{ $( whitespace()* eol_r() ) }
            / quiet! { $( comment() ) }
            / expected!("<newline>")

        rule wce() -> &'input str
            = quiet! { $( whitespace() / comment() / eol_r() ) }
            / expected!("<newline> <whitespace> or <comment>")

        rule field_name() -> &'input str
            = $( ['a'..='z' | 'A'..='Z'] ( "_"? ['a'..='z' | 'A'..='Z' | '0'..='9'] )* )

        rule name() -> &'input str
            = $( ['A'..='Z']['a'..='z' | 'A'..='Z' | '0'..='9']* )

        rule interface_name() -> &'input str /* no hyphen at begin and end */
            = quiet! { $( ['a'..='z'] ( ['-']*['a'..='z' | '0'..='9'] )* ( ['.']['a'..='z' | '0'..='9'] (['-']*['a'..='z' | '0'..='9'])* )+ ) }
              / expected!("<reverse domain name>")

        rule array() -> ()
            = "[]"

        rule dict() -> ()
            = "[string]"

        rule option() -> ()
            = "?"

        use crate::VType;
        use crate::VTypeExt;

        rule btype() -> VTypeExt<'input>
            = "bool"    { VTypeExt::Plain(VType::Bool)}
            / "int"     { VTypeExt::Plain(VType::Int)}
            / "float"   { VTypeExt::Plain(VType::Float)}
            / "string"  { VTypeExt::Plain(VType::String)}
            / "object"  { VTypeExt::Plain(VType::Object)}
            / t:$(name()) { VTypeExt::Plain(VType::Typename(t))}
            / v:vstruct() { VTypeExt::Plain(VType::Struct(Box::new(v)))}
            / v:venum()   { VTypeExt::Plain(VType::Enum(Box::new(v)))}

        rule type_() -> VTypeExt<'input>
            = v:btype() { v }
            / a:array() v:type_() { VTypeExt::Array(Box::new(v)) }
            / a:dict()  v:type_() { VTypeExt::Dict(Box::new(v)) }
            / o:option() v:btype() { VTypeExt::Option(Box::new(v)) }
            / o:option() a:array() v:type_() { VTypeExt::Option(Box::new(VTypeExt::Array(Box::new(v)))) }
            / o:option() a:dict() v:type_() { VTypeExt::Option(Box::new(VTypeExt::Dict(Box::new(v)))) }

        use crate::Argument;
        rule object_field() -> Argument<'input>
            = wce()* n:$(field_name()) wce()* [':'] wce()* v:type_() { Argument { name : n, vtype : v } }

        use crate::VStruct;
        rule vstruct() -> VStruct<'input>
            = ['('] wce()* e:object_field() ** [','] wce()* [')'] { VStruct{ elts: e} }

        use crate::VEnum;
        rule venum() -> VEnum<'input>
            = ['('] wce()* v:field_name() ** ( [','] wce()* ) wce()*  [')'] { VEnum { elts: v } }

        use crate::Typedef;
        use crate::VStructOrEnum;
        use crate::trim_doc;

        rule vtypedef() -> Typedef<'input>
            = d:$(wce()*) "type" wce()+ n:$(name()) wce()* v:vstruct() {
                Typedef{name: n, doc: trim_doc(d), elt: VStructOrEnum::VStruct(Box::new(v))}
            }
            / d:$(wce()*) "type" wce()+ n:$(name()) wce()* v:venum() {
                Typedef{name: n, doc: trim_doc(d), elt: VStructOrEnum::VEnum(Box::new(v))}
            }

        use crate::VError;
        rule error() -> VError<'input>
            = d:$(wce()*) "error" wce()+ n:$(name()) wce()* p:vstruct() { VError{name: n, doc: trim_doc(d), parm: p} }

        use crate::Method;
        rule method() -> Method<'input>
            = d:$(wce()*) "method" wce()+ n:$(name()) wce()* i:vstruct() wce()* "->" wce()* o:vstruct() {
                Method {
                    name: n,
                    doc: trim_doc(d),
                    input: i,
                    output: o
                }
             }

        use crate::MethodOrTypedefOrError;
        rule member() -> MethodOrTypedefOrError<'input>
            = m:method() { MethodOrTypedefOrError::Method(m) }
            / t:vtypedef() { MethodOrTypedefOrError::Typedef(t) }
            / e:error() { MethodOrTypedefOrError::Error(e) }

        use crate::IDL;
        pub rule ParseInterface() -> IDL<'input>
            = d:$(wce()*) "interface" wce()+ n:$interface_name() eol() mt:(member()++ eol()) wce()*  {
                IDL::from_token(__input, n, mt, trim_doc(d))
             }

    }
}
