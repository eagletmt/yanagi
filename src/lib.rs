#[macro_use]
extern crate serde_derive;

mod epollfd;
pub use epollfd::EpollFd;

pub mod syoboi_calendar;

pub mod signalfd;
pub use signalfd::SignalFd;

mod timerfd;
pub use timerfd::TimerFd;
