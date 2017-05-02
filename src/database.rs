extern crate chrono;
extern crate postgres;

pub struct Database {
    conn: postgres::Connection,
}

#[derive(Debug)]
pub struct ScheduledJob {
    pub pid: i32,
    pub enqueued_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug)]
pub struct Program {
    pub pid: i32,
    pub tid: i32,
    pub st_time: chrono::DateTime<chrono::Local>,
    pub ed_time: chrono::DateTime<chrono::Local>,
    pub count: String,
    pub st_offset: i32,
    pub subtitle: String,
    pub title: String,
    pub comment: String,
    pub channel_name: String,
    pub channel_for_recorder: i32,
}

#[derive(Debug)]
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
        Ok(Self { conn: postgres::Connection::connect(params, postgres::TlsMode::None)? })
    }

    pub fn initialize_tables(&self) -> Result<(), postgres::error::Error> {
        self.conn
            .execute(r#"
        create table if not exists channels (
            id serial not null primary key
            , name varchar(255) not null unique
            , for_recorder integer not null unique
            , for_syoboi integer not null unique
        )
        "#,
                     &[])?;
        self.conn
            .execute(r#"
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
        self.conn
            .execute(r#"
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

    pub fn get_program(&self, pid: i32) -> Result<Option<Program>, postgres::error::Error> {
        let rows = self.conn
            .query(r#"
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
}
