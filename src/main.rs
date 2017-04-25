extern crate chrono;
extern crate time;
extern crate yanagi;

fn main() {
    {
        let signalfd =
            yanagi::SignalFd::new(&[yanagi::signalfd::SIGINT]).expect("Unable to create signalfd");
        println!("{:?}", signalfd);
        let ssi = signalfd.read().expect("Unable to read signalfd");
        match ssi.ssi_signo as i32 {
            yanagi::signalfd::SIGINT => {
                println!("Got SIGINT");
            }
            signum => panic!("Unexpected ssi_signo {}", signum),
        }
    }
    let target = chrono::Local::now() + time::Duration::seconds(3);
    let timerfd = yanagi::TimerFd::new(target).expect("Unable to create timerfd");
    println!("{:?}", timerfd);
    let expirations = timerfd.read().expect("Unable to read timerfd");
    let now = chrono::Local::now();
    println!("Read {}", expirations);
    println!("target: {}", target);
    println!("now:    {}", now);
}
