use chainerror::*;
use std::{thread, time};
use varlink::Connection;

pub type Result<T> = std::result::Result<T, Box<std::error::Error>>;

fn run_self_test(address: String) -> Result<()> {
    let client_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = crate::run_server(&address, 4) {
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
        panic!("error: {:?}", e);
    }
    if let Err(_) = child.join() {
        Err(strerr!("Error joining thread").into())
    } else {
        Ok(())
    }
}

#[test]
fn test_unix() -> crate::Result<()> {
    run_self_test("unix:org.varlink.certification".into())
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_unix_abstract() -> Result<()> {
    run_self_test("unix:@org.varlink.certification".into())
}

#[test]
fn test_tcp() -> Result<()> {
    run_self_test("tcp:127.0.0.1:23456".into())
}

#[cfg(unix)]
#[test]
fn test_exec() -> Result<()> {
    use escargot::CargoBuild;
    fn get_exec() -> Result<String> {
        let runner = CargoBuild::new()
            .current_release()
            .run()
            .map_err(mstrerr!("Error running CargoBuild"))?;
        Ok(runner.path().to_owned().to_string_lossy().to_string())
    }

    crate::run_client(Connection::with_activate(&format!(
        "{} --varlink=$VARLINK_ADDRESS",
        get_exec()?
    ))?)
}

#[test]
fn test_wrong_address_1() {
    assert!(crate::run_server("tcpd:0.0.0.0:12345".into(), 1).is_err());
}
