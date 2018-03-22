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
        unimplemented!()
    }

    fn foo(
        &self,
        call: &mut _CallFoo,
        enum_: Option<FooArgs_enum>,
        foo: Option<TypeFoo>,
        interface: Option<Interface>,
    ) -> Result<(), Error> {
        unimplemented!()
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
    let args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    exit(match run_app(args[1].clone(), 0) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

#[test]
fn test_unix() {
    assert!(run_app("unix:/tmp/org.example.complex_unix".into(), 1).is_ok());
}
