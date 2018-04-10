#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_complex::*;
use std::env;
use std::io;
use std::io::Error;
use std::process::exit;
use varlink::VarlinkService;

mod org_example_complex;

struct MyImplementation;

impl org_example_complex::VarlinkInterface for MyImplementation {
    fn bar(&self, call: &mut _CallBar) -> Result<(), Error> {
        call.reply_method_not_implemented(None)
    }

    fn foo(
        &self,
        call: &mut _CallFoo,
        _enum_: FooArgs_enum,
        _foo: TypeFoo,
        _interface: Interface,
    ) -> Result<(), Error> {
        call.reply_method_not_implemented(None)
    }
}

fn run_app(address: String, timeout: u64) -> io::Result<()> {
    let myimplementation = MyImplementation;
    let myinterface = org_example_complex::new(Box::new(myimplementation));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );

    varlink::listen(service, &address, 10, timeout)
}

fn main() {
    let mut args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    if !args[1].starts_with("--varlink") {
        eprintln!("Usage: {} --varlink=<varlink address>", args[0]);
        exit(1);
    }

    exit(match run_app(args.swap_remove(1)[10..].into(), 0) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

#[test]
fn test_client() {
    use varlink::OrgVarlinkServiceInterface;
    use std::{thread, time};
    use org_example_complex::Error_::*;

    fn run_client_app(address: String) -> io::Result<()> {
        let conn = varlink::Connection::new(&address)?;

        let mut call = varlink::OrgVarlinkServiceClient::new(conn.clone());
        let info = call.get_info()?.recv()?;
        assert_eq!(&info.vendor, "org.varlink");
        assert_eq!(&info.product, "test service");
        assert_eq!(&info.version, "0.1");
        assert_eq!(&info.url, "http://varlink.org");
        assert_eq!(
            info.interfaces.get(1).unwrap().as_ref(),
            "org.example.complex"
        );

        let description = call.get_interface_description("org.example.complex".into())?
            .recv()?;

        assert!(description.description.is_some());

        let mut call = org_example_complex::VarlinkClient::new(conn);
        let r = call.bar()?.recv();
        match r {
            Err(VarlinkError_(varlink::Error::MethodNotImplemented(_))) => {}
            res => panic!("Unknown result {:?}", res),
        }
        /*
        let r = call.foo(None, None, None)?.recv();
        match r {
            Err(VarlinkError_(varlink::Error::MethodNotImplemented(_))) => {}
            res => panic!("Unknown result {:?}", res),
        }
        */
        Ok(())
    }

    let child = thread::spawn(move || {
        if let Err(e) = run_app("unix:/tmp/org.example.complex_client".into(), 4) {
            panic!("error: {}", e);
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let res = run_client_app("unix:/tmp/org.example.complex_client".into());
    if res.is_err() {
        eprintln!("{:?}", res);
    }
    assert!(res.is_ok());

    assert!(child.join().is_ok());
}
