use crate::proto::services as pb_service;
use crate::types::{Job, Program};
use futures::StreamExt as _;
use std::sync::Arc;
use tokio::sync::RwLock;

enum Task {
    Shutdown,
    Reload,
    StartRecorder(i32),
}

pub async fn start() -> Result<(), anyhow::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect("postgresql://kaede@longarch.enospc.tv/kaede")
        .await?;

    let shared_jobs = Arc::new(RwLock::new(Vec::new()));
    let scheduler_service =
        crate::services::scheduler::SchedulerService::new(pool.clone(), shared_jobs.clone());
    let (stop_tx, stop_rx) = futures::channel::oneshot::channel();
    let shutdown_signal = async move {
        if let Err(e) = stop_rx.await {
            println!("tokio::sync::oneshot::Receiver error: {:?}", e);
        }
    };
    let (reload_tx, reload_rx) = futures::channel::mpsc::channel(1);
    let system_service =
        crate::services::system::SystemService::new(stop_tx, reload_tx, pool.clone());
    let reload_stream = reload_rx.map(|_| Ok(Task::Reload));
    let server = tonic::transport::Server::builder()
        .add_service(pb_service::system_server::SystemServer::new(system_service))
        .add_service(pb_service::scheduler_server::SchedulerServer::new(
            scheduler_service,
        ));
    let server_handle = if let Some(l) = listenfd::ListenFd::from_env().take_tcp_listener(0)? {
        let listener = tokio::net::TcpListener::from_std(l)?;
        tokio::spawn(server.serve_with_incoming_shutdown(listener, shutdown_signal))
    } else {
        tokio::spawn(server.serve_with_shutdown("127.0.0.1:50051".parse()?, shutdown_signal))
    };
    let server_shutdown_stream = futures::stream::once(server_handle).map(|r| match r {
        Ok(r) => match r {
            Ok(_) => Ok(Task::Shutdown),
            Err(e) => Err(anyhow::Error::from(e)),
        },
        Err(e) => Err(anyhow::Error::from(e)),
    });
    let mut system_stream = futures::stream::select(server_shutdown_stream, reload_stream);

    let (mut recorder_tx, recorder_rx) = futures::channel::mpsc::channel(100);
    let waiter_handle = tokio::spawn(async {
        let mut recorder_rx = recorder_rx;
        while let Some(fut) = recorder_rx.next().await {
            if let Err(e) = fut.await {
                eprintln!("Record failure: {:?}", e);
            }
        }
    });
    'main: loop {
        let now = chrono::Local::now().naive_local();
        let mut rows = sqlx::query_as(Job::select_sql()).bind(&now).fetch(&pool);
        let mut delay_queue = tokio::time::DelayQueue::new();
        let mut jobs = Vec::new();
        while let Some(job) = rows.next().await {
            let job: Job = job?;
            println!("[{}] Enqueue at {}", job.pid, job.enqueued_at);
            delay_queue.insert(
                job.pid,
                job.enqueued_at.signed_duration_since(now).to_std()?,
            );
            jobs.push(job);
        }
        let delay_stream = delay_queue.map(|r| match r {
            Ok(expired) => Ok(Task::StartRecorder(expired.into_inner())),
            Err(e) => Err(anyhow::Error::from(e)),
        });
        *shared_jobs.write().await = jobs;

        let mut stream = futures::stream::select(system_stream, delay_stream);
        while let Some(task) = stream.next().await {
            match task? {
                Task::Shutdown => {
                    println!("Shutdown");
                    break 'main;
                }
                Task::Reload => {
                    println!("Reload");
                    break;
                }
                Task::StartRecorder(pid) => {
                    let handle = tokio::spawn(start_recorder(pool.clone(), pid));
                    recorder_tx.try_send(handle)?;
                }
            }
        }
        system_stream = stream.into_inner().0;
    }
    recorder_tx.close_channel();
    waiter_handle.await?;
    Ok(())
}

async fn start_recorder(pool: sqlx::PgPool, pid: i32) -> Result<(), anyhow::Error> {
    let program: Program = sqlx::query_as(Program::select_sql())
        .bind(pid)
        .fetch_one(&pool)
        .await?;
    let mut duration = program
        .end_time
        .signed_duration_since(program.start_time)
        .num_seconds();
    if program.channel_name.contains("NHK") {
        duration += 25;
    }
    let path =
        std::path::Path::new("/mnt/heidemarie").join(format!("{}_{}.ts", program.pid, program.tid));
    println!(
        "[{}] Start recpt1 {} {} {}",
        pid,
        program.recorder_channel,
        duration,
        path.display()
    );
    let status = tokio::process::Command::new("sleep")
        .arg(&format!("{}", duration))
        .status()
        .await;
    println!("[{}] Finish: {:?}", pid, status);
    Ok(())
}
