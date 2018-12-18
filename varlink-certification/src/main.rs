use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::env;
use std::hash::{Hash, Hasher};
use std::io;
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use chainerror::*;

pub type Result<T> = std::result::Result<T, Box<std::error::Error>>;

use varlink::{Connection, StringHashMap, StringHashSet, VarlinkService};

use crate::org_varlink_certification::*;

mod org_varlink_certification;
#[cfg(test)]
mod test; // Main

fn print_usage(program: &str, opts: &getopts::Options) {
    let brief = format!("Usage: {} [--varlink=<address>] [--client]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optopt("", "varlink", "varlink address URL", "<ADDRESS>");
    opts.optopt(
        "b",
        "bridge",
        "Command to execute and connect to",
        "<COMMAND>",
    );
    opts.optflag("", "client", "run in client mode");
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("", "timeout", "server timeout", "<seconds>");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            print_usage(&program, &opts);
            eprintln!("{}", f.to_string());
            exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return Ok(());
    }

    let client_mode = matches.opt_present("client");

    let timeout = matches
        .opt_str("timeout")
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(0);

    if client_mode {
        let connection = match matches.opt_str("varlink") {
            None => match matches.opt_str("bridge") {
                Some(bridge) => Connection::with_bridge(&bridge)
                    .map_err(mstrerr!("Connection::with_bridge({})", bridge))?,
                None => Connection::with_activate(&format!(
                    "{} \
                     --varlink=$VARLINK_ADDRESS",
                    program
                ))?,
            },
            Some(address) => Connection::with_address(&address)
                .map_err(mstrerr!("Connection::with_address({})", address))?,
        };
        run_client(connection)?
    } else if let Some(address) = matches.opt_str("varlink") {
        run_server(&address, timeout)?
    } else {
        print_usage(&program, &opts);
        eprintln!("Need varlink address in server mode.");
        exit(1);
    }

    Ok(())
}

// Client

fn run_client(connection: Arc<RwLock<varlink::Connection>>) -> Result<()> {
    let mut iface = VarlinkClient::new(connection);

    let ret = iface.start().call()?;
    eprintln!("{:#?}", ret);

    let client_id = ret.client_id;

    let ret = iface.test01(client_id.clone()).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test02(client_id.clone(), ret.bool).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test03(client_id.clone(), ret.int).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test04(client_id.clone(), ret.float).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test05(client_id.clone(), ret.string).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface
        .test06(client_id.clone(), ret.bool, ret.int, ret.float, ret.string)
        .call()?;
    eprintln!("{:#?}", ret);

    let ret = iface
        .test07(
            client_id.clone(),
            Test07_Args_struct {
                bool: ret.r#struct.bool,
                int: ret.r#struct.int,
                float: ret.r#struct.float,
                string: ret.r#struct.string,
            },
        )
        .call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test08(client_id.clone(), ret.map).call()?;
    eprintln!("{:#?}", ret);

    let ret = iface.test09(client_id.clone(), ret.set).call()?;
    eprintln!("{:#?}", ret);

    let mut ret_array = Vec::new();

    for ret in iface.test10(client_id.clone(), ret.mytype).more()? {
        let ret = ret?;
        eprintln!("{:#?}", ret);
        ret_array.push(ret.string.clone());
    }

    iface.test11(client_id.clone(), ret_array).oneway()?;

    let ret = iface.end(client_id.clone()).call()?;
    eprintln!("{:#?}", ret);

    Ok(())
}

// Server

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
        r#enum: MyType_enum::two,
        r#struct: MyType_struct {
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
            let got: serde_json::Value =
                serde_json::to_value($c.get_request().unwrap()).map_err(minto_cherr!())?;
            return $c.reply_certification_error(
                serde_json::to_value($wants).map_err(minto_cherr!())?,
                got,
            );
        }
    }};
}

macro_rules! check_call_normal {
    ($c:ident, $test:expr, $got:ty, $wants:expr) => {{
        let wants = $wants;
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
            }) if m == $test => {
                let v: ::std::result::Result<$got, serde_json::Error> =
                    serde_json::from_value(p.clone());
                match v {
                    Ok(w) => wants == w,
                    _ => false,
                }
            }

            _ => false,
        };
        if !check {
            let got: serde_json::Value =
                serde_json::to_value($c.get_request().unwrap()).map_err(minto_cherr!())?;
            let wants = serde_json::to_value(wants).map_err(minto_cherr!())?;
            return $c.reply_certification_error(
                serde_json::to_value(varlink::Request {
                    more: None,
                    oneway: None,
                    upgrade: None,
                    method: $test.into(),
                    parameters: Some(wants),
                })
                .map_err(minto_cherr!())?,
                got,
            );
        }
    }};
}

