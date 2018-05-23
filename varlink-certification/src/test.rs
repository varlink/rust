use std::io;
use std::{thread, time};

fn run_self_test(address: String) -> io::Result<()> {
    let client_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(address, 4) {
            match e.kind() {
                ::org_varlink_certification::ErrorKind::Varlink(kind) => {
                    if kind == ::varlink::ErrorKind::Timeout {}
                }
                _ => panic!("error: {}", e),
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = ::run_client(client_address);
    if let Err(e) = ret {
        panic!("error: {:?}", e);
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
    assert!(run_self_test("unix:/tmp/org.varlink.certification".into()).is_ok());
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_unix_abstract() {
    assert!(run_self_test("unix:@org.varlink.certification".into()).is_ok());
}

#[test]
fn test_tcp() {
    assert!(run_self_test("tcp:0.0.0.0:23456".into()).is_ok());
}

#[test]
fn test_exec() {
    let address: String;

    if ::std::path::Path::new("../../target/debug/varlink-certification").exists() {
        address = "exec:../../target/debug/varlink-certification".into();
    } else if ::std::path::Path::new("./target/debug/varlink-certification").exists() {
        address = "exec:./target/debug/varlink-certification".into();
    } else {
        eprintln!("test test::test_exec ... skipping, no varlink-certification binary found");
        return;
    }

    assert!(::run_client(address).is_ok());
}

#[test]
fn test_wrong_address_1() {
    assert!(::run_server("tcpd:0.0.0.0:12345".into(), 1).is_err());
}
