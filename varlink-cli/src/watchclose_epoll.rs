use epoll;
use libc;
use libc::{c_int, c_void, ssize_t};
use std::io::{self, Read, Result};
use std::os::unix::io::{AsRawFd, RawFd};

pub trait IsMinusOne {
    fn is_minus_one(&self) -> bool;
}

macro_rules! impl_is_minus_one {
    ($($t:ident)*) => ($(impl IsMinusOne for $t {
        fn is_minus_one(&self) -> bool {
            *self == -1
        }
    })*)
}

impl_is_minus_one! { i8 i16 i32 i64 isize }

pub fn cvt<T: IsMinusOne>(t: T) -> crate::io::Result<T> {
    if t.is_minus_one() {
        Err(crate::io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

fn max_len() -> usize {
    // The maximum read limit on most posix-like systems is `SSIZE_MAX`,
    // with the man page quoting that if the count of bytes to read is
    // greater than `SSIZE_MAX` the result is "unspecified".
    //
    // On macOS, however, apparently the 64-bit libc is either buggy or
    // intentionally showing odd behavior by rejecting any read with a size
    // larger than or equal to INT_MAX. To handle both of these the read
    // size is capped on both platforms.
    if cfg!(target_os = "macos") {
        <c_int>::max_value() as usize - 1
    } else {
        <ssize_t>::max_value() as usize
    }
}

pub struct WatchClose {
    fd: RawFd,
    towatch: RawFd,
    efd: RawFd,
    prev_block: bool,
}

impl Drop for WatchClose {
    fn drop(&mut self) {
        let _ = self.set_nonblocking(self.prev_block);
        let _ = epoll::close(self.efd);
    }
}

impl AsRawFd for WatchClose {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl WatchClose {
    pub fn new_read<P: AsRawFd, Q: AsRawFd>(fd: &P, towatch: &Q) -> Result<WatchClose> {
        let fd = fd.as_raw_fd();
        let mut wc = WatchClose {
            fd: fd,
            towatch: towatch.as_raw_fd(),
            efd: epoll::create(true)?,
            prev_block: false,
        };

        epoll::ctl(
            wc.efd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            wc.towatch,
            epoll::Event::new(epoll::Events::EPOLLRDHUP, 1),
        )?;

        epoll::ctl(
            wc.efd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            wc.fd,
            epoll::Event::new(epoll::Events::EPOLLIN | epoll::Events::EPOLLRDHUP, 0),
        )?;

        wc.prev_block = wc.set_nonblocking(true)?;
        Ok(wc)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<bool> {
        unsafe {
            let previous = cvt(libc::fcntl(self.fd, libc::F_GETFL))?;
            let new = if nonblocking {
                previous | libc::O_NONBLOCK
            } else {
                previous & !libc::O_NONBLOCK
            };
            if new != previous {
                cvt(libc::fcntl(self.fd, libc::F_SETFL, new))?;
            }
            Ok((previous & libc::O_NONBLOCK) == 0)
        }
    }
}

impl Read for WatchClose {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut v = vec![
            epoll::Event {
                events: epoll::Events::EPOLLIN.bits(),
                data: 0,
            },
            epoll::Event {
                events: epoll::Events::EPOLLRDHUP.bits(),
                data: 1,
            },
        ];

        'outer: loop {
            let r = epoll::wait(self.efd, -1, &mut v[..])?;

            let err_mask =
                epoll::Events::EPOLLRDHUP | epoll::Events::EPOLLHUP | epoll::Events::EPOLLERR;

            for ev in v.iter().take(r) {
                if err_mask.bits() & ev.events != 0 {
                    return Err(io::Error::from(io::ErrorKind::BrokenPipe));
                }
            }

            for ev in v.iter().take(r) {
                if epoll::Events::EPOLLIN.bits() & ev.events != 0 {
                    break 'outer;
                }
            }
        }

        let ret = cvt(unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut c_void,
                ::std::cmp::min(buf.len(), max_len()),
            )
        })?;

        Ok(ret as usize)
    }
}