macro_rules! check_call_more {
    ($c:ident, $test:expr, $got:ty, $wants:expr) => {{
        let wants = $wants;
        let check = match $c.get_request() {
            Some(&varlink::Request {
                oneway: Some(true), ..
            })
            | Some(&varlink::Request {
                upgrade: Some(true),
                ..
            }) => false,
            Some(&varlink::Request {
                more: Some(true),
                method: ref m,
                parameters: Some(ref p),
                ..
            }) if m == $test => {
                let v: ::std::result::Result<$got, serde_json::Error> =
                    serde_json::from_value(p.clone());
                match v {
                    Ok(w) => wants == w,
                    _ => false,
                }
            }

            _ => false,
        };
        if !check {
            let got: serde_json::Value =
                serde_json::to_value($c.get_request().unwrap()).map_err(minto_cherr!())?;
            let wants = serde_json::to_value(wants).map_err(minto_cherr!())?;
            return $c.reply_certification_error(
                serde_json::to_value(varlink::Request {
                    more: None,
                    oneway: None,
                    upgrade: None,
                    method: $test.into(),
                    parameters: Some(wants),
                })
                .map_err(minto_cherr!())?,
                got,
            );
        }
    }};
}

macro_rules! check_call_oneway {
    ($c:ident, $test:expr, $got:ty, $wants:expr) => {{
        let wants = $wants;
        let check = match $c.get_request() {
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
                parameters: Some(ref p),
                ..
            }) if m == $test => {
                let v: ::std::result::Result<$got, serde_json::Error> =
                    serde_json::from_value(p.clone());
                match v {
                    Ok(w) => wants == w,
                    _ => false,
                }
            }

            _ => false,
        };
        if !check {
            let got: serde_json::Value =
                serde_json::to_value($c.get_request().unwrap()).map_err(minto_cherr!())?;
            let wants = serde_json::to_value(wants).map_err(minto_cherr!())?;
            return $c.reply_certification_error(
                serde_json::to_value(varlink::Request {
                    more: None,
                    oneway: None,
                    upgrade: None,
                    method: $test.into(),
                    parameters: Some(wants),
                })
                .map_err(minto_cherr!())?,
                got,
            );
        }
    }};
}

impl VarlinkInterface for CertInterface {
    fn start(&self, call: &mut Call_Start) -> varlink::Result<()> {
        check_call_expr!(
            call,
            match call.get_request() {
                Some(&varlink::Request {
                    more: Some(true), ..
                })
                | Some(&varlink::Request {
                    upgrade: Some(true),
                    ..
                })
                | Some(&varlink::Request {
                    oneway: Some(true), ..
                }) => false,
                Some(&varlink::Request {
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

        call.reply(self.new_client_id())
    }

    fn test01(&self, call: &mut Call_Test01, client_id: String) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test01", "Test02") {
            return call.reply_client_id_error();
        }

        check_call_normal!(
            call,
            "org.varlink.certification.Test01",
            Test01_Args,
            Test01_Args { client_id }
        );

        call.reply(true)
    }

    fn test02(
        &self,
        call: &mut Call_Test02,
        client_id: String,
        _bool_: bool,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test02", "Test03") {
            return call.reply_client_id_error();
        }

        check_call_normal!(
            call,
            "org.varlink.certification.Test02",
            Test02_Args,
            Test02_Args {
                client_id,
                bool: true,
            }
        );
        call.reply(1)
    }

    fn test03(&self, call: &mut Call_Test03, client_id: String, _int: i64) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test03", "Test04") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.Test03",
            Test03_Args,
            Test03_Args { client_id, int: 1 }
        );

