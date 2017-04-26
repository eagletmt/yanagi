extern crate chrono;
extern crate time;
extern crate yanagi;

use std::os::unix::io::AsRawFd;

fn main() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<std::thread::JoinHandle<()>>>();

    let signalfd =
        yanagi::SignalFd::new(&[yanagi::signalfd::SIGINT]).expect("Unable to create signalfd");
    let target = chrono::Local::now() + time::Duration::seconds(3);
    let timerfd = yanagi::TimerFd::new(target).expect("Unable to create timerfd");

    let waiter = std::thread::spawn(move || loop {
                                        match rx.recv() {
                                            Ok(Some(t)) => {
            println!("Received thread");
            let r = t.join();
            if let Err(_) = r {
                println!("Thread paniced");
            }
        }
                                            Ok(None) => {
            println!("Stop waiter");
            break;
        }
                                            Err(e) => {
            panic!("Unable to recv thread: {}", e);
        }
                                        }
                                    });

    let epollfd = yanagi::EpollFd::new().expect("Unable to create epoll");
    epollfd
        .add_in(&signalfd)
        .expect("Unable to add signalfd");
    epollfd.add_in(&timerfd).expect("Unable to add timerfd");

    'eventloop: loop {
        let fds = epollfd.wait(64).expect("Unable to epoll_wait");
        println!("Received {} events", fds.len());
        for fd in fds {
            if fd == signalfd.as_raw_fd() {
                let ssi = signalfd.read().expect("Unable to read signalfd");
                match ssi.ssi_signo as i32 {
                    yanagi::signalfd::SIGINT => {
                        println!("Got SIGINT");
                        tx.send(None).expect("Unable to send stop request");
                    }
                    signum => panic!("Unexpected ssi_signo {}", signum),
                }
                break 'eventloop;
            } else if fd == timerfd.as_raw_fd() {
                let expirations = timerfd.read().expect("Unable to read timerfd");
                let now = chrono::Local::now();
                println!("Read {}", expirations);
                println!("target: {}", target);
                println!("now:    {}", now);
                let thread = std::thread::spawn(move || {
                                                    let second = 5;
                                                    println!("Sleep {} seconds", second);
                                                    let d = std::time::Duration::new(second, 0);
                                                    std::thread::sleep(d);
                                                });
                tx.send(Some(thread))
                    .expect("Unable to send thread handle");
            } else {
                panic!("Unknown fd {}", fd);
            }
        }
    }

    println!("Waiting waiter...");
    waiter.join().expect("Unable to join waiter thread");
}
