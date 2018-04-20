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

    let _ret = call.oneway().start()?;

    let ret = call.test01()?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test02(ret.bool)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test03(ret.int)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test04(ret.float)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test05(ret.string)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test06(ret.bool, ret.int, ret.float, ret.string)?
        .recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test07(Test07Args_struct {
        bool: ret.struct_.bool,
        int: ret.struct_.int,
        float: ret.struct_.float,
        string: ret.struct_.string,
    })?
        .recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test08(ret.map)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.test09(ret.set)?.recv()?;
    eprintln!("{:#?}", ret);

    let mut ret_array = Vec::new();

    for ret in call.more().test10(ret.mytype)? {
        let ret = ret?;
        eprintln!("{:#?}", ret);
        ret_array.push(ret.string.clone());
    }

    let ret = call.test11(ret_array)?.recv()?;
    eprintln!("{:#?}", ret);

    let ret = call.end()?.recv()?;
    eprintln!("{:#?}", ret);

    Ok(())
}

// Server

struct CertInterface;

fn new_mytype() -> io::Result<MyType> {
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

    Ok(MyType {
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
    })
}

macro_rules! check_call_expr {
	($c:ident, $pat:expr, $wants:expr) => {{
	    let check = $pat;
        if !check {
            let got: serde_json::Value = serde_json::to_value($c.get_request().unwrap())?;
            return $c.reply_certification_error(
                serde_json::to_value($wants)?,
                got,
            );
        }
	}};
}
macro_rules! check_call_normal {
	($c:ident, $test:expr, $wants:expr) => {{
	    let wants = serde_json::to_value($wants)?;
	    let check = match $c.get_request() {
            Some(&varlink::Request {
                more: Some(true), ..
            })
            | Some(&varlink::Request {
                oneway: Some(true), ..
            })
            | Some(&varlink::Request {
                upgrade: Some(true),
                ..
            }) => false,
            Some(&varlink::Request {
                method: ref m,
                parameters: Some(ref p),
                ..
            }) if m == $test
                && p == &wants =>
            {
                true
            }

            _ => false,
        };
        if !check {
            let got: serde_json::Value = serde_json::to_value($c.get_request().unwrap())?;
            return $c.reply_certification_error(
                    serde_json::to_value(varlink::Request {
                    more: None,
                    oneway: None,
                    upgrade: None,
                    method: "org.varlink.certification.$test".into(),
                    parameters: Some(wants),
                    }) ?,
                got,
            );
        }
	}};
}

impl VarlinkInterface for CertInterface {
    fn start(&self, call: &mut _CallStart) -> io::Result<()> {
        check_call_expr!(
            call,
            match call.get_request() {
                Some(&varlink::Request {
                    more: Some(true), ..
                })
                | Some(&varlink::Request {
                    upgrade: Some(true),
                    ..
                }) => false,
                Some(&varlink::Request {
                    oneway: Some(true),
                    method: ref m,
                    parameters: ref p,
                    ..
                }) if m == "org.varlink.certification.Start"
                    && (*p == None
                        || *p == Some(serde_json::Value::Object(serde_json::Map::new()))) =>
                {
                    true
                }

                _ => false,
            },
            varlink::Request {
                more: None,
                oneway: Some(true),
                upgrade: None,
                method: "org.varlink.certification.Start".into(),
                parameters: None,
            }
        );

        Ok(())
    }

    fn test01(&self, call: &mut _CallTest01) -> io::Result<()> {
        check_call_expr!(
            call,
            match call.get_request() {
                Some(&varlink::Request {
                    more: Some(true), ..
                })
                | Some(&varlink::Request {
                    oneway: Some(true), ..
                })
                | Some(&varlink::Request {
                    upgrade: Some(true),
                    ..
                }) => false,
                Some(&varlink::Request {
                    method: ref m,
                    parameters: ref p,
                    ..
                }) if m == "org.varlink.certification.Test01"
                    && (*p == None
                        || *p == Some(serde_json::Value::Object(serde_json::Map::new()))) =>
                {
                    true
                }

                _ => false,
            },
            varlink::Request {
                more: None,
                oneway: None,
                upgrade: None,
                method: "org.varlink.certification.Test01".into(),
                parameters: None,
            }
        );

        call.reply(true)
    }

