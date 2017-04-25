extern crate std;
extern crate libc;

pub use self::libc::SIGINT;

pub struct SignalFd {
    fd: std::os::unix::io::RawFd,
    mask: libc::sigset_t,
}

impl SignalFd {
    pub fn new(signals: &[libc::c_int]) -> std::io::Result<Self> {
        unsafe {
            let mut mask = std::mem::uninitialized();
            libc::sigemptyset(&mut mask);
            for &sig in signals {
                libc::sigaddset(&mut mask, sig);
            }
            let rc = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut());
            if rc == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                let fd = libc::signalfd(-1, &mask, libc::SFD_CLOEXEC);
                if fd == -1 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(Self { fd: fd, mask: mask })
                }
            }
        }
    }

    pub fn read(&self) -> std::io::Result<libc::signalfd_siginfo> {
        unsafe {
            let size = std::mem::size_of::<libc::signalfd_siginfo>();
            let mut buf: libc::signalfd_siginfo = std::mem::uninitialized();
            let rc = libc::read(self.fd,
                                &mut buf as *mut libc::signalfd_siginfo as *mut libc::c_void,
                                size);
            if rc == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(buf)
            }
        }
    }
}

impl std::fmt::Debug for SignalFd {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SignalFd {{ fd: {} }}", self.fd)
    }
}

impl Drop for SignalFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
            libc::pthread_sigmask(libc::SIG_UNBLOCK, &self.mask, std::ptr::null_mut());
        }
    }
}

impl std::os::unix::io::AsRawFd for SignalFd {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.fd
    }
}
