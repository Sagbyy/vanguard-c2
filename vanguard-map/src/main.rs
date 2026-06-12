use std::f64::consts::TAU;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use uuid::Uuid;
use vanguard_core::{Position, THREATS_SUBJECT, Threat};

const WORLD_RADIUS: f64 = 5_000.0;
const TICK: std::time::Duration = std::time::Duration::from_millis(500);
const SPAWN_EVERY_TICKS: u64 = 6; // one threat every 3 s
const PUBLISH_EVERY_TICKS: u64 = 2; // ground truth published every second
const THREAT_SPEED: std::ops::Range<f64> = 30.0..80.0;
const THREAT_LEVEL: std::ops::Range<usize> = 1..6;
const IMPACT_RADIUS: f64 = 10.0;
const SEED: u64 = 42;
const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";

const CENTER: Position = Position { x: 0.0, y: 0.0 };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    println!("map online — publishing threats on `{THREATS_SUBJECT}` via {nats_url}");

    let mut rng = StdRng::seed_from_u64(SEED);
    let mut threats: Vec<Threat> = Vec::new();
    let mut ticker = tokio::time::interval(TICK);

    let dt = TICK.as_secs_f64();
    for tick in 0u64.. {
        ticker.tick().await;
        let t = tick as f64 * dt;

        if tick % SPAWN_EVERY_TICKS == 0 {
            let threat = spawn_threat(&mut rng);
            println!(
                "[{t:6.1}s] threat {} spawned at ({:.0}, {:.0}) — {:.0} m/s, level {}",
                short(&threat.id),
                threat.position.x,
                threat.position.y,
                threat.speed,
                threat.threat_level,
            );
            threats.push(threat);
        }

        for threat in &mut threats {
            threat.position = threat.position.step_toward(&CENTER, threat.speed * dt);
        }

        threats.retain(|threat| {
            let reached = threat.position.distance(&CENTER) < IMPACT_RADIUS;
            if reached {
                println!(
                    "[{t:6.1}s] threat {} reached defended point — LEAKER",
                    short(&threat.id),
                );
            }
            !reached
        });

        if tick % PUBLISH_EVERY_TICKS == 0 {
            for threat in &threats {
                println!(
                    "[{t:6.1}s] threat {} at ({:.0}, {:.0})",
                    short(&threat.id),
                    threat.position.x,
                    threat.position.y,
                );
            }
            let payload = serde_json::to_vec(&threats)?;
            client.publish(THREATS_SUBJECT, payload.into()).await?;
        }
    }

    Ok(())
}

fn spawn_threat(rng: &mut StdRng) -> Threat {
    let angle = rng.gen_range(0.0..TAU);
    Threat {
        id: Uuid::new_v4(),
        position: Position {
            x: WORLD_RADIUS * angle.cos(),
            y: WORLD_RADIUS * angle.sin(),
        },
        speed: rng.gen_range(THREAT_SPEED),
        threat_level: rng.gen_range(THREAT_LEVEL),
    }
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}