    fn test02(&self, call: &mut _CallTest02, _bool_: bool) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test02",
            Test02Args_ { bool: true }
        );
        call.reply(1)
    }

    fn test03(&self, call: &mut _CallTest03, _int: i64) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test03",
            Test03Args_ { int: 1 }
        );

        call.reply(1.0)
    }

    fn test04(&self, call: &mut _CallTest04, _float: f64) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test04",
            Test04Args_ { float: 1.0 }
        );

        call.reply("ping".into())
    }

    fn test05(&self, call: &mut _CallTest05, _string: String) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test05",
            Test05Args_ {
                string: "ping".into(),
            }
        );

        call.reply(false, 2, std::f64::consts::PI, "a lot of string".into())
    }

    fn test06(
        &self,
        call: &mut _CallTest06,
        _bool_: bool,
        _int: i64,
        _float: f64,
        _string: String,
    ) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test06",
            Test06Args_ {
                bool: false,
                int: 2,
                float: std::f64::consts::PI,
                string: "a lot of string".into(),
            }
        );

        call.reply(Test06Reply_struct {
            bool: false,
            int: 2,
            float: std::f64::consts::PI,
            string: "a lot of string".into(),
        })
    }

    fn test07(&self, call: &mut _CallTest07, _struct_: Test07Args_struct) -> io::Result<()> {
        check_call_normal!(
            call,
            "org.varlink.certification.Test07",
            Test07Args_ {
                struct_: Test07Args_struct {
                    bool: false,
                    int: 2,
                    float: std::f64::consts::PI,
                    string: "a lot of string".into(),
                },
            }
        );

        let mut map: StringHashMap<String> = StringHashMap::new();
        map.insert("bar".into(), "Bar".into());
        map.insert("foo".into(), "Foo".into());
        call.reply(map)
    }

    fn test08(
        &self,
        call: &mut _CallTest08,
        _map: ::std::collections::HashMap<String, String>,
    ) -> io::Result<()> {
        let mut map: StringHashMap<String> = StringHashMap::new();
        map.insert("bar".into(), "Bar".into());
        map.insert("foo".into(), "Foo".into());

        check_call_normal!(
            call,
            "org.varlink.certification.Test08",
            Test08Args_ { map: map }
        );

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

        call.reply(new_mytype()?)
    }

    fn test10(&self, call: &mut _CallTest10, mytype: MyType) -> io::Result<()> {
        let mytype_wants = new_mytype()?;

        if mytype != mytype_wants || !call.wants_more() {
            let got: serde_json::Value = serde_json::to_value(call.get_request().unwrap())?;
            return call.reply_certification_error(
                serde_json::to_value(varlink::Request {
                    more: Some(true),
                    oneway: None,
                    upgrade: None,
                    method: "org.varlink.certification.Test10".into(),
                    parameters: Some(serde_json::to_value(mytype_wants)?),
                })?,
                got,
            );
        }

        call.set_continues(true);
        for i in 1..11 {
            if i == 10 {
                call.set_continues(false);
            }
            call.reply(format!("Reply number {}", i))?
        }
        Ok(())
    }

    fn test11(&self, call: &mut _CallTest11, last_more_replies: Vec<String>) -> io::Result<()> {
        if last_more_replies.len() != 10 {
            return call.reply_invalid_parameter("last_more_replies".into());
        }
        for i in 0..10 {
            match last_more_replies.get(i) {
                Some(s) if *s == format!("Reply number {}", i + 1) => {}
                _ => return call.reply_invalid_parameter("last_more_replies".into()),
            }
        }
        call.reply()
    }

    fn end(&self, call: &mut _CallEnd) -> io::Result<()> {
        call.reply(true)
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

    #[test]
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn test_unix_abstract() {
        assert!(run_self_test("unix:@org.varlink.certification".into()).is_ok());
    }

    #[test]
    fn test_tcp() {
        assert!(run_self_test("tcp:0.0.0.0:23456".into()).is_ok());
    }

    #[test]
    fn test_exec() {
        let address: String;

        if ::std::path::Path::new("../../target/debug/varlink-certification").exists() {
            address = "exec:../../target/debug/varlink-certification".into();
        } else if ::std::path::Path::new("./target/debug/varlink-certification").exists() {
            address = "exec:./target/debug/varlink-certification".into();
        } else {
            eprintln!("Skipping test, no varlink-certification");
            return;
        }

        assert!(::run_client(address.clone()).is_ok());
    }

    #[test]
    #[should_panic]
    fn test_wrong_address_1() {
        assert!(run_self_test("tcpd:0.0.0.0:12345".into()).is_ok());
    }
}
