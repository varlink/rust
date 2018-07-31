use serde_json::{from_slice, from_value};
use std::{thread, time};
use *;

#[test]
fn test_listen() {
    fn run_app<S: ?Sized + AsRef<str>>(address: &S, timeout: u64) -> Result<()> {
        let service = VarlinkService::new(
            "org.varlink",
            "test service",
            "0.1",
            "http://varlink.org",
            vec![], // Your varlink interfaces go here
        );

        if let Err(e) = listen(service, &address, 10, timeout) {
            if e.kind() != ErrorKind::Timeout {
                panic!("Error listen: {:#?}", e);
            }
        }
        Ok(())
    }

    fn run_client_app<S: ?Sized + AsRef<str>>(address: &S) -> Result<()> {
        let conn = Connection::new(address)?;
        let mut call = OrgVarlinkServiceClient::new(conn.clone());
        {
            let info = call.get_info()?;
            assert_eq!(&info.vendor, "org.varlink");
            assert_eq!(&info.product, "test service");
            assert_eq!(&info.version, "0.1");
            assert_eq!(&info.url, "http://varlink.org");
            assert_eq!(
                info.interfaces.get(0).unwrap().as_ref(),
                "org.varlink.service"
            );
        }
        let e = call.get_interface_description("org.varlink.unknown");
        assert!(e.is_err());

        match e.err().unwrap().kind() {
            ErrorKind::InvalidParameter(i) => assert_eq!(i, "interface".to_string()),
            kind => {
                panic!("Unknown error {:?}", kind);
            }
        }

        let e = MethodCall::<GetInfoArgs, ServiceInfo, Error>::new(
            conn.clone(),
            "org.varlink.service.GetInfos",
            GetInfoArgs {},
        ).call();

        match e.err().unwrap().kind() {
            ErrorKind::MethodNotFound(i) => {
                assert_eq!(i, "org.varlink.service.GetInfos".to_string())
            }
            kind => {
                panic!("Unknown error {:?}", kind);
            }
        }

        let e = MethodCall::<GetInfoArgs, ServiceInfo, Error>::new(
            conn.clone(),
            "org.varlink.unknowninterface.Foo",
            GetInfoArgs {},
        ).call();

        match e.err().unwrap().kind() {
            ErrorKind::InterfaceNotFound(i) => {
                assert_eq!(i, "org.varlink.unknowninterface".to_string())
            }
            kind => {
                panic!("Unknown error {:?}", kind);
            }
        }

        let description = call.get_interface_description("org.varlink.service")?;

        assert_eq!(
            &description.description.unwrap(),
            r#"# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

# Get a list of all the interfaces a service provides and information
# about the implementation.
method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: []string
)

# Get the description of an interface that is implemented by this service.
method GetInterfaceDescription(interface: string) -> (description: string)

# The requested interface was not found.
error InterfaceNotFound (interface: string)

# The requested method was not found
error MethodNotFound (method: string)

# The interface defines the requested method, but the service does not
# implement it.
error MethodNotImplemented (method: string)

# One of the passed parameters is invalid.
error InvalidParameter (parameter: string)
"#
        );

        Ok(())
    }

    let address = "unix:/tmp/test_listen_timeout";

    let child = thread::spawn(move || {
        if let Err(e) = run_app(address, 3) {
            panic!("error: {}", e);
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    assert!(run_client_app(address).is_ok());

    assert!(child.join().is_ok());
}

#[test]
fn test_handle() {
    let service = VarlinkService::new(
        "org.varlink",
        "test service",
        "0.1",
        "http://varlink.org",
        vec![],
    );

    let br = concat!(r#"{"method" : "org.varlink.service.GetInfo"}"#, "\0").as_bytes();

    let a = br[0..10].to_vec();
    let b = br[10..20].to_vec();
    let c = br[20..].to_vec();

    let mut w = vec![];

    let mut buf = Vec::<u8>::new();

    for mut i in vec![a, b, c] {
        buf.append(&mut i);

        match {
            let mut br = buf.as_slice();
            service.handle(&mut br, &mut w, None)
        } {
            Err(e) => {
                panic!("Unexpected handle error {:#?}", e);
            }
            Ok((_, Some(iface))) => {
                panic!("Unexpected handle return value {}", iface);
            }
            Ok((v, None)) => {
                if v.len() == 0 {
                    break;
                }
                //eprintln!("unhandled: {}", String::from_utf8_lossy(&v));
                buf.clone_from(&v);
            }
        }
    }

    w.pop();

    assert_eq!(
        w,
        concat!(
            r#"{"parameters":{"interfaces":["org.varlink.service"],"product":"test service","#,
            r#""url":"http://varlink.org","vendor":"org.varlink","version":"0.1"}}"#
        ).as_bytes()
    );

    let reply = from_slice::<Reply>(&w).unwrap();

    match from_value::<ServiceInfo>(reply.parameters.unwrap()) {
        Ok(si) => {
            assert_eq!(
                si,
                ServiceInfo {
                    vendor: "org.varlink".into(),
                    product: "test service".into(),
                    version: "0.1".into(),
                    url: "http://varlink.org".into(),
                    interfaces: vec!["org.varlink.service".into()],
                }
            );
        }
        Err(e) => panic!(e.to_string()),
    };
}
