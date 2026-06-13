mod cli;

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use futures::StreamExt;
use uuid::Uuid;
use vanguard_core::{
    DetectedThreat, Interceptor, InterceptorReport, InterceptorState, PlatformInterceptor,
    Position, Speed, THREATS_SUBJECT, Threat, ThreatClassification, report_subject,
};

use crate::cli::Args;

// Range at which the sensor can tell a real drone from a decoy (optical/RF
// discrimination only works up close). Override with CLASSIFICATION_RANGE_M.
const CLASSIFICATION_RANGE: f64 = 8_000.0;
// Above this speed a contact is classified as a cruise-missile-class fast mover.
const MISSILE_SPEED: f64 = 300.0;
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
            assigned_track: None,
        })
        .collect();

    let platform = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: args.name,
        position,
        interceptors,
        reach: args.reach,
        neighbor_platforms: Vec::new(),
    };

    println!(
        "{} (id {}) online at ({:.0}, {:.0}) — radar range {:.0} m, {} interceptor(s) ready",
        platform.name,
        platform.id,
        platform.position.x,
        platform.position.y,
        platform.reach,
        platform.interceptors.len(),
    );

    let classification_range = std::env::var("CLASSIFICATION_RANGE_M")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CLASSIFICATION_RANGE);

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    let mut threat_updates = client.subscribe(THREATS_SUBJECT).await?;
    let reports_subject = report_subject(&platform.id);
    println!(
        "{} radar active — listening on `{THREATS_SUBJECT}`, reporting on `{reports_subject}`",
        platform.name,
    );

    let mut acquired: HashSet<Uuid> = HashSet::new();
    // Last sighting per threat, used to estimate its speed vector (Δpos / Δt),
    // exactly like a real tracking radar: the platform never reads the
    // ground-truth speed.
    let mut last_seen: HashMap<Uuid, (Position, Instant)> = HashMap::new();

    while let Some(message) = threat_updates.next().await {
        let threats: Vec<Threat> = match serde_json::from_slice(&message.payload) {
            Ok(threats) => threats,
            Err(error) => {
                eprintln!(
                    "{} discarding invalid threat update: {error}",
                    platform.name
                );
                continue;
            }
        };

        let now = Instant::now();
        let now_ms = unix_timestamp_ms();
        let mut contacts: Vec<DetectedThreat> = Vec::new();
        let mut in_range = Vec::new();

        for threat in &threats {
            let range = platform.position.distance(&threat.position);
            if range > platform.reach {
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

            let speed = match last_seen.get(&threat.id) {
                Some((previous, at)) => {
                    let dt = now.duration_since(*at).as_secs_f64();
                    Speed {
                        x: (threat.position.x - previous.x) / dt,
                        y: (threat.position.y - previous.y) / dt,
                    }
                }
                None => Speed { x: 0.0, y: 0.0 },
            };
            last_seen.insert(threat.id, (threat.position.clone(), now));

            // Real vs decoy can only be told once the contact is close enough.
            // Beyond the classification range it stays an Unknown blip.
            let (classification, confidence) = if range <= classification_range {
                let class = if threat.is_decoy {
                    ThreatClassification::Decoy
                } else if threat.speed >= MISSILE_SPEED {
                    ThreatClassification::CruiseMissile
                } else {
                    ThreatClassification::Drone
                };
                (class, 0.95)
            } else {
                (ThreatClassification::Unknown, 0.3)
            };
            contacts.push(DetectedThreat {
                id: threat.id,
                position: threat.position.clone(),
                speed,
                threat_level: threat.threat_level,
                classification,
                confidence,
                detected_at: now_ms as f64 / 1000.0,
            });
            in_range.push(format!("{} at {:.0} m", short(&threat.id), range));
        }

        if in_range.is_empty() {
            println!("{} radar: no contact", platform.name);
        } else {
            println!("{} radar: {}", platform.name, in_range.join(", "));
        }

        let report = InterceptorReport {
            platform_id: platform.id,
            name: platform.name.clone(),
            position: platform.position.clone(),
            reach: platform.reach,
            threats: contacts,
            interceptors_remaining: platform
                .interceptors
                .iter()
                .filter(|i| matches!(i.state, InterceptorState::Idle))
                .count(),
            timestamp: unix_timestamp_ms(),
        };

        match serde_json::to_vec(&report) {
            Ok(payload) => {
                if let Err(error) = client
                    .publish(reports_subject.clone(), payload.into())
                    .await
                {
                    eprintln!("{} failed to publish report: {error}", platform.name);
                }
            }
            Err(error) => eprintln!("{} failed to serialize report: {error}", platform.name),
        }
    }

    Ok(())
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is set before the unix epoch")
        .as_millis() as u64
}
