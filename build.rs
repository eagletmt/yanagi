fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "proto/yanagi/resources/job.proto",
            "proto/yanagi/services/scheduler.proto",
            "proto/yanagi/services/system.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
