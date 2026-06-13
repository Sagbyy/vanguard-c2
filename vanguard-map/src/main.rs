use std::f64::consts::TAU;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use uuid::Uuid;
use vanguard_core::{Position, THREATS_SUBJECT, Threat};

const WORLD_RADIUS: f64 = 50_000.0; // ingress ring, well outside every radar bubble
const TICK: std::time::Duration = std::time::Duration::from_millis(500);
const SWARM_EVERY_TICKS: u64 = 90; // one swarm wave every ~45 s
const PUBLISH_EVERY_TICKS: u64 = 2; // ground truth published every second
const MAX_ACTIVE_THREATS: usize = 40; // saturation cap, keeps the raid readable
const SWARM_SIZE: std::ops::Range<usize> = 6..13; // drones per wave
const DECOY_RATIO: f64 = 0.6; // share of the wave that are empty decoys
const SECTOR_SPREAD_DEG: f64 = 20.0; // wave fans out within ±20° of one bearing
const REAL_SPEED: std::ops::Range<f64> = 100.0..170.0; // attack drones, m/s
const DECOY_SPEED: std::ops::Range<f64> = 70.0..120.0; // decoys, a bit slower
const IMPACT_RADIUS: f64 = 50.0;
const DEFENDED_ZONE_RADIUS: f64 = 6_000.0; // threats aim at random points across the city
const SEED: u64 = 42;
const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";

const CENTER: Position = Position { x: 0.0, y: 0.0 };

/// A live threat with its own impact point inside the defended zone.
struct Active {
    threat: Threat,
    target: Position,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    println!("map online — publishing threats on `{THREATS_SUBJECT}` via {nats_url}");

    let mut rng = StdRng::seed_from_u64(SEED);
    let mut actives: Vec<Active> = Vec::new();
    let mut ticker = tokio::time::interval(TICK);

    let dt = TICK.as_secs_f64();
    for tick in 0u64.. {
        ticker.tick().await;
        let t = tick as f64 * dt;

        if tick % SWARM_EVERY_TICKS == 0 && actives.len() < MAX_ACTIVE_THREATS {
            let swarm = spawn_swarm(&mut rng);
            let decoys = swarm.iter().filter(|a| a.threat.is_decoy).count();
            let bearing = swarm[0]
                .threat
                .position
                .y
                .atan2(swarm[0].threat.position.x)
                .to_degrees();
            println!(
                "[{t:6.1}s] SWARM inbound — {} drones ({} decoys) bearing {:.0}°",
                swarm.len(),
                decoys,
                (bearing + 360.0) % 360.0,
            );
            actives.extend(swarm);
        }

        for active in &mut actives {
            active.threat.position =
                active.threat.position.step_toward(&active.target, active.threat.speed * dt);
        }

        actives.retain(|active| {
            let reached = active.threat.position.distance(&active.target) < IMPACT_RADIUS;
            if reached {
                println!(
                    "[{t:6.1}s] threat {} reached impact point — LEAKER",
                    short(&active.threat.id),
                );
            }
            !reached
        });

        if tick % PUBLISH_EVERY_TICKS == 0 {
            let threats: Vec<&Threat> = actives.iter().map(|a| &a.threat).collect();
            let payload = serde_json::to_vec(&threats)?;
            client.publish(THREATS_SUBJECT, payload.into()).await?;
        }
    }

    Ok(())
}

/// One attack wave: real loitering munitions mixed with empty decoys, all
/// ingressing from the same bearing sector (±SECTOR_SPREAD_DEG). Each drone
/// aims at its own random impact point inside the defended zone.
fn spawn_swarm(rng: &mut StdRng) -> Vec<Active> {
    let center_bearing = rng.gen_range(0.0..TAU);
    let spread = SECTOR_SPREAD_DEG.to_radians();
    let size = rng.gen_range(SWARM_SIZE);

    (0..size)
        .map(|_| {
            let angle = center_bearing + rng.gen_range(-spread..spread);
            let is_decoy = rng.gen_bool(DECOY_RATIO);
            let (speed, threat_level) = if is_decoy {
                (rng.gen_range(DECOY_SPEED), 1)
            } else {
                (rng.gen_range(REAL_SPEED), rng.gen_range(3..6))
            };

            Active {
                threat: Threat {
                    id: Uuid::new_v4(),
                    position: Position {
                        x: WORLD_RADIUS * angle.cos(),
                        y: WORLD_RADIUS * angle.sin(),
                    },
                    speed,
                    threat_level,
                    is_decoy,
                },
                target: random_zone_point(rng),
            }
        })
        .collect()
}

/// Uniform random point within the defended zone around the city centre.
fn random_zone_point(rng: &mut StdRng) -> Position {
    let angle = rng.gen_range(0.0..TAU);
    let radius = DEFENDED_ZONE_RADIUS * rng.gen_range(0.0_f64..1.0).sqrt();
    Position {
        x: CENTER.x + radius * angle.cos(),
        y: CENTER.y + radius * angle.sin(),
    }
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}
