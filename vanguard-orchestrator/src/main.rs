mod assignment;
mod orchestrator;
mod state;
mod subjects;
mod tracks;

use anyhow::Result;
use orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    let nats = async_nats::connect("nats://localhost:4222").await?;

    Orchestrator::new(nats).run().await
}
