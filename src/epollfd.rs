extern crate std;
extern crate libc;

pub struct EpollFd {
    fd: std::os::unix::io::RawFd,
}

impl EpollFd {
    pub fn new() -> std::io::Result<Self> {
        let fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if fd == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(EpollFd { fd: fd })
        }
    }

    pub fn add_in<T>(&self, fd: &T) -> std::io::Result<()>
        where T: std::os::unix::io::AsRawFd
    {
        let mut event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: fd.as_raw_fd() as u64,
        };
        let rc =
            unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_ADD, fd.as_raw_fd(), &mut event) };
        if rc == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn wait(&self, maxevents: u32) -> std::io::Result<Vec<std::os::unix::io::RawFd>> {
        let mut events = Vec::with_capacity(maxevents as usize);
        let rc = unsafe { libc::epoll_wait(self.fd, events.as_mut_ptr(), maxevents as i32, -1) };
        if rc == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            let size = rc as usize;
            unsafe { events.set_len(size) };
            let mut fds = Vec::with_capacity(size);
            for i in 0..size {
                fds.push(events[i].u64 as std::os::unix::io::RawFd);
            }
            Ok(fds)
        }
    }
}

impl Drop for EpollFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

impl std::os::unix::io::AsRawFd for EpollFd {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.fd
    }
}
