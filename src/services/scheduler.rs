use crate::proto::resources as pb;
use crate::proto::services as pb_service;
use crate::types::Job;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SchedulerService {
    jobs: Arc<Mutex<Vec<Job>>>,
    pool: sqlx::PgPool,
}
impl SchedulerService {
    pub fn new(pool: sqlx::PgPool, jobs: Arc<Mutex<Vec<Job>>>) -> Self {
        Self { pool, jobs }
    }
}

#[tonic::async_trait]
impl pb_service::scheduler_server::Scheduler for SchedulerService {
    async fn get_jobs(
        &self,
        _request: tonic::Request<pb_service::GetJobsRequest>,
    ) -> Result<tonic::Response<pb_service::GetJobsResponse>, tonic::Status> {
        let jobs = self
            .jobs
            .lock()
            .await
            .iter()
            .map(|job| pb::Job {
                pid: job.pid as u32,
                tid: job.tid as u32,
                start_time: Some(prost_types::Timestamp {
                    seconds: job.start_time.timestamp(),
                    nanos: job.start_time.timestamp_subsec_nanos() as i32,
                }),
                end_time: Some(prost_types::Timestamp {
                    seconds: job.end_time.timestamp(),
                    nanos: job.end_time.timestamp_subsec_nanos() as i32,
                }),
                channel_name: job.channel_name.clone(),
                channel_for_recorder: job.channel_for_recorder as u32,
                channel_for_syoboi: job.channel_for_syoboi as u32,
                count: job.count.clone(),
                start_offset: job.start_offset,
                subtitle: job
                    .subtitle
                    .as_ref()
                    .map(String::clone)
                    .unwrap_or_else(|| "".to_owned()),
                title: job
                    .title
                    .as_ref()
                    .map(String::clone)
                    .unwrap_or_else(|| "".to_owned()),
                comment: job
                    .comment
                    .as_ref()
                    .map(String::clone)
                    .unwrap_or_else(|| "".to_owned()),
                enqueued_at: None,
            })
            .collect();
        Ok(tonic::Response::new(pb_service::GetJobsResponse { jobs }))
    }

    async fn track_tid(
        &self,
        request: tonic::Request<pb_service::TrackTidRequest>,
    ) -> Result<tonic::Response<pb_service::TrackTidResponse>, tonic::Status> {
        let message = request.into_inner();
        match track_tid_impl(&self.pool, message.tid).await {
            Ok(title) => {
                if let Some(title) = title {
                    Ok(tonic::Response::new(pb_service::TrackTidResponse {
                        tid: message.tid,
                        title,
                    }))
                } else {
                    Err(tonic::Status::invalid_argument("No such TID"))
                }
            }
            Err(e) => Err(tonic::Status::internal(e.to_string())),
        }
    }
}

async fn track_tid_impl(pool: &sqlx::PgPool, tid: u32) -> Result<Option<String>, anyhow::Error> {
    if let Some(title) = crate::syoboi_calendar::title_medium(tid).await? {
        sqlx::query("insert into tracking_titles (tid, created_at) values ($1, now())")
            .bind(tid)
            .execute(pool)
            .await?;
        Ok(Some(title))
    } else {
        Ok(None)
    }
}
