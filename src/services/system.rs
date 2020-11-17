use crate::proto::services as pb_service;
use std::cell::Cell;
use tokio::sync::Mutex;

pub struct SystemService {
    stop_tx: Mutex<Cell<Option<futures::channel::oneshot::Sender<()>>>>,
    reload_tx: Mutex<Cell<futures::channel::mpsc::Sender<()>>>,
    pool: sqlx::PgPool,
}

impl SystemService {
    pub fn new(
        stop_tx: futures::channel::oneshot::Sender<()>,
        reload_tx: futures::channel::mpsc::Sender<()>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            stop_tx: Mutex::new(Cell::new(Some(stop_tx))),
            reload_tx: Mutex::new(Cell::new(reload_tx)),
            pool,
        }
    }
}

#[tonic::async_trait]
impl pb_service::system_server::System for SystemService {
    async fn stop(
        &self,
        _request: tonic::Request<pb_service::StopRequest>,
    ) -> Result<tonic::Response<pb_service::StopResponse>, tonic::Status> {
        let tx_cell = self.stop_tx.lock().await;
        let tx_option = tx_cell.replace(None);
        match tx_option {
            Some(stop_tx) => match stop_tx.send(()) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to send stop request: {:?}", e);
                    return Err(tonic::Status::internal(format!("{:?}", e)));
                }
            },
            None => {
                println!("Already stopping...");
            }
        }
        Ok(tonic::Response::new(pb_service::StopResponse {}))
    }

    async fn reload(
        &self,
        _request: tonic::Request<pb_service::ReloadRequest>,
    ) -> Result<tonic::Response<pb_service::ReloadResponse>, tonic::Status> {
        match reload_impl(&self.reload_tx).await {
            Ok(_) => Ok(tonic::Response::new(pb_service::ReloadResponse {})),
            Err(e) => Err(tonic::Status::internal(e.to_string())),
        }
    }

    async fn refresh(
        &self,
        _request: tonic::Request<pb_service::RefreshRequest>,
    ) -> Result<tonic::Response<pb_service::RefreshResponse>, tonic::Status> {
        match refresh_impl(&self.pool).await {
            Ok(_) => {
                reload_impl(&self.reload_tx)
                    .await
                    .map_err(|e| tonic::Status::internal(e.to_string()))?;
                Ok(tonic::Response::new(pb_service::RefreshResponse {}))
            }
            Err(e) => Err(tonic::Status::internal(e.to_string())),
        }
    }
}

async fn reload_impl(
    reload_tx: &Mutex<Cell<futures::channel::mpsc::Sender<()>>>,
) -> Result<(), anyhow::Error> {
    let mut tx_cell = reload_tx.lock().await;
    tx_cell.get_mut().try_send(())?;
    Ok(())
}

async fn refresh_impl(pool: &sqlx::PgPool) -> Result<(), anyhow::Error> {
    use futures::StreamExt as _;

    let mut channels = std::collections::HashMap::new();
    let mut rows = sqlx::query_as("select id, for_syoboi from channels").fetch(pool);
    #[derive(sqlx::FromRow)]
    struct Channel {
        id: i32,
        for_syoboi: i32,
    }
    while let Some(channel) = rows.next().await {
        let channel: Channel = channel?;
        channels.insert(channel.for_syoboi, channel.id);
    }

    let mut tracking_tids = std::collections::HashSet::new();
    let mut rows = sqlx::query_as("select tid from tracking_titles").fetch(pool);
    #[derive(sqlx::FromRow)]
    struct TrackingTitle {
        tid: i32,
    }
    while let Some(tracking_title) = rows.next().await {
        let tracking_title: TrackingTitle = tracking_title?;
        tracking_tids.insert(tracking_title.tid);
    }

    let mut job_pids = std::collections::HashSet::new();
    let mut rows = sqlx::query_as("select pid from jobs").fetch(pool);
    #[derive(sqlx::FromRow)]
    struct Job {
        pid: i32,
    }
    while let Some(job) = rows.next().await {
        let job: Job = job?;
        job_pids.insert(job.pid);
    }

    let prog_items = crate::syoboi_calendar::cal_chk().await?;

    let mut tx = pool.begin().await?;
    for prog_item in prog_items {
        if let Some(channel_id) = channels.get(&prog_item.ch_id) {
            const UPSERT_SQL: &str = r#"
                insert into programs (pid, tid, start_time, end_time, channel_id, count, start_offset, subtitle, title, comment)
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                on conflict (pid) do update set tid = $2, start_time = $3, end_time = $4, channel_id = $5, count = $6, start_offset = $7, subtitle = $8, title = $9, comment = $10
            "#;
            sqlx::query(UPSERT_SQL)
                .bind(prog_item.pid)
                .bind(prog_item.tid)
                .bind(prog_item.st_time)
                .bind(prog_item.ed_time)
                .bind(channel_id)
                .bind(prog_item.count)
                .bind(prog_item.st_offset)
                .bind(prog_item.sub_title)
                .bind(prog_item.title)
                .bind(prog_item.prog_comment)
                .execute(&mut tx)
                .await?;
            if tracking_tids.contains(&prog_item.tid) {
                const UPSERT_SQL: &str = r#"
                    insert into jobs (pid, enqueued_at, created_at)
                    values ($1, $2, $3)
                    on conflict (pid) do update set enqueued_at = $2
                "#;
                const JOB_TIME_GAP: i64 = 15;
                let enqueued_at = prog_item.st_time
                    + chrono::Duration::seconds(prog_item.st_offset - JOB_TIME_GAP);
                let now = chrono::Local::now().naive_local();
                sqlx::query(UPSERT_SQL)
                    .bind(prog_item.pid)
                    .bind(enqueued_at)
                    .bind(now)
                    .execute(&mut tx)
                    .await?;
            }
        }
        job_pids.remove(&prog_item.pid);
    }
    tx.commit().await?;

    for job_pid in job_pids {
        println!("Program {} has gone away. Delete its job", job_pid);
        sqlx::query("delete from jobs where pid = $1")
            .bind(job_pid)
            .execute(pool)
            .await?;
    }
    Ok(())
}
