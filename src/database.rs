extern crate chrono;
extern crate postgres;
extern crate serde;
extern crate std;

pub struct Database {
    conn: std::sync::Mutex<postgres::Connection>,
}

#[derive(Debug, Serialize)]
pub struct ScheduledJob {
    pub pid: i32,
    #[serde(serialize_with="serialize_datetime")]
    pub enqueued_at: chrono::DateTime<chrono::Local>,
}

fn serialize_datetime<S>(datetime: &chrono::DateTime<chrono::Local>,
                         serializer: S)
                         -> Result<S::Ok, S::Error>
    where S: serde::Serializer
{
    serializer.serialize_str(&datetime.to_rfc3339())
}

#[derive(Debug, Serialize)]
pub struct Program {
    pub pid: i32,
    pub tid: i32,
    #[serde(serialize_with="serialize_datetime")]
    pub st_time: chrono::DateTime<chrono::Local>,
    #[serde(serialize_with="serialize_datetime")]
    pub ed_time: chrono::DateTime<chrono::Local>,
    pub count: String,
    pub st_offset: i32,
    pub subtitle: String,
    pub title: String,
    pub comment: String,
    pub channel_name: String,
    pub channel_for_recorder: i32,
}

#[derive(Debug, Serialize)]
pub struct Schedule {
    #[serde(serialize_with="serialize_datetime")]
    pub enqueued_at: chrono::DateTime<chrono::Local>,
    pub program: Program,
}

#[derive(Debug, Serialize)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub for_recorder: i32,
    pub for_syoboi: i32,
}

impl Program {
    pub fn filename(&self) -> String {
        let mut fname = format!("{}_{} {} #{} {}",
                                self.tid,
                                self.pid,
                                self.title,
                                self.count,
                                self.subtitle);
        if !self.comment.is_empty() {
            fname.push_str(&format!(" ({})", self.comment));
        }
        fname.push_str(&format!(" at {}", self.channel_name));
        fname.replace("/", "／")
    }
}

impl Database {
    pub fn new<T>(params: T) -> Result<Self, postgres::error::ConnectError>
        where T: postgres::params::IntoConnectParams
    {
        Ok(Self {
               conn: std::sync::Mutex::new(postgres::Connection::connect(params,
                                                                         postgres::TlsMode::None)?),
           })
    }

    pub fn initialize_tables(&self) -> Result<(), postgres::error::Error> {
        let conn = self.conn.lock().expect("Unable to acquire lock");

        conn.execute(r#"
        create table if not exists channels (
            id serial not null primary key
            , name varchar(255) not null unique
            , for_recorder integer not null unique
            , for_syoboi integer not null unique
        )
        "#,
                     &[])?;
        conn.execute(r#"
        create table if not exists programs (
            pid integer not null primary key
            , tid integer not null
            , st_time timestamp with time zone not null
            , ed_time timestamp with time zone not null
            , channel_id integer not null references channels (id)
            , count varchar(16) not null
            , st_offset integer not null
            , subtitle varchar(255) not null
            , title varchar(255) not null
            , comment varchar(255) not null
        )
        "#,
                     &[])?;
        conn.execute(r#"
        create table if not exists jobs (
            pid integer not null primary key references programs (pid)
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
        let conn = self.conn.lock().expect("Unable to acquire lock");
        let rows = conn.query("select pid, enqueued_at from jobs where enqueued_at >= $1 order by enqueued_at",
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

    pub fn get_schedules(&self,
                         now: &chrono::DateTime<chrono::Local>)
                         -> Result<Vec<Schedule>, postgres::error::Error> {
        let conn = self.conn.lock().expect("Unable to acquire lock");
        let rows = conn.query(r#"
            select
                j.enqueued_at
                , p.pid
                , p.tid
                , p.st_time
                , p.ed_time
                , p.count
                , p.st_offset
                , p.subtitle
                , p.title
                , p.comment
                , c.name as channel_name
                , c.for_recorder as channel_for_recorder
            from
                jobs j
                inner join programs p using (pid)
                inner join channels c on c.id = p.channel_id
            where
                j.enqueued_at >= $1
            order by j.enqueued_at
        "#,
                              &[now])?;
        Ok(rows.into_iter()
               .map(|row| {
            Schedule {
                enqueued_at: row.get("enqueued_at"),
                program: Program {
                    pid: row.get("pid"),
                    tid: row.get("tid"),
                    st_time: row.get("st_time"),
                    ed_time: row.get("ed_time"),
                    count: row.get("count"),
                    st_offset: row.get("st_offset"),
                    subtitle: row.get("subtitle"),
                    title: row.get("title"),
                    comment: row.get("comment"),
                    channel_name: row.get("channel_name"),
                    channel_for_recorder: row.get("channel_for_recorder"),
                },
            }
        })
               .collect())
    }


    pub fn get_program(&self, pid: i32) -> Result<Option<Program>, postgres::error::Error> {
        let conn = self.conn.lock().expect("Unable to acquire lock");
        let rows = conn.query(r#"
            select
                p.pid
                , p.tid
                , p.st_time
                , p.ed_time
                , p.count
                , p.st_offset
                , p.subtitle
                , p.title
                , p.comment
                , c.name as channel_name
                , c.for_recorder as channel_for_recorder
            from
                programs p
                inner join channels c on c.id = p.channel_id
            where p.pid = $1
        "#,
                              &[&pid])?;

        Ok(rows.into_iter()
               .next()
               .map(|row| {
            Program {
                pid: row.get("pid"),
                tid: row.get("tid"),
                st_time: row.get("st_time"),
                ed_time: row.get("ed_time"),
                count: row.get("count"),
                st_offset: row.get("st_offset"),
                subtitle: row.get("subtitle"),
                title: row.get("title"),
                comment: row.get("comment"),
                channel_name: row.get("channel_name"),
                channel_for_recorder: row.get("channel_for_recorder"),
            }
        }))
    }

    pub fn get_channels(&self) -> Result<Vec<Channel>, postgres::error::Error> {
        let conn = self.conn.lock().expect("Unable to acquire lock");
        let rows = conn.query("select id, name, for_recorder, for_syoboi from channels",
                              &[])?;
        Ok(rows.into_iter()
               .map(|row| {
                        Channel {
                            id: row.get("id"),
                            name: row.get("name"),
                            for_recorder: row.get("for_recorder"),
                            for_syoboi: row.get("for_syoboi"),
                        }
                    })
               .collect())
    }
}
