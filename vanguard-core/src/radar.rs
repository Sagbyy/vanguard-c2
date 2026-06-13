use std::collections::HashMap;
use std::time::Instant;

use uuid::Uuid;

use crate::control::PlatformSpec;
use crate::interceptor::{DetectedThreat, InterceptorReport, ThreatClassification};
use crate::position::{Position, Speed};
use crate::threat::Threat;

/// One platform's sensor: detects threats within `reach` and estimates their
/// velocity from successive sightings. It does NOT tell a real drone from a
/// decoy — that recognition is done by the interceptor's seeker in terminal
/// flight (see `vanguard-control`). Every contact is reported as `Unknown`.
pub struct Radar {
    spec: PlatformSpec,
    last_seen: HashMap<Uuid, (Position, Instant)>,
}

impl Radar {
    pub fn new(spec: PlatformSpec) -> Self {
        Self { spec, last_seen: HashMap::new() }
    }

    pub fn spec(&self) -> &PlatformSpec {
        &self.spec
    }

    /// Builds this platform's radar report from the ground-truth threats.
    pub fn observe(&mut self, threats: &[Threat], now_ms: u64) -> InterceptorReport {
        let now = Instant::now();
        let mut contacts = Vec::new();

        for threat in threats {
            let range = self.spec.position.distance(&threat.position);
            if range > self.spec.reach {
                continue;
            }

            let speed = match self.last_seen.get(&threat.id) {
                Some((previous, at)) => {
                    let dt = now.duration_since(*at).as_secs_f64().max(1e-3);
                    Speed {
                        x: (threat.position.x - previous.x) / dt,
                        y: (threat.position.y - previous.y) / dt,
                    }
                }
                None => Speed { x: 0.0, y: 0.0 },
            };
            self.last_seen
                .insert(threat.id, (threat.position.clone(), now));

            contacts.push(DetectedThreat {
                id: threat.id,
                position: threat.position.clone(),
                speed,
                threat_level: threat.threat_level,
                classification: ThreatClassification::Unknown,
                confidence: 0.3,
                detected_at: now_ms as f64 / 1000.0,
            });
        }

        // Forget tracks no longer present so the map doesn't grow unbounded.
        let alive: std::collections::HashSet<Uuid> = threats.iter().map(|t| t.id).collect();
        self.last_seen.retain(|id, _| alive.contains(id));

        InterceptorReport {
            platform_id: self.spec.id,
            name: self.spec.name.clone(),
            position: self.spec.position.clone(),
            reach: self.spec.reach,
            threats: contacts,
            interceptors_remaining: self.spec.ammo,
            timestamp: now_ms,
        }
    }
}
