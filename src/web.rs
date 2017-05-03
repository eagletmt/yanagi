extern crate chrono;
extern crate iron;
extern crate router;
extern crate serde_json;
extern crate std;

pub struct Web {
    router: router::Router,
}

impl Web {
    pub fn new(db_arc: std::sync::Arc<super::Database>) -> Self {
        let router = router! {
            jobs_index: get "/v1/jobs" => JobsIndexHandler::new(db_arc.clone()),
        };

        Self { router: router }
    }
}

impl iron::middleware::Handler for Web {
    fn handle(&self, request: &mut iron::Request) -> Result<iron::Response, iron::IronError> {
        self.router.handle(request)
    }
}


#[derive(Debug, Serialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

fn error_response<E>(response: iron::Response, err: E) -> iron::Response
    where E: std::error::Error
{
    use web::iron::Set;

    let json = serde_json::to_string(&ErrorResponse { error: err.description() }).expect("Unable to serialize error message");
    response.set((iron::status::InternalServerError, json))
}

struct JobsIndexHandler {
    db_arc: std::sync::Arc<super::Database>,
}

impl JobsIndexHandler {
    fn new(db_arc: std::sync::Arc<super::Database>) -> Self {
        Self { db_arc: db_arc }
    }
}

impl iron::middleware::Handler for JobsIndexHandler {
    fn handle(&self, _: &mut iron::Request) -> Result<iron::Response, iron::IronError> {
        use web::iron::Set;

        let now = chrono::Local::now();
        let mut response = iron::Response::new();
        response.headers.set(iron::headers::ContentType::json());
        match self.db_arc.get_jobs(&now) {
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
