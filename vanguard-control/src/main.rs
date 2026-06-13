//! Platform host: runs every interceptor platform as a radar driven by the
//! ground-truth threat feed, and lets the UI add/remove platforms live.

mod engagement;

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use uuid::Uuid;
use vanguard_core::{
    CONTROL_RESET, ENGAGEMENTS, EngagementReport, INTERCEPTOR_ABORT, INTERCEPTOR_RETARGET,
    INTERCEPTORS, MAP_CONFIG, MapConfig, PLATFORM_ADD, PLATFORM_REMOVE, PlatformSpec, Position,
    Radar, RetargetCommand, THREAT_DESTROYED, THREATS_SUBJECT, Threat, ThreatClassification,
    ThreatDestroyed, report_subject,
};

use crate::engagement::Engagements;

const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";
/// Distance at which an interceptor's seeker recognises real-vs-decoy (terminal).
const RECOGNITION_RANGE: f64 = 4_000.0;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let recognition_range = std::env::var("RECOGNITION_RANGE_M")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(RECOGNITION_RANGE);
    let client = async_nats::connect(&nats_url).await?;
    let mut threats_sub = client.subscribe(THREATS_SUBJECT).await?;
    let mut add_sub = client.subscribe(PLATFORM_ADD).await?;
    let mut remove_sub = client.subscribe(PLATFORM_REMOVE).await?;
    let mut reset_sub = client.subscribe(CONTROL_RESET).await?;
    let mut config_sub = client.subscribe(MAP_CONFIG).await?;
    let mut retarget_sub = client.subscribe(INTERCEPTOR_RETARGET).await?;
    let mut abort_sub = client.subscribe(INTERCEPTOR_ABORT).await?;
    println!("control host online via {nats_url}");

    let mut radars = preset_radars();
    let mut engagements = Engagements::default();
    let mut last_ms = 0u64;
    let mut time_scale = 1.0_f64;

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
                let dt = if last_ms == 0 {
                    0.0
                } else {
                    (((now_ms - last_ms) as f64) / 1000.0).clamp(0.0, 2.0)
                };
                last_ms = now_ms;
                engagements.sync(&radars);

                // Radar reports: platforms only DETECT (contacts stay Unknown).
                // We stamp the seeker's recognition (interceptor-side) on each
                // contact so the dashboard colours real/decoy, and build the
                // engageable set = every detected contact except confirmed decoys.
                let recognized = engagements.recognized().clone();
                let mut engageable: HashSet<Uuid> = HashSet::new();
                for (id, radar) in radars.iter_mut() {
                    let mut report = radar.observe(&threats, now_ms);
                    report.interceptors_remaining = engagements.ammo(id);
                    for contact in &mut report.threats {
                        if let Some(class) = recognized.get(&contact.id) {
                            contact.classification = class.clone();
                        }
                        let is_decoy = matches!(recognized.get(&contact.id), Some(ThreatClassification::Decoy));
                        if !is_decoy {
                            engageable.insert(contact.id);
                        }
                    }
                    if let Ok(payload) = serde_json::to_vec(&report) {
                        let _ = client.publish(report_subject(id), payload.into()).await;
                    }
                }

                // Fly interceptors, recognise, resolve impacts, assign new shots.
                let by_id: HashMap<Uuid, &Threat> = threats.iter().map(|t| (t.id, t)).collect();
                for tid in engagements.step(&radars, &threats, &engageable, dt, time_scale, recognition_range) {
                    println!("NEUTRALIZED {} (total {})", &tid.to_string()[..8], engagements.neutralized);
                    let position = by_id.get(&tid).map(|t| t.position.clone()).unwrap_or(Position { x: 0.0, y: 0.0 });
                    let event = ThreatDestroyed { id: tid, position };
                    if let Ok(payload) = serde_json::to_vec(&event) {
                        let _ = client.publish(THREAT_DESTROYED, payload.into()).await;
                    }
                }

                // Publish the firing picture + in-flight interceptors.
                // Diverting interceptors return to base (within their platform's
                // range), so the safe areas are the platform positions themselves.
                let report = EngagementReport {
                    lines: engagements.lines(),
                    neutralized: engagements.neutralized,
                    safe_zones: radars.values().map(|r| r.spec().position.clone()).collect(),
                };
                if let Ok(payload) = serde_json::to_vec(&report) {
                    let _ = client.publish(ENGAGEMENTS, payload.into()).await;
                }
                if let Ok(payload) = serde_json::to_vec(&engagements.interceptors()) {
                    let _ = client.publish(INTERCEPTORS, payload.into()).await;
                }
            }

            Some(msg) = add_sub.next() => {
                match serde_json::from_slice::<PlatformSpec>(&msg.payload) {
                    Ok(spec) => {
                        println!("+ platform {} at ({:.0}, {:.0}) reach {:.0}", spec.name, spec.position.x, spec.position.y, spec.reach);
                        radars.insert(spec.id, Radar::new(spec));
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

            Some(msg) = config_sub.next() => {
                if let Ok(cfg) = serde_json::from_slice::<MapConfig>(&msg.payload) {
                    time_scale = cfg.time_scale.max(0.0);
                }
            }

            Some(msg) = retarget_sub.next() => {
                if let Ok(cmd) = serde_json::from_slice::<RetargetCommand>(&msg.payload) {
                    engagements.retarget(cmd.interceptor_id, cmd.target_id);
                }
            }

            Some(msg) = abort_sub.next() => {
                if let Ok(id) = std::str::from_utf8(&msg.payload).unwrap_or("").parse::<Uuid>() {
                    engagements.abort(id);
                }
            }

            Some(_) = reset_sub.next() => {
                println!("reset — restoring Kyiv preset");
                radars = preset_radars();
                engagements.reset();
                time_scale = 1.0;
            }
        }
    }
}

/// Builds the radars for the default Kyiv deployment.
fn preset_radars() -> HashMap<Uuid, Radar> {
    kyiv_preset()
        .into_iter()
        .map(|spec| (spec.id, Radar::new(spec)))
        .collect()
}

/// Default Kyiv deployment: long-range ring around the city (all 20 km reach).
fn kyiv_preset() -> Vec<PlatformSpec> {
    let ring = [
        ("hostomel", -18600.0, 14800.0),
        ("brovary", 18900.0, 6800.0),
        ("vasylkiv", -15100.0, -30400.0),
        ("boryspil", 30200.0, -11700.0),
        ("vyshhorod", 1000.0, 19000.0),
        ("obukhiv", 6000.0, -28000.0),
        ("sviatoshyn", -9000.0, 2000.0),
    ];

    ring.iter()
        .map(|&(name, x, y)| spec(name, x, y, 20_000.0, 6))
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
