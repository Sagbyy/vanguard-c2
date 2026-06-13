//! Platform host: runs every interceptor platform as a radar driven by the
//! ground-truth threat feed, and lets the UI add/remove platforms live.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use uuid::Uuid;
use vanguard_core::{
    CONTROL_RESET, PLATFORM_ADD, PLATFORM_REMOVE, PlatformSpec, Position, Radar, THREATS_SUBJECT,
    Threat, report_subject,
};

const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";
const CLASSIFICATION_RANGE: f64 = 8_000.0;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    let mut threats_sub = client.subscribe(THREATS_SUBJECT).await?;
    let mut add_sub = client.subscribe(PLATFORM_ADD).await?;
    let mut remove_sub = client.subscribe(PLATFORM_REMOVE).await?;
    let mut reset_sub = client.subscribe(CONTROL_RESET).await?;
    println!("control host online via {nats_url}");

    let mut radars = preset_radars();

    loop {
        tokio::select! {
            Some(msg) = threats_sub.next() => {
                let threats: Vec<Threat> = match serde_json::from_slice(&msg.payload) {
                    Ok(t) => t,
                    Err(error) => {
                        eprintln!("invalid threats: {error}");
                        continue;
                    }
                };
                let now_ms = unix_timestamp_ms();
                for radar in radars.values_mut() {
                    let report = radar.observe(&threats, now_ms);
                    let subject = report_subject(&report.platform_id);
                    if let Ok(payload) = serde_json::to_vec(&report) {
                        let _ = client.publish(subject, payload.into()).await;
                    }
                }
            }

            Some(msg) = add_sub.next() => {
                match serde_json::from_slice::<PlatformSpec>(&msg.payload) {
                    Ok(spec) => {
                        println!("+ platform {} at ({:.0}, {:.0}) reach {:.0}", spec.name, spec.position.x, spec.position.y, spec.reach);
                        radars.insert(spec.id, Radar::new(spec, CLASSIFICATION_RANGE));
                    }
                    Err(error) => eprintln!("invalid platform spec: {error}"),
                }
            }

            Some(msg) = remove_sub.next() => {
                if let Ok(id) = std::str::from_utf8(&msg.payload).unwrap_or("").parse::<Uuid>()
                    && radars.remove(&id).is_some()
                {
                    println!("- platform {}", &id.to_string()[..8]);
                }
            }

            Some(_) = reset_sub.next() => {
                println!("reset — restoring Kyiv preset");
                radars = preset_radars();
            }
        }
    }
}

/// Builds the radars for the default Kyiv deployment.
fn preset_radars() -> HashMap<Uuid, Radar> {
    kyiv_preset()
        .into_iter()
        .map(|spec| (spec.id, Radar::new(spec, CLASSIFICATION_RANGE)))
        .collect()
}

/// Default Kyiv deployment: long-range ring + short-range in-city point defence.
fn kyiv_preset() -> Vec<PlatformSpec> {
    let ring = [
        ("hostomel", -18600.0, 14800.0),
        ("brovary", 18900.0, 6800.0),
        ("vasylkiv", -15100.0, -30400.0),
        ("boryspil", 30200.0, -11700.0),
        ("vyshhorod", 1000.0, 19000.0),
        ("obukhiv", 6000.0, -28000.0),
    ];
    let city = [
        ("maidan", 0.0, 0.0),
        ("livoberezhna", 6000.0, -1500.0),
        ("sviatoshyn", -9000.0, 2000.0),
    ];

    ring.iter()
        .map(|&(name, x, y)| spec(name, x, y, 20_000.0, 6))
        .chain(
            city.iter()
                .map(|&(name, x, y)| spec(name, x, y, 7_000.0, 4)),
        )
        .collect()
}

fn spec(name: &str, x: f64, y: f64, reach: f64, ammo: usize) -> PlatformSpec {
    PlatformSpec {
        id: Uuid::new_v4(),
        name: name.to_string(),
        position: Position { x, y },
        reach,
        ammo,
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before the unix epoch")
        .as_millis() as u64
}
