use std::io::{self, Read, Result};
use std::fs::File;
use std::os::windows::io::{AsRawHandle, FromRawHandle};

pub struct WatchClose {
    inner: File,
}

impl AsRawHandle for WatchClose {
    fn as_raw_handle(&self) -> RawFd {
        self.inner.as_raw_handle()
    }
}

impl WatchClose {
    pub fn new_read<P: AsRawHandle, Q: AsRawHandle>(fd: &P, towatch: &Q) -> Result<WatchClose> {
        let fd = fd.as_raw_handle();
        let mut wc = WatchClose {
            inner: File::from_raw_handle(fd),
        };
        Ok(wc)
    }
}

impl Read for WatchClose {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        return self.inner.read(buf);
    }
}
