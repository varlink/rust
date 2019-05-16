use std::io::{self, Read, Result};
use std::os::unix::io::{AsRawFd, RawFd};

use libc::{self, c_int, c_void, ssize_t};

#[repr(i32)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
enum ControlOptions {
    EPOLL_CTL_ADD = libc::EPOLL_CTL_ADD,
    EPOLL_CTL_MOD = libc::EPOLL_CTL_MOD,
    EPOLL_CTL_DEL = libc::EPOLL_CTL_DEL,
}

bitflags! {
    struct Events: u32 {
        const EPOLLET      = libc::EPOLLET as u32;
        const EPOLLIN      = libc::EPOLLIN as u32;
        const EPOLLERR     = libc::EPOLLERR as u32;
        const EPOLLHUP     = libc::EPOLLHUP as u32;
        const EPOLLOUT     = libc::EPOLLOUT as u32;
        const EPOLLPRI     = libc::EPOLLPRI as u32;
        const EPOLLRDHUP   = libc::EPOLLRDHUP as u32;
        const EPOLLWAKEUP  = libc::EPOLLWAKEUP as u32;
        const EPOLLONESHOT = libc::EPOLLONESHOT as u32;
    }
}

#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
#[derive(Clone, Copy)]
struct Event {
    pub events: u32,
    pub data: u64,
}

impl Event {
    pub fn new(events: Events, data: u64) -> Event {
        Event {
            events: events.bits(),
            data: data,
        }
    }
}

fn epoll_create(cloexec: bool) -> io::Result<RawFd> {
    let flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
    unsafe { cvt(libc::epoll_create1(flags)) }
}

fn epoll_ctl(epfd: RawFd, op: ControlOptions, fd: RawFd, mut event: Event) -> io::Result<()> {
    let e = &mut event as *mut _ as *mut libc::epoll_event;
    unsafe { cvt(libc::epoll_ctl(epfd, op as i32, fd, e))? };
    Ok(())
}

fn epoll_wait(epfd: RawFd, timeout: i32, buf: &mut [Event]) -> io::Result<usize> {
    let timeout = if timeout < -1 { -1 } else { timeout };
    let num_events = unsafe {
        cvt(libc::epoll_wait(
            epfd,
            buf.as_mut_ptr() as *mut libc::epoll_event,
            buf.len() as i32,
            timeout,
        ))? as usize
    };
    Ok(num_events)
}

fn epoll_close(epfd: RawFd) -> io::Result<()> {
    cvt(unsafe { libc::close(epfd) })?;
    Ok(())
}

trait IsMinusOne {
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

fn cvt<T: IsMinusOne>(t: T) -> crate::io::Result<T> {
    if t.is_minus_one() {
        Err(crate::io::Error::last_os_error())
    } else {
        Ok(t)
    }
}

fn max_len() -> usize {
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
        let _ = epoll_close(self.efd);
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
            efd: epoll_create(true)?,
            prev_block: false,
        };

        epoll_ctl(
            wc.efd,
            ControlOptions::EPOLL_CTL_ADD,
            wc.towatch,
            Event::new(Events::EPOLLRDHUP, 1),
        )?;

        epoll_ctl(
            wc.efd,
            ControlOptions::EPOLL_CTL_ADD,
            wc.fd,
            Event::new(Events::EPOLLIN | Events::EPOLLRDHUP, 0),
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
            Event {
                events: Events::EPOLLIN.bits(),
                data: 0,
            },
            Event {
                events: Events::EPOLLRDHUP.bits(),
                data: 1,
            },
        ];

        'outer: loop {
            let r = epoll_wait(self.efd, -1, &mut v[..])?;

            let err_mask = Events::EPOLLRDHUP | Events::EPOLLHUP | Events::EPOLLERR;

            for ev in v.iter().take(r) {
                if err_mask.bits() & ev.events != 0 {
                    return Err(io::Error::from(io::ErrorKind::BrokenPipe));
                }
            }

            for ev in v.iter().take(r) {
                if Events::EPOLLIN.bits() & ev.events != 0 {
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
