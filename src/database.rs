extern crate chrono;
extern crate postgres;

pub struct Database {
    conn: postgres::Connection,
}

pub struct ScheduledJob {
    pub pid: i32,
    pub enqueued_at: chrono::DateTime<chrono::Local>,
}

impl Database {
    pub fn new<T>(params: T) -> Result<Self, postgres::error::ConnectError>
        where T: postgres::params::IntoConnectParams
    {
        Ok(Self { conn: postgres::Connection::connect(params, postgres::TlsMode::None)? })
    }

    pub fn initialize_tables(&self) -> Result<(), postgres::error::Error> {
        self.conn
            .execute(r#"
        create table if not exists jobs (
            pid integer not null primary key
            , enqueued_at timestamp with time zone not null
            , finished_at timestamp with time zone
            , created_at timestamp with time zone not null
        )
        "#,
                     &[])?;
        Ok(())
    }

    pub fn get_jobs(&self,
                    now: &chrono::DateTime<chrono::Local>)
                    -> Result<Vec<ScheduledJob>, postgres::error::Error> {
        let rows = self.conn
            .query("select pid, enqueued_at from jobs where enqueued_at >= $1 order by enqueued_at",
                   &[now])?;
        Ok(rows.into_iter()
               .map(|row| {
                        ScheduledJob {
                            pid: row.get("pid"),
                            enqueued_at: row.get("enqueued_at"),
                        }
                    })
               .collect())
    }
}
