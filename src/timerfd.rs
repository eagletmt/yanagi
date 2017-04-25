extern crate chrono;
extern crate libc;
extern crate std;

#[repr(C)]
struct itimerspec {
    it_interval: libc::timespec,
    it_value: libc::timespec,
}

extern "C" {
    fn timerfd_create(clockid: libc::c_int, flags: libc::c_int) -> std::os::unix::io::RawFd;
    fn timerfd_settime(fd: std::os::unix::io::RawFd,
                       flags: libc::c_int,
                       new_value: *const itimerspec,
                       old_value: *mut itimerspec)
                       -> libc::c_int;
}

// TODO: Use /usr/include/sys/timerfd.h
static TFD_CLOEXEC: libc::c_int = 0o2000000;
static TFD_TIMER_ABSTIME: libc::c_int = 1 << 0;

#[derive(Debug)]
pub struct TimerFd {
    fd: std::os::unix::io::RawFd,
}

impl TimerFd {
    pub fn new<Tz>(time: chrono::DateTime<Tz>) -> std::io::Result<Self>
        where Tz: chrono::TimeZone
    {
        let fd = unsafe { timerfd_create(libc::CLOCK_REALTIME, TFD_CLOEXEC) };
        if fd == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            let new_value = itimerspec {
                it_interval: libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: libc::timespec {
                    tv_sec: time.timestamp(),
                    tv_nsec: time.timestamp_subsec_nanos() as i64,
                },
            };
            let rc =
                unsafe { timerfd_settime(fd, TFD_TIMER_ABSTIME, &new_value, std::ptr::null_mut()) };
            if rc == 0 {
                Ok(Self { fd: fd })
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }

    pub fn read(&self) -> std::io::Result<u64> {
        let mut buf = 0 as u64;
        let ptr: *mut u64 = &mut buf;
        let rc = unsafe { libc::read(self.fd, ptr as *mut libc::c_void, 8) };
        if rc == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(buf)
        }
    }
}

impl Drop for TimerFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

impl std::os::unix::io::AsRawFd for TimerFd {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.fd
    }
}
