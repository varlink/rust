use chainerror::*;
use std::{thread, time};
use varlink::Connection;

pub type Result<T> = std::result::Result<T, Box<std::error::Error>>;

fn run_self_test(address: &'static str) -> Result<()> {
    let child = thread::spawn(move || {
        if let Err(e) = crate::run_server(address, 4) {
            if *e.kind() != ::varlink::ErrorKind::Timeout {
                panic!("error: {:?}", e);
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = crate::run_client(Connection::with_address(&address)?);
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
    run_self_test("unix:io.systemd.network")
}
