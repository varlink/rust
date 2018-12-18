use std::error::Error as StdError;
use std::result::Result as StdResult;

type Result<T> = StdResult<T, Box<StdError>>;

use chainerror::*;
use std::io::BufRead;
use std::{thread, time};
use varlink::Connection;

fn run_self_test(address: String, multiplex: bool) -> Result<()> {
    let server_address = address.clone();

    let child = thread::spawn(move || {
        if let Err(e) = crate::run_server(&server_address, 4, multiplex) {
            match e.kind() {
                ::varlink::ErrorKind::Timeout => {}
                _ => panic!("error: {}", e),
            }
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    {
        let con = Connection::with_address(&address)
            .map_err(mstrerr!("Could not connect to {}", address))?;

        let mut conn = con.write().unwrap();
        let mut writer = conn.writer.take().unwrap();
        let mut reader = conn.reader.take().unwrap();

        {
            let br = concat!(
                r#"{"method" : "org.example.ping.Ping", "parameters": { "ping": "ping" }}"#,
                "\0"
            )
            .as_bytes();

            let a = &br[0..10];
            let b = &br[10..20];
            let c = &br[20..];

            for i in vec![a, b, c] {
                assert!(writer.write_all(i).is_ok());
                assert!(writer.flush().is_ok());
                thread::sleep(time::Duration::from_millis(500));
            }

            let mut inbuf = Vec::new();
            let reply = concat!(r#"{"parameters":{"pong":"ping"}}"#, "\0").as_bytes();

            assert!(reader.read_until(0, &mut inbuf).is_ok());
            eprintln!("Got reply: {}", String::from_utf8_lossy(&inbuf));
            assert_eq!(inbuf, reply);
        }

        {
            let mut inbuf = Vec::new();
            let reply = "{}\0".as_bytes();

            assert!(writer
                .write_all(
                    concat!(
                        r#"{"method" : "org.example.ping.Upgrade", "upgrade" : true}"#,
                        "\0"
                    )
                    .as_bytes(),
                )
                .is_ok());
            assert!(writer.flush().is_ok());

            assert!(reader.read_until(0, &mut inbuf).is_ok());
            eprintln!("Got reply: {}", String::from_utf8_lossy(&inbuf));
            assert_eq!(inbuf, reply);
        }
        {
            let br = concat!(
                r#"{"method" : "org.example.ping.Ping", "parameters": { "ping": "ping" }}"#,
                "\n"
            )
            .as_bytes();

            let a = &br[0..10];
            let b = &br[10..20];
            let c = &br[20..];

            for i in vec![a, b, c] {
                assert!(writer.write_all(i).is_ok());
                assert!(writer.flush().is_ok());
                thread::sleep(time::Duration::from_millis(500));
            }

            let mut inbuf = Vec::new();
            let reply = concat!(
                r#"server reply: {"method" : "org.example.ping.Ping", "#,
                r#""parameters": { "ping": "ping" }}"#,
                "\n"
            )
            .as_bytes();

            assert!(reader.read_until(0x0a, &mut inbuf).is_ok());
            eprintln!("Got reply: {}", String::from_utf8_lossy(&inbuf));
            assert_eq!(inbuf, reply);
        }
        {
            let br = concat!(
                r#"{"method" : "org.example.ping.Ping", "parameters": { "ping": "ping" }}"#,
                "\nEnd\n"
            )
            .as_bytes();

            let a = &br[0..10];
            let b = &br[10..20];
            let c = &br[20..];

            for i in vec![a, b, c] {
                assert!(writer.write_all(i).is_ok());
                assert!(writer.flush().is_ok());
                thread::sleep(time::Duration::from_millis(500));
            }

            let mut inbuf = Vec::new();
            let reply = concat!(
                r#"server reply: {"method" : "org.example.ping.Ping", "#,
                r#""parameters": { "ping": "ping" }}"#,
                "\n"
            )
            .as_bytes();

            assert!(reader.read_until(0x0a, &mut inbuf).is_ok());
            eprintln!("Got reply: {}", String::from_utf8_lossy(&inbuf));
            assert_eq!(inbuf, reply);
        }
    }

    {
        let con = Connection::with_address(&address)
            .map_err(mstrerr!("Could not connect to {}", address))?;

        let ret = crate::run_client(&con);
        if let Err(e) = ret {
            panic!("error: {:#?}", e);
        }
    }
    eprintln!("run_client finished");

    if let Err(_) = child.join() {
        Err(strerr!("Error joining thread").into())
    } else {
        Ok(())
    }
}

/*
#[test]
fn test_unix_multiplex() -> Result<()> {
    run_self_test("unix:/tmp/org.example.ping_multiplex".into(), true)
}
*/

#[test]
fn test_unix() -> Result<()> {
    run_self_test("unix:org.example.ping".into(), false)
}

#[test]
fn test_tcp() -> Result<()> {
    run_self_test("tcp:127.0.0.1:12345".into(), false)
}
