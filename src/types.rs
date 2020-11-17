#[derive(sqlx::FromRow)]
pub struct Job {
    pub pid: i32,
    pub tid: i32,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub channel_name: String,
    pub channel_for_recorder: i32,
    pub channel_for_syoboi: i32,
    pub enqueued_at: chrono::NaiveDateTime,
    pub count: String,
    pub start_offset: i32,
    pub subtitle: Option<String>,
    pub title: Option<String>,
    pub comment: Option<String>,
}
impl Job {
    pub const fn select_sql() -> &'static str {
        r#"
            select
                j.pid
                , p.tid
                , p.start_time
                , p.end_time
                , c.name as channel_name
                , c.for_recorder as channel_for_recorder
                , c.for_syoboi as channel_for_syoboi
                , p.count
                , p.start_offset
                , p.subtitle
                , p.title
                , p.comment
                , j.enqueued_at
            from
                jobs j
                inner join programs p on p.pid = j.pid
                inner join channels c on c.id = p.channel_id
            where
                enqueued_at >= $1
            order by j.enqueued_at
            "#
    }
}

#[derive(sqlx::FromRow)]
pub struct Program {
    pub pid: i32,
    pub tid: i32,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub channel_name: String,
    pub recorder_channel: i32,
}
impl Program {
    pub const fn select_sql() -> &'static str {
        r#"
        select
            p.pid
            , p.tid
            , p.start_time
            , p.end_time
            , c.name as channel_name
            , c.for_recorder as recorder_channel
        from programs p inner join channels c on c.id = p.channel_id
        where p.pid = $1
        "#
    }
}
