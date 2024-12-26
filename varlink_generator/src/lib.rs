//! Generate rust code from varlink interface definition files
//!
//! To create a varlink program in rust, place your varlink interface definition file in src/.
//! E.g. `src/org.example.ping.varlink`:
//!
//! ```varlink
//! # Example service
//! interface org.example.ping
//!
//! # Returns the same string
//! method Ping(ping: string) -> (pong: string)
//! ```
//!
//! Add `varlink_generator` to your Cargo.toml `[build-dependencies]`.
//!
//! Then create a `build.rs` file in your project directory using [`varlink_generator::cargo_build_tosource`]:
//!
//! ```rust,no_run
//! extern crate varlink_generator;
//!
//! fn main() {
//!     varlink_generator::cargo_build_tosource("src/org.example.ping.varlink",
//!                                              /* rustfmt */ true);
//! }
//! ```
//! [`varlink_generator::cargo_build_tosource`]: fn.cargo_build_tosource.html

#![recursion_limit = "512"]
#![doc(
    html_logo_url = "https://varlink.org/images/varlink.png",
    html_favicon_url = "https://varlink.org/images/varlink-small.png"
)]

use std::borrow::Cow;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::str::FromStr;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use varlink_parser::{Typedef, VEnum, VError, VStruct, VStructOrEnum, VType, VTypeExt, IDL};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Parse(varlink_parser::Error),
    #[error("I/O error: {0}")]
    Io(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

trait ToRustString<'short, 'long: 'short> {
    fn to_rust_string(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) -> Cow<'long, str>;
}

trait ToTokenStream<'short, 'long: 'short> {
    fn to_tokenstream(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    );
}

#[derive(Default)]
pub struct GeneratorOptions {
    pub bool_type: Option<&'static str>,
    pub int_type: Option<&'static str>,
    pub float_type: Option<&'static str>,
    pub string_type: Option<&'static str>,
    pub preamble: Option<TokenStream>,
}

impl<'short, 'long: 'short> ToRustString<'short, 'long> for VType<'long> {
    fn to_rust_string(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) -> Cow<'long, str> {
        match *self {
            VType::Bool => options.bool_type.unwrap_or("bool").into(),
            VType::Int => options.int_type.unwrap_or("i64").into(),
            VType::Float => options.float_type.unwrap_or("f64").into(),
            VType::String => options.string_type.unwrap_or("String").into(),
            VType::Object => "serde_json::Value".into(),
            VType::Typename(v) => v.into(),
            VType::Enum(ref v) => {
                v.to_tokenstream(name, tokenstream, options);
                Cow::Owned(name.to_string())
            }
            VType::Struct(ref v) => {
                v.to_tokenstream(name, tokenstream, options);
                Cow::Owned(name.to_string())
            }
        }
    }
}

impl<'short, 'long: 'short> ToRustString<'short, 'long> for VTypeExt<'long> {
    fn to_rust_string(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) -> Cow<'long, str> {
        match *self {
            VTypeExt::Plain(ref vtype) => vtype.to_rust_string(name, tokenstream, options),
            VTypeExt::Array(ref v) => {
                format!("Vec<{}>", v.to_rust_string(name, tokenstream, options)).into()
            }
            VTypeExt::Dict(ref v) => match *v.as_ref() {
                VTypeExt::Plain(VType::Struct(ref s)) if s.elts.is_empty() => {
                    "varlink::StringHashSet".into()
                }
                _ => format!(
                    "varlink::StringHashMap<{}>",
                    v.to_rust_string(name, tokenstream, options)
                )
                .into(),
            },
            VTypeExt::Option(ref v) => {
                format!("Option<{}>", v.to_rust_string(name, tokenstream, options)).into()
            }
        }
    }
}

