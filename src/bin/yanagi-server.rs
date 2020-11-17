#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    yanagi::server::start().await
}
