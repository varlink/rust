extern crate getopts;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_varlink_certification::*;
use std::env;
use std::io;
use std::process::exit;
use varlink::{StringHashMap, StringHashSet, VarlinkService};

mod org_varlink_certification;

// Main

fn print_usage(program: &str, opts: getopts::Options) {
    let brief = format!("Usage: {} [--varlink=<address>] [--client]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optopt("", "varlink", "varlink address URL", "<address>");
    opts.optflag("", "client", "run in client mode");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            print_usage(&program, opts);
            return;
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let client_mode = matches.opt_present("client");

    let address = match matches.opt_str("varlink") {
        None => {
            if !client_mode {
                eprintln!("Need varlink address in server mode.");
                print_usage(&program, opts);
                return;
            }
            format!("exec:{}", program)
        }
        Some(a) => a,
    };

    let ret = match client_mode {
        true => run_client(address),
        false => run_server(address, 0),
    };

    exit(match ret {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

// Client

fn run_client(address: String) -> io::Result<()> {
    let connection = varlink::Connection::new(&address)?;
    let mut call = VarlinkClient::new(connection);

    let _ret = call.start()?.recv()?;

    let ret = call.test01()?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test02(ret.bool)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test03(ret.int)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test04(ret.float)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test05(ret.string)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test06(ret.bool, ret.int, ret.float, ret.string)?
        .recv()?;
    println!("{:#?}", ret);

    let ret = call.test07(Test07Args_struct {
        bool: ret.struct_.bool,
        int: ret.struct_.int,
        float: ret.struct_.float,
        string: ret.struct_.string,
    })?
        .recv()?;
    println!("{:#?}", ret);

    let ret = call.test08(ret.map)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.test09(ret.set)?.recv()?;
    println!("{:#?}", ret);

    let ret = call.end(ret.mytype)?.recv()?;
    println!("{:#?}", ret);

    Ok(())
}

// Server

struct CertInterface;

impl VarlinkInterface for CertInterface {
    fn start(&self, call: &mut _CallStart) -> io::Result<()> {
        call.reply()
    }

    fn test01(&self, call: &mut _CallTest01) -> io::Result<()> {
        call.reply(true)
    }

    fn test02(&self, call: &mut _CallTest02, bool_: bool) -> io::Result<()> {
        if bool_ != true {
            return call.reply_invalid_parameter("bool".into());
        }
        call.reply(1)
    }

    fn test03(&self, call: &mut _CallTest03, int: i64) -> io::Result<()> {
        if int != 1 {
            return call.reply_invalid_parameter("int".into());
        }
        call.reply(1.0)
    }

    fn test04(&self, call: &mut _CallTest04, float: f64) -> io::Result<()> {
        if float != 1.0 {
            return call.reply_invalid_parameter("float".into());
        }
        call.reply("ping".into())
    }

    fn test05(&self, call: &mut _CallTest05, string: String) -> io::Result<()> {
        if string != "ping" {
            return call.reply_invalid_parameter("string".into());
        }
        call.reply(false, 2, std::f64::consts::PI, "a lot of string".into())
    }

    fn test06(
        &self,
        call: &mut _CallTest06,
        bool_: bool,
        int: i64,
        float: f64,
        string: String,
    ) -> io::Result<()> {
        if bool_ != false {
            return call.reply_invalid_parameter("bool".into());
        }

        if int != 2 {
            return call.reply_invalid_parameter("int".into());
        }

        if float != std::f64::consts::PI {
            return call.reply_invalid_parameter("float".into());
        }

        if string != "a lot of string" {
            return call.reply_invalid_parameter("string".into());
        }

        call.reply(Test06Reply_struct {
            bool: false,
            int: 2,
            float: std::f64::consts::PI,
            string: "a lot of string".into(),
        })
    }

    fn test07(&self, call: &mut _CallTest07, struct_: Test07Args_struct) -> io::Result<()> {
        if struct_.bool != false {
            return call.reply_invalid_parameter("struct.bool".into());
        }

        if struct_.int != 2 {
            return call.reply_invalid_parameter("struct.int".into());
        }

        if struct_.float != std::f64::consts::PI {
            return call.reply_invalid_parameter("struct.float".into());
        }

        if struct_.string != "a lot of string" {
            return call.reply_invalid_parameter("struct.string".into());
        }

        let mut map: StringHashMap<String> = StringHashMap::new();
        map.insert("bar".into(), "Bar".into());
        map.insert("foo".into(), "Foo".into());
        call.reply(map)
    }

    fn test08(
        &self,
        call: &mut _CallTest08,
        map: ::std::collections::HashMap<String, String>,
    ) -> io::Result<()> {
        if map.len() != 2 || map.get("bar".into()) != Some(&String::from("Bar"))
            || map.get("foo".into()) != Some(&String::from("Foo"))
        {
            return call.reply_invalid_parameter("map".into());
        }

        let mut set = StringHashSet::new();
        set.insert("one".into());
        set.insert("two".into());
        set.insert("three".into());
        call.reply(set)
    }

    fn test09(&self, call: &mut _CallTest09, set: varlink::StringHashSet) -> io::Result<()> {
        if set.len() != 3 || set.get("one") == None || set.get("two") == None
            || set.get("three") == None
        {
            return call.reply_invalid_parameter("set".into());
        }

        let mut mytype_dictionary: StringHashMap<String> = StringHashMap::new();
        mytype_dictionary.insert("foo".into(), "Foo".into());
        mytype_dictionary.insert("bar".into(), "Bar".into());

        let mut mytype_stringset = varlink::StringHashSet::new();
        mytype_stringset.insert("one".into());
        mytype_stringset.insert("two".into());
        mytype_stringset.insert("three".into());

        let mut ele1: StringHashMap<Interface_foo> = StringHashMap::new();
        ele1.insert("foo".into(), Interface_foo::foo);
        ele1.insert("bar".into(), Interface_foo::bar);

        let mut ele2: StringHashMap<Interface_foo> = StringHashMap::new();
        ele2.insert("one".into(), Interface_foo::foo);
        ele2.insert("two".into(), Interface_foo::bar);

        let mytype = MyType {
            object: serde_json::from_str(
                r#"{"method": "org.varlink.certification.Test09",
                       "parameters": {"map": {"foo": "Foo", "bar": "Bar"}}}"#,
            )?,
            enum_: MyType_enum::two,
            struct_: MyType_struct {
                first: 1,
                second: "2".into(),
            },
            array: vec!["one".into(), "two".into(), "three".into()],
            dictionary: mytype_dictionary,
            stringset: mytype_stringset,
            nullable: None,
            nullable_array_struct: None,
            interface: Interface {
                foo: Some(vec![None, Some(ele1), None, Some(ele2)]),
                anon: Interface_anon {
                    foo: true,
                    bar: false,
                },
            },
        };

        call.reply(mytype)
    }

    fn end(&self, call: &mut _CallEnd, mytype: MyType) -> io::Result<()> {
        if mytype.dictionary.len() != 2
            || mytype.dictionary.get("bar".into()) != Some(&String::from("Bar"))
            || mytype.dictionary.get("foo".into()) != Some(&String::from("Foo"))
        {
            return call.reply_invalid_parameter("mytype.dictionary".into());
        }

        if mytype.stringset.len() != 3 || mytype.stringset.get("one") == None
            || mytype.stringset.get("two") == None
            || mytype.stringset.get("three") == None
        {
            return call.reply_invalid_parameter("mytype.stringset".into());
        }

        if mytype.array.len() != 3 || mytype.array.get(0) != Some(&String::from("one"))
            || mytype.array.get(1) != Some(&String::from("two"))
            || mytype.array.get(2) != Some(&String::from("three"))
        {
            return call.reply_invalid_parameter("mytype.array".into());
        }

        call.reply()
    }
}

fn run_server(address: String, timeout: u64) -> io::Result<()> {
    let certinterface = CertInterface;
    let myinterface = new(Box::new(certinterface));
    let service = VarlinkService::new(
        "org.varlink",
        "Varlink Certification Suite",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );
    varlink::listen(service, &address, 10, timeout)
}

#[cfg(test)]
mod test {
    use std::io;
    use std::{thread, time};

    fn run_self_test(address: String) -> io::Result<()> {
        let client_address = address.clone();

        let child = thread::spawn(move || {
            if let Err(e) = ::run_server(address, 4) {
                panic!("error: {}", e);
            }
        });

        // give server time to start
        thread::sleep(time::Duration::from_secs(1));

        let ret = ::run_client(client_address);
        if let Err(e) = ret {
            panic!("error: {}", e);
        }
        if let Err(e) = child.join() {
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("{:#?}", e),
            ))
        } else {
            Ok(())
        }
    }

    #[test]
    fn test_unix() {
        assert!(run_self_test("unix:/tmp/org.varlink.certification".into()).is_ok());
    }
}