fn to_snake_case(mut str: &str) -> String {
    let mut words = vec![];
    // Preserve leading underscores
    str = str.trim_start_matches(|c: char| {
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

impl<'short, 'long: 'short> ToTokenStream<'short, 'long> for VStruct<'long> {
    fn to_tokenstream(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) {
        let tname: Ident = format_ident!("r#{}", name);

        let mut enames = vec![];
        let mut etypes = vec![];
        for e in &self.elts {
            let ename_ident: Ident = syn::parse_str(&(String::from("r#") + e.name)).unwrap();
            enames.push(ename_ident);
            etypes.push(
                TokenStream::from_str(
                    e.vtype
                        .to_rust_string(
                            format!("{}_{}", name, e.name).as_ref(),
                            tokenstream,
                            options,
                        )
                        .as_ref(),
                )
                .unwrap(),
            );
        }
        tokenstream.extend(quote!(
            #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
            pub struct #tname {
                #(pub #enames: #etypes,)*
            }
        ));
    }
}

impl<'short, 'long: 'short> ToTokenStream<'short, 'long> for VEnum<'long> {
    fn to_tokenstream(
        &'long self,
        name: &str,
        tokenstream: &mut TokenStream,
        _options: &'long GeneratorOptions,
    ) {
        let tname: Ident = syn::parse_str(&(String::from("r#") + name)).unwrap();

        let mut enames = vec![];

        for elt in &self.elts {
            let ename_ident: Ident = syn::parse_str(&(String::from("r#") + elt)).unwrap();
            enames.push(ename_ident);
        }
        tokenstream.extend(quote!(
            #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
            pub enum #tname {
                #(#enames, )*
            }
        ));
    }
}

impl<'short, 'long: 'short> ToTokenStream<'short, 'long> for Typedef<'long> {
    fn to_tokenstream(
        &'long self,
        _name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) {
        match self.elt {
            VStructOrEnum::VStruct(ref v) => v.to_tokenstream(self.name, tokenstream, options),
            VStructOrEnum::VEnum(ref v) => v.to_tokenstream(self.name, tokenstream, options),
        }
    }
}

impl<'short, 'long: 'short> ToTokenStream<'short, 'long> for VError<'long> {
    fn to_tokenstream(
        &'long self,
        _name: &str,
        tokenstream: &mut TokenStream,
        options: &'long GeneratorOptions,
    ) {
        let args_name = Ident::new(&format!("{}_Args", self.name), Span::call_site());
        let mut args_enames = vec![];
        let mut args_etypes = vec![];
        let mut args_anot = vec![];

        for e in &self.parm.elts {
            args_anot.push(if let VTypeExt::Option(_) = e.vtype {
                quote!(#[serde(skip_serializing_if = "Option::is_none")])
            } else {
                quote!()
            });
            let ename_ident: Ident = syn::parse_str(&(String::from("r#") + e.name)).unwrap();
            args_enames.push(ename_ident);
            args_etypes.push(
                TokenStream::from_str(
                    e.vtype
                        .to_rust_string(
                            format!("{}_Args_{}", self.name, e.name).as_ref(),
                            tokenstream,
                            options,
                        )
                        .as_ref(),
                )
                .unwrap(),
            );
        }
        tokenstream.extend(quote!(
            #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
            pub struct #args_name {
                #(#args_anot pub #args_enames: #args_etypes,)*
            }
        ));
    }
}

fn varlink_to_rust(idl: &IDL, options: &GeneratorOptions, tosource: bool) -> Result<TokenStream> {
    let mut ts = TokenStream::new();

    if tosource {
        ts.extend(quote!(
            #![doc = "This file was automatically generated by the varlink rust generator" ]
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
        ));
    }

    ts.extend(quote!(
        use serde_derive::{Deserialize, Serialize};
        use std::io::BufRead;
        use std::sync::{Arc, RwLock};
        use varlink::{self, CallTrait};
    ));

    if let Some(ref v) = options.preamble {
        ts.extend(v.clone());
    }

    generate_error_code(options, idl, &mut ts);

    for t in idl.typedefs.values() {
        t.to_tokenstream("", &mut ts, options);
    }

    for t in idl.errors.values() {
        t.to_tokenstream("", &mut ts, options);
    }

    let mut server_method_decls = TokenStream::new();
    let mut client_method_decls = TokenStream::new();
    let mut server_method_impls = TokenStream::new();
    let mut client_method_impls = TokenStream::new();
    let iname = idl.name;
    let description = idl.description;

    for t in idl.methods.values() {
        let mut in_field_types = Vec::new();
        let mut in_field_names = Vec::new();
        let in_struct_name = Ident::new(&format!("{}_Args", t.name), Span::call_site());
        let mut in_anot: Vec<TokenStream> = Vec::new();

        let mut out_field_types = Vec::new();
        let mut out_field_names = Vec::new();
        let out_struct_name = Ident::new(&format!("{}_Reply", t.name), Span::call_site());
        let mut out_anot: Vec<TokenStream> = Vec::new();

        let call_name = Ident::new(&format!("Call_{}", t.name), Span::call_site());
        let method_name = Ident::new(&to_snake_case(t.name), Span::call_site());
        let varlink_method_name = format!("{}.{}", idl.name, t.name);

        generate_anon_struct(
            &format!("{}_{}", t.name, "Args"),
            &t.input,
            options,
            &mut ts,
            &mut in_field_types,
            &mut in_field_names,
            &mut in_anot,
        );

        generate_anon_struct(
            &format!("{}_{}", t.name, "Reply"),
            &t.output,
            options,
            &mut ts,
            &mut out_field_types,
            &mut out_field_names,
            &mut out_anot,
        );

        {
            let out_field_names = out_field_names.iter();
            let out_field_types = out_field_types.iter();
            let in_field_names = in_field_names.iter();
            let in_field_types = in_field_types.iter();

            ts.extend(quote!(
                #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
                pub struct #out_struct_name {
                                #(#out_anot pub #out_field_names: #out_field_types,)*
                }

                impl varlink::VarlinkReply for #out_struct_name {}

                #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
                pub struct #in_struct_name {
                                #(#in_anot pub #in_field_names: #in_field_types,)*
                }
            ));
        }

        {
            let field_names_1 = out_field_names.iter();
            let field_names_2 = out_field_names.iter();
            let field_types_1 = out_field_types.iter();
            if !t.output.elts.is_empty() {
                ts.extend(quote!(
                #[allow(dead_code)]
                pub trait #call_name: VarlinkCallError {
                    fn reply(&mut self, #(#field_names_1: #field_types_1),*) -> varlink::Result<()> {
                        self.reply_struct(#out_struct_name { #(#field_names_2),* }.into())
                    }
                }
            ));
            } else {
                ts.extend(quote!(
                    #[allow(dead_code)]
                    pub trait #call_name: VarlinkCallError {
                        fn reply(&mut self) -> varlink::Result<()> {
                            self.reply_struct(varlink::Reply::parameters(None))
                        }
                    }
                ));
            }
        }

        ts.extend(quote!(
            impl #call_name for varlink::Call<'_> {}
        ));

        // #server_method_decls
        {
            let in_field_names = in_field_names.iter();
            let in_field_types = in_field_types.iter();
            server_method_decls.extend(quote!(
                fn #method_name (&self, call: &mut dyn #call_name, #(#in_field_names: #in_field_types),*) ->
                varlink::Result<()>;
            ));
        }

        // #client_method_decls
        {
            let in_field_names = in_field_names.iter();
            let in_field_types = in_field_types.iter();
            client_method_decls.extend(quote!(
                fn #method_name(&mut self, #(#in_field_names: #in_field_types),*) ->
                varlink::MethodCall<#in_struct_name, #out_struct_name, Error>;
            ));
        }

        // #client_method_impls
        {
            let in_field_names_2 = in_field_names.iter();
            let in_field_names = in_field_names.iter();
            let in_field_types = in_field_types.iter();

            client_method_impls.extend(quote!(
            fn #method_name(&mut self, #(#in_field_names: #in_field_types),*) -> varlink::MethodCall<#in_struct_name, #out_struct_name,
            Error> {
             varlink::MethodCall::<#in_struct_name, #out_struct_name, Error>::new(
                self.connection.clone(),
                #varlink_method_name,
                #in_struct_name {#(#in_field_names_2),*})
             }
            ));
        }

        // #server_method_impls
        {
            let in_field_names = in_field_names.iter();

            if !t.input.elts.is_empty() {
                server_method_impls.extend(quote!(
                    #varlink_method_name => {
                        if let Some(args) = req.parameters.clone() {
                            let args: #in_struct_name = match serde_json::from_value(args) {
                                Ok(v) => v,
                                Err(e) => {
                                    let es = format!("{}", e);
                                    let _ = call.reply_invalid_parameter(es.clone());
                                    return Err(varlink::context!(varlink::ErrorKind::SerdeJsonDe(es)));
                                }
                            };
                            self.inner.#method_name(call as &mut dyn #call_name, #(args.#in_field_names),*)
                        } else {
                            call.reply_invalid_parameter("parameters".into())
                        }
                    },
                ));
            } else {
                server_method_impls.extend(quote!(
                    #varlink_method_name => self.inner.#method_name(call as &mut dyn #call_name),
                ));
            }
        }
    }

    ts.extend(quote!(
        #[allow(dead_code)]
        pub trait VarlinkInterface {
            #server_method_decls

            fn call_upgraded(&self, _call: &mut varlink::Call, _bufreader: &mut dyn BufRead) -> varlink::Result<Vec<u8>> {
                Ok(Vec::new())
            }
        }

        #[allow(dead_code)]
        pub trait VarlinkClientInterface {
            #client_method_decls
        }

        #[allow(dead_code)]
        pub struct VarlinkClient {
            connection: Arc<RwLock<varlink::Connection>>,
        }

        impl VarlinkClient {
            #[allow(dead_code)]
            pub fn new(connection: Arc<RwLock<varlink::Connection>>) -> Self {
                VarlinkClient {
                    connection,
                }
            }
        }

        impl VarlinkClientInterface for VarlinkClient {
            #client_method_impls
        }

        #[allow(dead_code)]
        pub struct VarlinkInterfaceProxy {
            inner: Box<dyn VarlinkInterface + Send + Sync>,
        }

        #[allow(dead_code)]
        pub fn new(inner: Box<dyn VarlinkInterface + Send + Sync>) -> VarlinkInterfaceProxy {
            VarlinkInterfaceProxy { inner }
        }

        impl varlink::Interface for VarlinkInterfaceProxy {
            fn get_description(&self) -> &'static str {
                #description
            }

            fn get_name(&self) -> &'static str {
                #iname
            }

            fn call_upgraded(&self, call: &mut varlink::Call, bufreader: &mut dyn BufRead) -> varlink::Result<Vec<u8>> {
                self.inner.call_upgraded(call, bufreader)
            }

            fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {
                let req = call.request.unwrap();
                match req.method.as_ref() {
                    #server_method_impls
                    m => {
                        call.reply_method_not_found(String::from(m))
                    }
                }
            }
        }
    ));

    Ok(ts)
}

