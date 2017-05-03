extern crate chrono;
extern crate iron;
extern crate router;
extern crate serde;
extern crate serde_json;
extern crate std;

pub struct Web {
    router: router::Router,
}

impl Web {
    pub fn new(db_arc: std::sync::Arc<super::Database>) -> Self {
        let router = router! {
            schedules_index: get "/v1/schedules" => SchedulesIndexHandler::new(db_arc.clone()),
            channels_index: get "/v1/channels" => ChannelsIndexHandler::new(db_arc.clone()),
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

fn respond_json<T, E>(result: Result<T, E>) -> Result<iron::Response, iron::IronError>
    where T: serde::Serialize,
          E: std::error::Error
{
    use web::iron::Set;

    let mut response = iron::Response::new();
    response.headers.set(iron::headers::ContentType::json());
    match result {
        Ok(body) => {
            match serde_json::to_string(&body) {
                Ok(json) => Ok(response.set((iron::status::Ok, json))),
                Err(err) => Ok(error_response(response, err)),
            }
        }
        Err(err) => Ok(error_response(response, err)),
    }
}

struct SchedulesIndexHandler {
    db_arc: std::sync::Arc<super::Database>,
}
impl SchedulesIndexHandler {
    fn new(db_arc: std::sync::Arc<super::Database>) -> Self {
        Self { db_arc: db_arc }
    }
}
impl iron::middleware::Handler for SchedulesIndexHandler {
    fn handle(&self, _: &mut iron::Request) -> Result<iron::Response, iron::IronError> {
        let now = chrono::Local::now();
        respond_json(self.db_arc.get_schedules(&now))
    }
}

struct ChannelsIndexHandler {
    db_arc: std::sync::Arc<super::Database>,
}
impl ChannelsIndexHandler {
    fn new(db_arc: std::sync::Arc<super::Database>) -> Self {
        Self { db_arc: db_arc }
    }
}
impl iron::middleware::Handler for ChannelsIndexHandler {
    fn handle(&self, _: &mut iron::Request) -> Result<iron::Response, iron::IronError> {
        respond_json(self.db_arc.get_channels())
    }
}
