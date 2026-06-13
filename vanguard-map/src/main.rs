use std::f64::consts::TAU;

use futures::StreamExt;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use uuid::Uuid;
use vanguard_core::{
    CONTROL_RESET, MAP_CONFIG, MapConfig, Position, THREAT_DESTROYED, THREATS_SUBJECT, Threat,
};

const WORLD_RADIUS: f64 = 50_000.0; // ingress ring, well outside every radar bubble
const TICK: std::time::Duration = std::time::Duration::from_millis(500);
const PUBLISH_EVERY_TICKS: u64 = 2; // ground truth published every second
const SECTOR_SPREAD_DEG: f64 = 20.0; // wave fans out within ±20° of one bearing
const REAL_SPEED: std::ops::Range<f64> = 100.0..170.0; // attack drones, m/s
const DECOY_SPEED: std::ops::Range<f64> = 70.0..120.0; // decoys, a bit slower
const IMPACT_RADIUS: f64 = 50.0;
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

    let mut config_sub = client.subscribe(MAP_CONFIG).await?;
    let mut reset_sub = client.subscribe(CONTROL_RESET).await?;
    let mut destroyed_sub = client.subscribe(THREAT_DESTROYED).await?;

    let mut rng = StdRng::seed_from_u64(SEED);
    let mut actives: Vec<Active> = Vec::new();
    let mut config = MapConfig::default();
    let mut ticker = tokio::time::interval(TICK);
    let mut last_swarm_t = f64::NEG_INFINITY;

    let dt = TICK.as_secs_f64();
    // Sim time and tick count advance ONLY on ticker ticks — config messages
    // must never move the clock (a slider drag floods config updates).
    let mut t = 0.0f64;
    let mut ticks = 0u64;
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            Some(msg) = config_sub.next() => {
                match serde_json::from_slice::<MapConfig>(&msg.payload) {
                    Ok(new) => {
                        println!("config updated: {new:?}");
                        config = new;
                    }
                    Err(error) => eprintln!("invalid config: {error}"),
                }
                continue;
            }
            Some(_) = reset_sub.next() => {
                println!("reset — clearing threats, default config");
                actives.clear();
                config = MapConfig::default();
                last_swarm_t = f64::NEG_INFINITY;
                continue;
            }
            Some(msg) = destroyed_sub.next() => {
                if let Ok(id) = std::str::from_utf8(&msg.payload).unwrap_or("").parse::<Uuid>() {
                    actives.retain(|a| a.threat.id != id);
                }
                continue;
            }
        }
        t += dt;
        ticks += 1;

        if t - last_swarm_t >= config.spawn_interval_s && actives.len() < config.max_active {
            last_swarm_t = t;
            let swarm = spawn_swarm(&mut rng, &config);
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
            active.threat.position = active
                .threat
                .position
                .step_toward(&active.target, active.threat.speed * dt);
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

        if ticks.is_multiple_of(PUBLISH_EVERY_TICKS) {
            let threats: Vec<&Threat> = actives.iter().map(|a| &a.threat).collect();
            let payload = serde_json::to_vec(&threats)?;
            client.publish(THREATS_SUBJECT, payload.into()).await?;
        }
    }
}

/// One attack wave: real loitering munitions mixed with empty decoys, all
/// ingressing from the same bearing sector (±SECTOR_SPREAD_DEG). Each drone
/// aims at its own random impact point inside the defended zone.
fn spawn_swarm(rng: &mut StdRng, config: &MapConfig) -> Vec<Active> {
    let center_bearing = rng.gen_range(0.0..TAU);
    let spread = SECTOR_SPREAD_DEG.to_radians();
    let max = config.swarm_max.max(config.swarm_min);
    let size = rng.gen_range(config.swarm_min..=max);
    let decoy_ratio = config.decoy_ratio.clamp(0.0, 1.0);

    (0..size)
        .map(|_| {
            let angle = center_bearing + rng.gen_range(-spread..spread);
            let is_decoy = rng.gen_bool(decoy_ratio);
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
                target: random_zone_point(rng, config.zone_radius),
            }
        })
        .collect()
}

/// Uniform random point within the defended zone around the city centre.
fn random_zone_point(rng: &mut StdRng, zone_radius: f64) -> Position {
    let angle = rng.gen_range(0.0..TAU);
    let radius = zone_radius * rng.gen_range(0.0_f64..1.0).sqrt();
    Position {
        x: CENTER.x + radius * angle.cos(),
        y: CENTER.y + radius * angle.sin(),
    }
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}