fn generate_anon_struct(
    name: &str,
    vstruct: &VStruct,
    options: &GeneratorOptions,
    ts: &mut TokenStream,
    field_types: &mut Vec<TokenStream>,
    field_names: &mut Vec<Ident>,
    anot: &mut Vec<TokenStream>,
) {
    for e in &vstruct.elts {
        anot.push(if let VTypeExt::Option(_) = e.vtype {
            quote!(#[serde(skip_serializing_if = "Option::is_none")])
        } else {
            quote!()
        });
        let ename_ident: Ident = syn::parse_str(&(String::from("r#") + e.name)).unwrap();
        field_names.push(ename_ident);
        field_types.push(
            TokenStream::from_str(
                e.vtype
                    .to_rust_string(format!("{}_{}", name, e.name).as_ref(), ts, options)
                    .as_ref(),
            )
            .unwrap(),
        );
    }
}

fn generate_error_code(
    options: &GeneratorOptions,
    idl: &varlink_parser::IDL,
    ts: &mut TokenStream,
) {
    // Errors traits
    {
        let mut error_structs_and_enums = TokenStream::new();
        let mut funcs = TokenStream::new();
        {
            let mut errors = Vec::new();
            let mut errors_display = Vec::new();
            for t in idl.errors.values() {
                errors.push(
                    TokenStream::from_str(&format!(
                        "{ename}(Option<{ename}_Args>)",
                        ename = t.name,
                    ))
                    .unwrap(),
                );
                errors_display.push(
                    TokenStream::from_str(&format!(
                        "ErrorKind::{ename}(v) => write!(f, \"{iname}.{ename}: {{:#?}}\", v)",
                        ename = t.name,
                        iname = idl.name,
                    ))
                    .unwrap(),
                );
            }

            ts.extend(quote!(
                #[allow(dead_code)]
                #[derive(Clone, PartialEq, Debug)]
                #[allow(clippy::enum_variant_names)]
                pub enum ErrorKind {
                    Varlink_Error,
                    VarlinkReply_Error,
                    #(#errors),*
                }
                impl ::std::fmt::Display for ErrorKind {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        match self {
                            ErrorKind::Varlink_Error => write!(f, "Varlink Error"),
                            ErrorKind::VarlinkReply_Error => write!(f, "Varlink error reply"),
                            #(#errors_display),*
                        }
                    }
                }
            ));
        }
        ts.extend(quote!(
        pub struct Error(
            pub ErrorKind,
            pub Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
            pub Option<&'static str>,
        );

        impl Error {
            #[allow(dead_code)]
            pub fn kind(&self) -> &ErrorKind {
                &self.0
            }
        }

        impl From<ErrorKind> for Error {
            fn from(e: ErrorKind) -> Self {
                Error(e, None, None)
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.1.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::fmt::Debug for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                use std::error::Error as StdError;

                if let Some(ref o) = self.2 {
                    std::fmt::Display::fmt(o, f)?;
                }

                std::fmt::Debug::fmt(&self.0, f)?;
                if let Some(e) = self.source() {
                    std::fmt::Display::fmt("\nCaused by:\n", f)?;
                    std::fmt::Debug::fmt(&e, f)?;
                }
                Ok(())
            }
        }

        #[allow(dead_code)]
        pub type Result<T> = std::result::Result<T, Error>;

        impl From<varlink::Error> for Error {
            fn from(
                e: varlink::Error,
            ) -> Self {
                match e.kind() {
                    varlink::ErrorKind::VarlinkErrorReply(r) => Error(ErrorKind::from(r), Some(Box::from(e)), Some(concat!(file!(), ":", line!(), ": "))),
                    _  => Error(ErrorKind::Varlink_Error, Some(Box::from(e)), Some(concat!(file!(), ":", line!(), ": ")))
                }
            }
        }

        #[allow(dead_code)]
        impl Error {
            pub fn source_varlink_kind(&self) -> Option<&varlink::ErrorKind> {
                use std::error::Error as StdError;
                let mut s: &dyn StdError = self;
                while let Some(c) = s.source() {
                    let k = self
                        .source()
                        .and_then(|e| e.downcast_ref::<varlink::Error>())
                        .map(|e| e.kind());

                    if k.is_some() {
                        return k;
                    }

                    s = c;
                }
                None
            }
        }
    ));
        {
            let mut arms = TokenStream::new();
            for t in idl.errors.values() {
                let error_name = format!("{iname}.{ename}", iname = idl.name, ename = t.name);
                let ename = TokenStream::from_str(&format!("ErrorKind::{}", t.name)).unwrap();
                arms.extend(quote!(
                    varlink::Reply { error: Some(ref t), .. } if t == #error_name => {
                        match e {
                           varlink::Reply {
                               parameters: Some(p),
                               ..
                           } => match serde_json::from_value(p.clone()) {
                               Ok(v) => #ename(v),
                               Err(_) => #ename(None),
                           },
                           _ => #ename(None),
                        }
                    }
                ));
            }

            ts.extend(quote!(
                impl From<&varlink::Reply> for ErrorKind {
                    #[allow(unused_variables)]
                    fn from(e: &varlink::Reply) -> Self {
                        match e {
                        #arms
                        _ => ErrorKind::VarlinkReply_Error,
                        }
                    }
                }
            ));
        }
        for t in idl.errors.values() {
            let mut inparms_name = Vec::new();
            let mut inparms_type = Vec::new();

            let inparms;
            let parms;
            let args_name = Ident::new(&format!("{}_Args", t.name), Span::call_site());
            if !t.parm.elts.is_empty() {
                for e in &t.parm.elts {
                    let ename_ident: Ident =
                        syn::parse_str(&(String::from("r#") + e.name)).unwrap();
                    inparms_name.push(ename_ident);
                    inparms_type.push(
                        TokenStream::from_str(
                            e.vtype
                                .to_rust_string(
                                    format!("{}_Args_{}", t.name, e.name).as_ref(),
                                    &mut error_structs_and_enums,
                                    options,
                                )
                                .as_ref(),
                        )
                        .unwrap(),
                    );
                }
                let innames = inparms_name.iter();
                let innames2 = inparms_name.iter();
                inparms = quote!(#(#innames : #inparms_type),*);
                parms = quote!(Some(serde_json::to_value(#args_name {#(#innames2),*}).map_err(varlink::map_context!())?));
            } else {
                parms = quote!(None);
                inparms = quote!();
            }
            let errorname = format!("{iname}.{ename}", iname = idl.name, ename = t.name);
            let func_name = Ident::new(
                &format!("reply_{}", to_snake_case(t.name)),
                Span::call_site(),
            );

            funcs.extend(quote!(
                fn #func_name(&mut self, #inparms) -> varlink::Result<()> {
                    self.reply_struct(varlink::Reply::error(#errorname, #parms))
                }
            ));
        }
        ts.extend(quote!(
            #error_structs_and_enums
            #[allow(dead_code)]
            pub trait VarlinkCallError: varlink::CallTrait {
                #funcs
            }
        ));
    }
    ts.extend(quote!(
        impl VarlinkCallError for varlink::Call<'_> {}
    ));
}

pub fn compile(source: String) -> Result<TokenStream> {
    let idl = IDL::try_from(source.as_str()).map_err(Error::Parse)?;
    varlink_to_rust(
        &idl,
        &GeneratorOptions {
            ..Default::default()
        },
        true,
    )
}

/// `generate` reads a varlink interface definition from `reader` and writes
/// the rust code to `writer`.
pub fn generate(reader: &mut dyn Read, writer: &mut dyn Write, tosource: bool) -> Result<()> {
    generate_with_options(
        reader,
        writer,
        &GeneratorOptions {
            ..Default::default()
        },
        tosource,
    )
}

/// `generate_with_options` reads a varlink interface definition from `reader`
/// and writes the rust code to `writer`.
pub fn generate_with_options(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    options: &GeneratorOptions,
    tosource: bool,
) -> Result<()> {
    let mut buffer = String::new();

    reader.read_to_string(&mut buffer).map_err(Error::Io)?;

    let idl = IDL::try_from(buffer.as_str()).map_err(Error::Parse)?;
    let ts = varlink_to_rust(&idl, options, tosource)?;

    writer
        .write_all(ts.to_string().as_bytes())
        .map_err(Error::Io)
}

/// cargo build helper function
///
/// `cargo_build` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build("src/org.example.ping.varlink");
/// }
/// ```
///
pub fn cargo_build<T: AsRef<Path> + ?Sized>(input_path: &T) {
    cargo_build_options_many(
        &[input_path],
        &GeneratorOptions {
            ..Default::default()
        },
    )
}

/// cargo build helper function
///
/// `cargo_build_many` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build_many(&[
///         "src/org.example.ping.varlink",
///         "src/org.example.more.varlink",
///     ]);
/// }
/// ```
///
pub fn cargo_build_many<T>(input_paths: &[T])
where
    T: std::marker::Sized,
    T: AsRef<Path>,
{
    cargo_build_options_many(
        input_paths,
        &GeneratorOptions {
            ..Default::default()
        },
    )
}

/// cargo build helper function
///
/// `cargo_build_options` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build_options(
///         "src/org.example.ping.varlink",
///         &varlink_generator::GeneratorOptions {
///             int_type: Some("i128"),
///             ..Default::default()
///         },
///     );
/// }
/// ```
pub fn cargo_build_options<T: AsRef<Path> + ?Sized>(input_path: &T, options: &GeneratorOptions) {
    cargo_build_options_many(&[input_path], options)
}

/// cargo build helper function
///
/// `cargo_build_options_many` is used in a `build.rs` program to build the rust code
/// from a varlink interface definition.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build_options_many(
///         &[
///             "src/org.example.ping.varlink",
///             "src/org.example.more.varlink",
///         ],
///         &varlink_generator::GeneratorOptions {
///             int_type: Some("i128"),
///             ..Default::default()
///         },
///     );
/// }
/// ```
pub fn cargo_build_options_many<T>(input_paths: &[T], options: &GeneratorOptions)
where
    T: std::marker::Sized,
    T: AsRef<Path>,
{
    for input_path in input_paths {
        let input_path = input_path.as_ref();

        let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
        let rust_path = out_dir
            .join(input_path.file_name().unwrap())
            .with_extension("rs");

        let writer: &mut dyn Write = &mut (File::create(&rust_path).unwrap_or_else(|e| {
            eprintln!(
                "Could not open varlink output file `{}`: {}",
                rust_path.display(),
                e
            );
            exit(1);
        }));

        let reader: &mut dyn Read = &mut (File::open(input_path).unwrap_or_else(|e| {
            eprintln!(
                "Could not read varlink input file `{}`: {}",
                input_path.display(),
                e
            );
            exit(1);
        }));

        if let Err(e) = generate_with_options(reader, writer, options, false) {
            eprintln!(
                "Could not generate rust code from varlink file `{}`: {}",
                input_path.display(),
                e,
            );

            exit(1);
        }

        println!("cargo:rerun-if-changed={}", input_path.display());
    }
}

/// cargo build helper function
///
/// `cargo_build_tosource` is used in a `build.rs` program to build the rust
/// code from a varlink interface definition. This function saves the rust code
/// in the same directory as the varlink file. The name is the name of the
/// varlink file and "." replaced with "_" and of course ending with ".rs".
///
/// Use this, if you are using an IDE with code completion, as most cannot cope
/// with `include!(concat!(env!("OUT_DIR"), "<varlink_file>"));`
///
/// Set `rustfmt` to `true`, if you want the generator to run rustfmt on the
/// generated code. This might be good practice to avoid large changes after a
/// global `cargo fmt` run.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build_tosource("src/org.example.ping.varlink", true);
/// }
/// ```
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
/// `cargo_build_tosource_options` is used in a `build.rs` program to build the
/// rust code from a varlink interface definition. This function saves the rust
/// code in the same directory as the varlink file. The name is the name of the
/// varlink file and "." replaced with "_" and of course ending with ".rs".
///
/// Use this, if you are using an IDE with code completion, as most cannot cope
/// with `include!(concat!(env!("OUT_DIR"), "<varlink_file>"));`
///
/// Set `rustfmt` to `true`, if you want the generator to run rustfmt on the
/// generated code. This might be good practice to avoid large changes after a
/// global `cargo fmt` run.
///
/// Errors are emitted to stderr and terminate the process.
///
/// # Examples
///
/// ```rust,no_run
/// extern crate varlink_generator;
///
/// fn main() {
///     varlink_generator::cargo_build_tosource_options(
///         "src/org.example.ping.varlink",
///         true,
///         &varlink_generator::GeneratorOptions {
///             int_type: Some("i128"),
///             ..Default::default()
///         },
///     );
/// }
/// ```
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
        .replace('.', "_");
    let rust_path = input_path
        .parent()
        .unwrap()
        .join(Path::new(&newfilename).with_extension("rs"));

    let writer: &mut dyn Write = &mut (File::create(&rust_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not open varlink output file `{}`: {}",
            rust_path.display(),
            e
        );
        exit(1);
    }));

    let reader: &mut dyn Read = &mut (File::open(input_path).unwrap_or_else(|e| {
        eprintln!(
            "Could not read varlink input file `{}`: {}",
            input_path.display(),
            e
        );
        exit(1);
    }));

    if let Err(e) = generate_with_options(reader, writer, options, true) {
        eprintln!(
            "Could not generate rust code from varlink file `{}`: {}",
            input_path.display(),
            e,
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
