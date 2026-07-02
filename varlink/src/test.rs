use crate::*;
use serde_json::{from_slice, from_value};
use std::{thread, time};

#[test]
fn test_listen() -> Result<()> {
    fn run_app<S: ?Sized + AsRef<str>>(address: &S, timeout: u64) -> Result<()> {
        let service = VarlinkService::new(
            "org.varlink",
            "test service",
            "0.1",
            "http://varlink.org",
            vec![], // Your varlink interfaces go here
        );

        if let Err(e) = listen(
            service,
            &address,
            &ListenConfig {
                idle_timeout: timeout,
                ..Default::default()
            },
        ) {
            if *e.kind() != ErrorKind::Timeout {
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
                info.interfaces.first().unwrap().as_ref(),
                "org.varlink.service"
            );
        }
        let e = call.get_interface_description("org.varlink.unknown");
        assert!(e.is_err());

        match e.err().unwrap().kind() {
            ErrorKind::InvalidParameter(i) => assert_eq!(*i, "interface".to_string()),
            kind => {
                panic!("Unknown error {:?}", kind);
            }
        }

        let e = MethodCall::<GetInfoArgs, ServiceInfo, Error>::new(
            conn.clone(),
            "org.varlink.service.GetInfos",
            GetInfoArgs {},
        )
        .call();

        match e.err().unwrap().kind() {
            ErrorKind::MethodNotFound(i) => {
                assert_eq!(*i, "org.varlink.service.GetInfos".to_string())
            }
            kind => {
                panic!("Unknown error {:?}", kind);
            }
        }

        let e = MethodCall::<GetInfoArgs, ServiceInfo, Error>::new(
            conn.clone(),
            "org.varlink.unknowninterface.Foo",
            GetInfoArgs {},
        )
        .call();

        match e.err().unwrap().kind() {
            ErrorKind::InterfaceNotFound(i) => {
                assert_eq!(*i, "org.varlink.unknowninterface".to_string())
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

    let address = "unix:test_listen_timeout";

    let child = thread::spawn(move || {
        if let Err(e) = run_app(address, 3) {
            panic!("error: {}", e);
        }
    });

    // give server time to start
    thread::sleep(time::Duration::from_secs(1));

    run_client_app(address)?;

    assert!(child.join().is_ok());

    Ok(())
}

#[test]
fn test_handle() -> Result<()> {
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

    for mut i in [a, b, c] {
        buf.append(&mut i);

        let res = {
            let mut br = buf.as_slice();
            ConnectionHandler::handle(&service, &mut br, &mut w, None)?
        };
        match res {
            (_, Some(iface)) => {
                panic!("Unexpected handle return value {}", iface);
            }
            (v, None) => {
                if v.is_empty() {
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
        )
        .as_bytes()
    );

    let reply = from_slice::<Reply>(&w).unwrap();

    let si = from_value::<ServiceInfo>(reply.parameters.unwrap()).map_err(map_context!())?;

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
    Ok(())
}

// push_fd is only meaningful on a socket-backed connection: a reader/writer-pair
// connection (Connection::default here) has no socket for ancillary data.
#[cfg(unix)]
#[test]
fn test_push_fd_requires_socket() {
    let conn = Arc::new(RwLock::new(Connection::default()));
    let mut call = MethodCall::<serde_json::Value, serde_json::Value, Error>::new(
        conn,
        "org.example.Test.Method",
        serde_json::json!({}),
    );
    let err = call.push_fd(0).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}

// push_fd attaches descriptors to the outgoing message via SCM_RIGHTS. Send over
// one end of a socketpair and recvmsg() the other end to confirm the fd arrives,
// referring to the same underlying object (a pipe we can then read through).
#[cfg(unix)]
#[test]
fn test_push_fd_passes_descriptor() -> Result<()> {
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::net::UnixStream;

    let (client, server) = UnixStream::pair().unwrap();

    // Pipe whose write end we pass; reading its read end proves the fd traveled.
    let mut pipe = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0);
    let (pipe_r, pipe_w) =
        unsafe { (OwnedFd::from_raw_fd(pipe[0]), OwnedFd::from_raw_fd(pipe[1])) };

    let reader = Box::new(client.try_clone().unwrap());
    let writer = Box::new(client.try_clone().unwrap());
    // Connection implements Drop, so the fields must be listed explicitly (no
    // ..Default::default() functional update).
    let conn = Arc::new(RwLock::new(Connection {
        reader: Some(BufReader::new(reader)),
        writer: Some(writer),
        address: String::new(),
        stream: Some(Box::new(client)),
        child: None,
        tempdir: None,
    }));

    let mut call = MethodCall::<serde_json::Value, serde_json::Value, Error>::new(
        conn,
        "org.example.Test.Method",
        serde_json::json!({}),
    );
    assert_eq!(call.push_fd(pipe_w.as_raw_fd()).unwrap(), 0);
    call.oneway()?; // sends the request with the fd attached
    drop(pipe_w); // leave the received dup as the pipe's only writer

    // recvmsg() the request bytes plus the SCM_RIGHTS descriptor.
    let mut buf = [0u8; 256];
    let mut iov = libc::iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    };
    let mut cbuf =
        vec![0u8; unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) } as usize];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1 as _;
    msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cbuf.len() as _;

    let n = unsafe { libc::recvmsg(server.as_raw_fd(), &mut msg, 0) };
    assert!(n > 0, "recvmsg returned {}", n);
    // A varlink message is NUL-terminated JSON.
    assert!(buf[..n as usize].contains(&0));

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    assert!(!cmsg.is_null(), "no control message received");
    assert_eq!(unsafe { (*cmsg).cmsg_level }, libc::SOL_SOCKET);
    assert_eq!(unsafe { (*cmsg).cmsg_type }, libc::SCM_RIGHTS);
    let received_fd = unsafe { std::ptr::read_unaligned(libc::CMSG_DATA(cmsg) as *const i32) };
    let received = unsafe { OwnedFd::from_raw_fd(received_fd) };

    // Writing through the received fd must surface on our pipe's read end.
    let payload = b"hi";
    let w = unsafe {
        libc::write(
            received.as_raw_fd(),
            payload.as_ptr() as *const libc::c_void,
            payload.len(),
        )
    };
    assert_eq!(w, payload.len() as isize);
    drop(received);

    let mut got = [0u8; 2];
    std::fs::File::from(pipe_r).read_exact(&mut got).unwrap();
    assert_eq!(&got, payload);
    Ok(())
}
