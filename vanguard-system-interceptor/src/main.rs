mod cli;

use std::collections::HashSet;

use clap::Parser;
use futures::StreamExt;
use uuid::Uuid;
use vanguard_core::{
    Interceptor, InterceptorState, PlatformInterceptor, Position, THREATS_SUBJECT, Threat,
};

use crate::cli::Args;

const DETECTION_RANGE: f64 = 1_500.0;
const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let position = Position {
        x: args.x,
        y: args.y,
    };

    let interceptors: Vec<Interceptor> = (0..args.interceptors)
        .map(|_| Interceptor {
            id: Uuid::new_v4(),
            position: position.clone(),
            state: InterceptorState::Idle,
        })
        .collect();

    let platform = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: args.name,
        position,
        interceptors,
        range: DETECTION_RANGE,
        neighbor_platforms: Vec::new(),
    };

    println!(
        "{} (id {}) online at ({:.0}, {:.0}) — radar range {:.0} m, {} interceptor(s) ready",
        platform.name,
        platform.id,
        platform.position.x,
        platform.position.y,
        platform.range,
        platform.interceptors.len(),
    );

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    let mut threat_updates = client.subscribe(THREATS_SUBJECT).await?;
    println!(
        "{} radar active — listening on `{THREATS_SUBJECT}` via {nats_url}",
        platform.name,
    );

    let mut acquired: HashSet<Uuid> = HashSet::new();

    while let Some(message) = threat_updates.next().await {
        let threats: Vec<Threat> = match serde_json::from_slice(&message.payload) {
            Ok(threats) => threats,
            Err(error) => {
                eprintln!("{} discarding invalid threat update: {error}", platform.name);
                continue;
            }
        };

        let mut in_range = Vec::new();
        for threat in &threats {
            let range = platform.position.distance(&threat.position);
            if range > platform.range {
                continue;
            }

            if acquired.insert(threat.id) {
                println!(
                    "{} RADAR CONTACT threat {} at ({:.0}, {:.0}) — range {:.0} m",
                    platform.name,
                    short(&threat.id),
                    threat.position.x,
                    threat.position.y,
                    range,
                );
            }
            in_range.push(format!("{} at {:.0} m", short(&threat.id), range));
        }

        if in_range.is_empty() {
            println!("{} radar: no contact", platform.name);
        } else {
            println!("{} radar: {}", platform.name, in_range.join(", "));
        }
    }

    Ok(())
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}
