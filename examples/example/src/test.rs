use failure::{self, Error, Fail};
use std::{thread, time};

fn run_self_test(address: &'static str) -> Result<(), Error> {
    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(address, 4) {
            if e.kind() != ::varlink::ErrorKind::Timeout {
                panic!("error: {:#?}", e.cause());
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = ::run_client(address);
    if let Err(e) = ret {
        eprintln!("error: {:#?}", e.cause());
        return Err(e.into());
    }

    if let Err(e) = child.join() {
        Err(failure::err_msg(format!("{:#?}", e)))
    } else {
        Ok(())
    }
}

#[test]
fn test_unix() {
    assert!(run_self_test("unix:/tmp/io.systemd.network").is_ok());
}
