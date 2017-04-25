extern crate chrono;
extern crate time;
extern crate yanagi;

fn main() {
    let target = chrono::Local::now() + time::Duration::seconds(3);
    let timerfd = yanagi::TimerFd::new(target).expect("Unable to create timerfd");
    println!("{:?}", timerfd);
    let expirations = timerfd.read().expect("Unable to read timerfd");
    let now = chrono::Local::now();
    println!("Read {}", expirations);
    println!("target: {}", target);
    println!("now:    {}", now);
}
