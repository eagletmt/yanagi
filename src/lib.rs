pub mod proto {
    pub mod resources {
        tonic::include_proto!("yanagi.resources");
    }
    pub mod services {
        tonic::include_proto!("yanagi.services");
    }
}

pub mod server;
pub(crate) mod types;
pub(crate) mod services {
    pub mod scheduler;
    pub mod system;
}
pub mod syoboi_calendar;
