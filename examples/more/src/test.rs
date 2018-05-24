use std::io;
use std::{thread, time};

fn run_self_test(address: String) -> io::Result<()> {
    let client_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(address, 4, 100) {
            match e.kind() {
                ::varlink::ErrorKind::Timeout => {}
                _ => panic!("error: {}", e),
            }
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
    assert!(run_self_test("unix:/tmp/org.example.more".into()).is_ok());
}
