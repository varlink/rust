#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate varlink;

use org_example_more::*;
use std::{thread, time};
use std::env;
use std::io;
use std::io::{Error, ErrorKind};
use std::process::exit;
use varlink::VarlinkService;

// Dynamically build the varlink rust code.
mod org_example_more;

struct MyOrgExampleMore;

impl VarlinkInterface for MyOrgExampleMore {
    fn ping(&self, call: &mut _CallPing, ping: String) -> io::Result<()> {
        return call.reply(ping);
    }

    fn stop_serving(&self, call: &mut _CallStopServing) -> io::Result<()> {
        call.reply()?;
        Err(Error::new(ErrorKind::ConnectionRefused, "Disconnect"))
    }

    fn test_more(&self, call: &mut _CallTestMore, n: i64) -> io::Result<()> {
        if !call.wants_more() {
            return call.reply_test_more_error("called without more".into());
        }

        if n == 0 {
            return call.reply_test_more_error("n == 0".into());
        }

        call.set_continues(true);

        call.reply(State {
            start: Some(true),
            end: None,
            progress: None,
        })?;

        for i in 0..n {
            thread::sleep(time::Duration::from_secs(1));
            call.reply(State {
                progress: Some(i * 100 / n),
                start: None,
                end: None,
            })?;
        }

        call.reply(State {
            progress: Some(100),
            start: None,
            end: None,
        })?;

        call.set_continues(false);

        call.reply(State {
            end: Some(true),
            progress: None,
            start: None,
        })
    }
}

fn run_app(address: String, timeout: u64) -> io::Result<()> {
    let myexamplemore = MyOrgExampleMore;
    let myinterface = new(Box::new(myexamplemore));
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![Box::new(myinterface)],
    );
    varlink::listen(service, &address, 10, timeout)
}

fn main() {
    let mut args: Vec<_> = env::args().collect();
    match args.len() {
        2 => {}
        _ => {
            eprintln!("Usage: {} <varlink address>", args[0]);
            exit(1);
        }
    };

    if !args[1].starts_with("--varlink") {
        eprintln!("Usage: {} --varlink=<varlink address>", args[0]);
        exit(1);
    }

    exit(match run_app(args.swap_remove(1)[10..].into(), 0) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    });
}

#[cfg(test)]
mod test {
    use org_example_more::*;
    use org_example_more;
    use std::{thread, time};
    use std::io;
    use varlink;
    use varlink::OrgVarlinkServiceInterface;

    fn run_client_app(address: String) -> io::Result<()> {
        let con1 = varlink::Connection::new(&address)?;
        let new_addr;
        {
            let conn = con1.read().unwrap();
            new_addr = conn.address();
        }

        let mut call = varlink::OrgVarlinkServiceClient::new(con1.clone());
        let info = call.get_info()?.recv()?;
        assert_eq!(&info.vendor, "org.varlink");
        assert_eq!(&info.product, "test service");
        assert_eq!(&info.version, "0.1");
        assert_eq!(&info.url, "http://varlink.org");
        assert_eq!(
            info.interfaces.get(0).unwrap().as_ref(),
            "org.varlink.service"
        );

        let description = call.get_interface_description("org.example.more".into())?
            .recv()?;

        assert_eq!(
            &description.description.unwrap(),
            r#"# Example service
interface org.example.more

# Enum, returning either start, progress or end
# progress: [0-100]
type State (
  start: ?bool,
  progress: ?int,
  end: ?bool
)

# Returns the same string
method Ping(ping: string) -> (pong: string)

# Dummy progress method
# n: number of progress steps
method TestMore(n: int) -> (state: State)

# Stop serving
method StopServing() -> ()

# Something failed in TestMore
error TestMoreError (reason: string)
"#
        );

        let mut call = org_example_more::VarlinkClient::new(con1);

        let con2 = varlink::Connection::new(&new_addr)?;
        let mut pingcall = org_example_more::VarlinkClient::new(con2);

        for reply in call.more().test_more(4)? {
            let reply = reply?;
            match reply.state {
                State {
                    start: Some(true),
                    end: None,
                    progress: None,
                    ..
                } => {
                    eprintln!("--- Start ---");
                }
                State {
                    start: None,
                    end: Some(true),
                    progress: None,
                    ..
                } => {
                    eprintln!("--- End ---");
                }
                State {
                    start: None,
                    end: None,
                    progress: Some(progress),
                    ..
                } => {
                    eprintln!("Progress: {}", progress);
                    if progress > 50 {
                        let reply = pingcall.ping("Test".into())?.recv()?;
                        println!("Pong: '{}'", reply.pong);
                    }
                }
                _ => panic!("Got unknown state: {:?}", reply.state),
            }
        }

        {
            let reply = call.test_more(0)?.recv();
            assert!(reply.is_err());
            match reply {
                Err(Error_::TestMoreError(Some(TestMoreErrorArgs_ {
                    reason: ref e,
                }))) if e == "called without more" => {}
                r => panic!("Unknown reply {:#?}", r),
            }
        }

        for reply in call.more().test_more(0)? {
            assert!(reply.is_err());
            match reply {
                Err(Error_::TestMoreError(Some(TestMoreErrorArgs_ {
                    reason: ref e,
                }))) if e == "n == 0" => {}
                r => panic!("Unknown reply {:#?}", r),
            }
        }

        let _r = call.stop_serving()?.recv()?;
        Ok(())
    }

    fn run_client(address: String) -> io::Result<()> {
        let client_address = address.clone();

        let child = thread::spawn(move || {
            if let Err(e) = ::run_app(address, 4) {
                panic!("error: {}", e);
            }
        });

        // give server time to start
        thread::sleep(time::Duration::from_secs(1));

        assert!(run_client_app(client_address).is_ok());
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
        assert!(run_client("unix:/tmp/org.example.more_client".into()).is_ok());
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn test_unix_abstract() {
        assert!(run_client("unix:@org.example.more_unix".into()).is_ok());
    }

    #[test]
    fn test_tcp() {
        assert!(run_client("tcp:0.0.0.0:12345".into()).is_ok());
    }

    #[test]
    fn test_exec() {
        let address: String;

        if ::std::path::Path::new("../../target/debug/varlink-server-more").exists() {
            address = "exec:../../target/debug/varlink-server-more".into();
        } else if ::std::path::Path::new("./target/debug/varlink-server-more").exists() {
            address = "exec:./target/debug/varlink-server-more".into();
        } else {
            eprintln!("Skipping test, no varlink-server-more");
            return;
        }

        assert!(run_client_app(address.clone()).is_ok());
    }

    #[test]
    #[should_panic]
    fn test_wrong_address_1() {
        assert!(run_client("tcpd:0.0.0.0:12345".into()).is_ok());
    }
}
