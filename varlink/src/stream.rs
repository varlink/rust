use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

#[cfg(windows)]
use uds_windows::UnixStream;

use crate::error::*;

#[cfg(unix)]
pub trait Stream: Read + Write + Send + Sync + AsRawFd {
    fn split(&mut self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)>;
    fn shutdown(&mut self) -> Result<()>;
    fn try_clone(&mut self) -> ::std::io::Result<Box<dyn Stream>>;
    fn set_nonblocking(&mut self, b: bool) -> Result<()>;
}

#[cfg(windows)]
pub trait Stream: Read + Write + Send + Sync + AsRawSocket {
    fn split(&mut self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)>;
    fn shutdown(&mut self) -> Result<()>;
    fn try_clone(&mut self) -> ::std::io::Result<Box<dyn Stream>>;
    fn set_nonblocking(&mut self, b: bool) -> Result<()>;
}

impl Stream for TcpStream {
    #[inline]
    fn split(&mut self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        Ok((
            Box::new(TcpStream::try_clone(self).map_err(map_context!())?),
            Box::new(TcpStream::try_clone(self).map_err(map_context!())?),
        ))
    }

    #[inline]
    fn shutdown(&mut self) -> Result<()> {
        TcpStream::shutdown(self, Shutdown::Both).map_err(map_context!())?;
        Ok(())
    }

    #[inline]
    fn try_clone(&mut self) -> ::std::io::Result<Box<dyn Stream>> {
        Ok(Box::new(TcpStream::try_clone(self)?))
    }

    #[inline]
    fn set_nonblocking(&mut self, b: bool) -> Result<()> {
        TcpStream::set_nonblocking(self, b).map_err(map_context!())?;
        Ok(())
    }
}

impl Stream for UnixStream {
    #[inline]
    fn split(&mut self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        Ok((
            Box::new(UnixStream::try_clone(self).map_err(map_context!())?),
            Box::new(UnixStream::try_clone(self).map_err(map_context!())?),
        ))
    }

    #[inline]
    fn shutdown(&mut self) -> Result<()> {
        UnixStream::shutdown(self, Shutdown::Both).map_err(map_context!())?;
        Ok(())
    }

    #[inline]
    fn try_clone(&mut self) -> ::std::io::Result<Box<dyn Stream>> {
        Ok(Box::new(UnixStream::try_clone(self)?))
    }

    #[inline]
    fn set_nonblocking(&mut self, b: bool) -> Result<()> {
        UnixStream::set_nonblocking(self, b).map_err(map_context!())?;
        Ok(())
    }
}