        call.reply(1.0)
    }

    fn test04(
        &self,
        call: &mut Call_Test04,
        client_id: String,
        _float: f64,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test04", "Test05") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.Test04",
            Test04_Args,
            Test04_Args {
                client_id,
                float: 1.0,
            }
        );

        call.reply("ping".into())
    }

    fn test05(
        &self,
        call: &mut Call_Test05,
        client_id: String,
        _string: String,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test05", "Test06") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.Test05",
            Test05_Args,
            Test05_Args {
                client_id,
                string: "ping".into(),
            }
        );

        call.reply(false, 2, std::f64::consts::PI, "a lot of string".into())
    }

    fn test06(
        &self,
        call: &mut Call_Test06,
        client_id: String,
        _bool_: bool,
        _int: i64,
        _float: f64,
        _string: String,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test06", "Test07") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.Test06",
            Test06_Args,
            Test06_Args {
                client_id,
                bool: false,
                int: 2,
                float: std::f64::consts::PI,
                string: "a lot of string".into(),
            }
        );

        call.reply(Test06_Reply_struct {
            bool: false,
            int: 2,
            float: std::f64::consts::PI,
            string: "a lot of string".into(),
        })
    }

    fn test07(
        &self,
        call: &mut Call_Test07,
        client_id: String,
        _struct_: Test07_Args_struct,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test07", "Test08") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.Test07",
            Test07_Args,
            Test07_Args {
                client_id,
                r#struct: Test07_Args_struct {
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
        call: &mut Call_Test08,
        client_id: String,
        _map: ::std::collections::HashMap<String, String>,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test08", "Test09") {
            return call.reply_client_id_error();
        }
        let mut map: StringHashMap<String> = StringHashMap::new();
        map.insert("bar".into(), "Bar".into());
        map.insert("foo".into(), "Foo".into());

        check_call_normal!(
            call,
            "org.varlink.certification.Test08",
            Test08_Args,
            Test08_Args { client_id, map }
        );

        let mut set = StringHashSet::new();
        set.insert("one".into());
        set.insert("two".into());
        set.insert("three".into());
        call.reply(set)
    }

    fn test09(
        &self,
        call: &mut Call_Test09,
        client_id: String,
        _set: varlink::StringHashSet,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test09", "Test10") {
            return call.reply_client_id_error();
        }
        let mut set = StringHashSet::new();
        set.insert("one".into());
        set.insert("two".into());
        set.insert("three".into());

        check_call_normal!(
            call,
            "org.varlink.certification.Test09",
            Test09_Args,
            Test09_Args { client_id, set }
        );

        call.reply(new_mytype().map_err(minto_cherr!())?)
    }

    fn test10(
        &self,
        call: &mut Call_Test10,
        client_id: String,
        _mytype: MyType,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test10", "Test11") {
            return call.reply_client_id_error();
        }
        check_call_more!(
            call,
            "org.varlink.certification.Test10",
            Test10_Args,
            Test10_Args {
                client_id,
                mytype: new_mytype().map_err(minto_cherr!())?,
            }
        );

        call.set_continues(true);
        for i in 1..11 {
            if i == 10 {
                call.set_continues(false);
            }
            call.reply(format!("Reply number {}", i))?
        }
        Ok(())
    }

    fn test11(
        &self,
        call: &mut Call_Test11,
        client_id: String,
        _last_more_replies: Vec<String>,
    ) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "Test11", "End") {
            return call.reply_client_id_error();
        }
        let mut last_more_replies: Vec<String> = Vec::new();

        for i in 0..10 {
            last_more_replies.push(format!("Reply number {}", i + 1));
        }

        check_call_oneway!(
            call,
            "org.varlink.certification.Test11",
            Test11_Args,
            Test11_Args {
                client_id,
                last_more_replies,
            }
        );

        Ok(())
    }

    fn end(&self, call: &mut Call_End, client_id: String) -> varlink::Result<()> {
        if !self.check_client_id(&client_id, "End", "End") {
            return call.reply_client_id_error();
        }
        check_call_normal!(
            call,
            "org.varlink.certification.End",
            End_Args,
            End_Args { client_id }
        );

        call.reply(true)
    }
}

struct Context {
    test: String,
}

struct ClientIds {
    lifetimes: VecDeque<(Instant, String)>,
    contexts: StringHashMap<Context>,
    max_lifetime: u64,
}

impl ClientIds {
    fn check_client_id(&mut self, client_id: &str, test: &str, next_test: &str) -> bool {
        self.check_lifetime_timeout();

        match self.contexts.get_mut(client_id) {
            Some(context) => {
                if context.test != test {
                    false
                } else {
                    context.test = next_test.into();
                    true
                }
            }
            _ => false,
        }
    }

    fn check_lifetime_timeout(&mut self) {
        loop {
            let pop = match self.lifetimes.front() {
                None => false,

                Some(&(ref instant, ref client_id)) => {
                    if instant.elapsed().as_secs() > self.max_lifetime {
                        self.contexts.remove(client_id);
                        true
                    } else {
                        false
                    }
                }
            };

            if !pop {
                break;
            }
            self.lifetimes.pop_front();
        }
    }

    fn new_client_id(&mut self) -> String {
        let now = Instant::now();
        let mut hasher = DefaultHasher::new();
        format!("{:?}", now).hash(&mut hasher);
        let client_id = format!("{:x}", hasher.finish());
        self.contexts.insert(
            client_id.clone(),
            Context {
                test: "Test01".into(),
            },
        );
        self.lifetimes.push_back((now, client_id.clone()));
        client_id
    }
}

struct CertInterface {
    pub client_ids: Arc<RwLock<ClientIds>>,
}

impl CertInterface {
    fn check_client_id(&self, client_id: &str, test: &str, next_test: &str) -> bool {
        let mut client_ids = self.client_ids.write().unwrap();
        client_ids.check_client_id(client_id, test, next_test)
    }

    fn new_client_id(&self) -> String {
        let mut client_ids = self.client_ids.write().unwrap();
        client_ids.new_client_id()
    }
}

pub fn run_server(address: &str, timeout: u64) -> varlink::Result<()> {
    let certinterface = CertInterface {
        client_ids: Arc::new(RwLock::new(ClientIds {
            lifetimes: VecDeque::new(),
            contexts: StringHashMap::new(),
            max_lifetime: 60 * 60 * 12,
        })),
    };

    let myinterface = new(Box::new(certinterface));
    let service = VarlinkService::new(
        "org.varlink",
        "Varlink Certification Suite",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    if let Err(e) = varlink::listen(service, &address, 1, 10, timeout) {
        match e.kind() {
            ::varlink::ErrorKind::Timeout => {}
            _ => Err(e)?,
        }
    }
    Ok(())
}
