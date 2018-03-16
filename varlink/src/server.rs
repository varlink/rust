use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::thread;
use std::sync::Arc;
use std::net::{TcpListener, TcpStream};
// FIXME: abstract unix domains sockets still not in std
// FIXME: https://github.com/rust-lang/rust/issues/14194
use unix_socket::{UnixListener, UnixStream};
use std::fs;
use std::env;
use std::os::unix::io::FromRawFd;

enum VarlinkListener {
    TCP(TcpListener),
    UNIX(UnixListener),
}

enum VarlinkStream {
    TCP(TcpStream),
    UNIX(UnixStream),
}

impl<'a> VarlinkStream {
    pub fn split(&mut self) -> io::Result<(Box<Read + Send + Sync>, Box<Write + Send + Sync>)> {
        match *self {
            VarlinkStream::TCP(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
            VarlinkStream::UNIX(ref mut s) => {
                Ok((Box::new(s.try_clone()?), Box::new(s.try_clone()?)))
            }
        }
    }
}

fn activation_listener() -> io::Result<Option<UnixListener>> {
    /*
	FIXME: only working on nightly https://github.com/rust-lang/rust/pull/45059

    let spid = env::var("LISTEN_PID");
    if let Ok(pid) = spid {
        let mypid = std::process::id();
        match pid.parse::<u32>() {
            mypid => {}
            _ => return Ok(None),
        }
    } else {
        return Ok(None);
    }
*/
    let snfds = env::var("LISTEN_FDS");
    if let Ok(nfds) = snfds {
        match nfds.parse::<u32>() {
            Ok(1) => {}
            _ => return Ok(None),
        }
    } else {
        return Ok(None);
    }
    unsafe {
        //syscall.CloseOnExec(3)
        let listener = UnixListener::from_raw_fd(3);
        env::remove_var("LISTEN_PID");
        env::remove_var("LISTEN_FDS");

        Ok(Some(listener))
    }
}

//FIXME: add Drop with shutdown() and unix file removal
impl VarlinkListener {
    pub fn new(address: &str) -> io::Result<Self> {
        if let Some(l) = activation_listener()? {
            return Ok(VarlinkListener::UNIX(l));
        }

        if address.starts_with("tcp:") {
            Ok(VarlinkListener::TCP(TcpListener::bind(&address[4..])?))
        } else if address.starts_with("unix:") {
            let mut addr = String::from(address[5..].split(";").next().unwrap());
            if addr.starts_with("@") {
                addr = addr.replacen("@", "\0", 1);
            } else {
                // ignore error on non-existant file
                let _ = fs::remove_file(addr.clone());
            }
            Ok(VarlinkListener::UNIX(UnixListener::bind(addr)?))
        } else {
            Err(Error::new(ErrorKind::Other, "unknown varlink address"))
        }
    }

    pub fn accept(&self) -> io::Result<VarlinkStream> {
        match self {
            &VarlinkListener::TCP(ref l) => {
                let (mut s, _addr) = l.accept()?;
                Ok(VarlinkStream::TCP(s))
            }
            &VarlinkListener::UNIX(ref l) => {
                let (mut s, _addr) = l.accept()?;
                Ok(VarlinkStream::UNIX(s))
            }
        }
    }
}

pub fn listen(addr: &str, service: Arc<::VarlinkService>) -> io::Result<()> {
    println!("Listening on {}", addr);
    let listener = VarlinkListener::new(addr)?;

    loop {
        let mut stream;
        match listener.accept() {
            Err(e) => match Error::last_os_error().raw_os_error() {
                Some(11) => continue,
                _ => return Err(e),
            },
            Ok(s) => stream = s,
        }
        let service = service.clone();
        let _join = thread::spawn(move || -> io::Result<()> {
            let (mut r, mut w) = stream.split()?;
            if let Err(e) = service.handle(&mut r, &mut w) {
                println!("Handle Error: {}", e);
            }
            Ok(())
        });
    }
}
