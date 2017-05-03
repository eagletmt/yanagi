extern crate chrono;
extern crate hyper;
extern crate iron;
extern crate yanagi;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate router;

use std::os::unix::io::AsRawFd;

fn main() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<std::thread::JoinHandle<()>>>();
    let db = yanagi::Database::new("postgresql://eagletmt@localhost/yanagi").expect("Unable to connect to PostgreSQL");
    let now = chrono::Local::now();

    db.initialize_tables()
        .expect("Unable to initialize tables");
    let jobs = db.get_jobs(&now).expect("Unable to get jobs");

    let signalfd =
        yanagi::SignalFd::new(&[yanagi::signalfd::SIGINT]).expect("Unable to create signalfd");
    let epollfd = yanagi::EpollFd::new().expect("Unable to create epoll");
    epollfd
        .add_in(&signalfd)
        .expect("Unable to add signalfd");

    let db_arc = std::sync::Arc::new(std::sync::Mutex::new(db));

    let mut listening =
        run_web("localhost:3000", db_arc.clone()).expect("Unable start HTTP server");

    let mut timerfds = std::collections::HashMap::new();
    for job in jobs {
        let timerfd = yanagi::TimerFd::new(job.enqueued_at).expect("Unable to create timerfd");
        epollfd.add_in(&timerfd).expect("Unable to add timerfd");
        timerfds.insert(timerfd.as_raw_fd(), (timerfd, job));
    }

    let waiter = std::thread::spawn(move || run_waiter(rx));

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
            } else if let Some((timerfd, job)) = timerfds.remove(&fd) {
                let expirations = timerfd.read().expect("Unable to read timerfd");
                println!("Read {}", expirations);
                println!("now:         {}", chrono::Local::now());
                println!("enqueued_at: {}", job.enqueued_at);
                println!("pid: {}", job.pid);
                let arc = db_arc.clone();
                let thread = std::thread::spawn(move || get_programs(arc, job.pid));
                tx.send(Some(thread))
                    .expect("Unable to send thread handle");
                epollfd
                    .del(&timerfd)
                    .expect("Unable to delete timerfd from epollfd");
            } else {
                panic!("Unknown fd {}", fd);
            }
        }
    }

    println!("Waiting waiter...");
    waiter.join().expect("Unable to join waiter thread");
    // FIXME: This is NOT graceful shutdown.
    // https://github.com/hyperium/hyper/pull/1013
    println!("Shutting down server...");
    listening.close().expect("Unable to shutdown server");
    println!("Finished");
}

fn run_waiter(rx: std::sync::mpsc::Receiver<Option<std::thread::JoinHandle<()>>>) {
    loop {
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
    }
}

fn get_programs(db_arc: std::sync::Arc<std::sync::Mutex<yanagi::Database>>, pid: i32) {
    let program = db_arc
        .lock()
        .expect("Unable to lock Database mutex")
        .get_program(pid)
        .expect("Unable to get program")
        .expect("Unable to find specified program");
    let duration = program.ed_time.timestamp() - program.st_time.timestamp();
    println!("Record {} for {} seconds", program.filename(), duration);

    std::thread::sleep(std::time::Duration::new(duration as u64, 0));

    let program = db_arc
        .lock()
        .expect("Unable to lock Database mutex")
        .get_program(pid)
        .expect("Unable to get program")
        .expect("Unable to find specified program");
    println!("Record {} finished", program.filename());
    /*
    let hyper = hyper::Client::new();
    let syoboi = yanagi::syoboi_calendar::Client::new(hyper);
    let response = syoboi
        .cal_chk(&yanagi::syoboi_calendar::CalChkRequest::default())
        .expect("Unable to get cal_chk.php");
    println!("Got prog items");
    for prog in response.prog_items.prog_items {
        println!("{} - {} {} #{} {} ({})",
                 prog.start_time(),
                 prog.end_time(),
                 prog.title,
                 prog.count,
                 prog.sub_title,
                 prog.ch_name);
    }
    */
}

fn run_web<A>(addr: A,
              db_arc: std::sync::Arc<std::sync::Mutex<yanagi::Database>>)
              -> Result<iron::Listening, iron::error::HttpError>
    where A: std::net::ToSocketAddrs
{
    let router = router!{
        jobs_index: get "/v1/jobs" => JobsIndexHandler::new(db_arc.clone()),
    };
    iron::Iron::new(router).http(addr)
}

struct JobsIndexHandler {
    db_arc: std::sync::Arc<std::sync::Mutex<yanagi::Database>>,
}

impl JobsIndexHandler {
    fn new(db_arc: std::sync::Arc<std::sync::Mutex<yanagi::Database>>) -> Self {
        Self { db_arc: db_arc }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

fn error_response<E>(response: iron::Response, err: E) -> iron::Response
    where E: std::error::Error
{
    use iron::Set;

    let json = serde_json::to_string(&ErrorResponse { error: err.description() }).expect("Unable to serialize error message");
    response.set((iron::status::InternalServerError, json))
}

impl iron::middleware::Handler for JobsIndexHandler {
    fn handle(&self, _: &mut iron::Request) -> Result<iron::Response, iron::IronError> {
        use iron::Set;

        let now = chrono::Local::now();
        let mut response = iron::Response::new();
        response.headers.set(iron::headers::ContentType::json());
        match self.db_arc
                  .lock()
                  .expect("Unable to lock Database mutex")
                  .get_jobs(&now) {
            Ok(jobs) => {
                match serde_json::to_string(&jobs) {
                    Ok(jobs_json) => Ok(response.set((iron::status::Ok, jobs_json))),
                    Err(err) => Ok(error_response(response, err)),
                }
            }
            Err(err) => Ok(error_response(response, err)),
        }
    }
}
