use failure::Fail;
use crate::io_systemd_network::Result;
use std::io;
use std::{thread, time};
use varlink::Connection;

fn run_self_test(address: &'static str) -> Result<()> {
    let child = thread::spawn(move || {
        if let Err(e) = crate::run_server(address, 4) {
            if e.kind() != ::varlink::ErrorKind::Timeout {
                panic!("error: {:#?}", e.cause());
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = crate::run_client(Connection::with_address(&address)?);
    if let Err(e) = ret {
        panic!("error: {}", e);
    }
    if let Err(e) = child.join() {
        Err(io::Error::new(io::ErrorKind::ConnectionRefused, format!("{:#?}", e)).into())
    } else {
        Ok(())
    }
}

#[test]
fn test_unix() -> Result<()> {
    run_self_test("unix:io.systemd.network")
}
