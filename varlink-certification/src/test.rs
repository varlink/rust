use escargot::CargoBuild;
use failure::ResultExt;
use std::io;
use std::{thread, time};
use varlink::Connection;
use Result;

fn run_self_test(address: String) -> Result<()> {
    let client_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = ::run_server(&address, 4) {
            match e.kind() {
                ::varlink::ErrorKind::Timeout => {}
                _ => panic!("error: {}", e),
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    let ret = ::run_client(Connection::with_address(&client_address)?);
    if let Err(e) = ret {
        panic!("error: {:?}", e);
    }
    if let Err(e) = child.join() {
        Err(io::Error::new(io::ErrorKind::ConnectionRefused, format!("{:#?}", e)).into())
    } else {
        Ok(())
    }
}

#[test]
fn test_unix() -> ::Result<()> {
    run_self_test("unix:/tmp/org.varlink.certification".into())
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_unix_abstract() -> Result<()> {
    run_self_test("unix:@org.varlink.certification".into())
}

#[test]
fn test_tcp() -> Result<()> {
    run_self_test("tcp:0.0.0.0:23456".into())
}

fn get_exec() -> Result<String> {
    let runner = CargoBuild::new()
        .current_release()
        .run()
        .context(::ErrorKind::Io_Error(::std::io::ErrorKind::NotFound))?;
    Ok(runner.path().to_owned().to_string_lossy().to_string())
}

#[test]
fn test_exec() -> Result<()> {
    ::run_client(Connection::with_activate(&format!(
        "{} --varlink=$VARLINK_ADDRESS",
        get_exec()?
    ))?)
}

#[test]
fn test_wrong_address_1() {
    assert!(::run_server("tcpd:0.0.0.0:12345".into(), 1).is_err());
}
