use crate::Result;
use chainerror::*;
use std::{thread, time};
use varlink::Connection;

fn run_self_test(address: String) -> Result<()> {
    let client_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = crate::run_server(&address, 4, 100) {
            match e.kind() {
                ::varlink::ErrorKind::Timeout => {}
                _ => panic!("error: {}", e),
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = crate::run_client(Connection::with_address(&client_address)?);
    if let Err(e) = ret {
        panic!("error: {}", e);
    }
    if let Err(_) = child.join() {
        Err(strerr!("Error joining thread").into())
    } else {
        Ok(())
    }
}

#[test]
fn test_unix() -> Result<()> {
    run_self_test("unix:org.example.more".into())
}

#[test]
fn test_tcp() -> Result<()> {
    run_self_test("tcp:127.0.0.1:12345".into())
}
