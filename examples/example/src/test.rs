use failure::Fail;
use std::io;
use std::{thread, time};
use varlink::Connection;
use Result;

fn run_self_test(address: &'static str) -> Result<()> {
    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(address, 4) {
            if e.kind() != ::varlink::ErrorKind::Timeout {
                panic!("error: {:#?}", e.cause());
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = ::run_client(Connection::with_address(&address)?);
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
fn test_unix() {
    assert!(run_self_test("unix:/tmp/io.systemd.network").is_ok());
}
