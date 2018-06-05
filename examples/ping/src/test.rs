use std::io;
use std::{thread, time};

fn run_self_test(address: String) -> io::Result<()> {
    let server_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(&server_address, 4) {
            match e.kind() {
                ::varlink::ErrorKind::Timeout => {}
                _ => panic!("error: {}", e),
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = ::run_client(&address);
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
    assert!(run_self_test("unix:/tmp/org.example.ping".into()).is_ok());
}
