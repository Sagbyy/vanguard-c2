//! Engagement layer: assigns platforms to confirmed real threats (Hungarian),
//! then flies a real interceptor to a predicted intercept point until impact.

use std::collections::{HashMap, HashSet};

use pathfinding::kuhn_munkres::kuhn_munkres;
use pathfinding::matrix::Matrix;
use uuid::Uuid;
use vanguard_core::{Engagement, FlyingInterceptor, Position, Radar, Speed, Threat, predicted_intercept};

/// Interceptor cruise speed (m/s) — fast enough to run down drones and missiles.
const INTERCEPTOR_SPEED: f64 = 800.0;
/// Detonation radius: within this of the target (or reachable this tick) = kill.
const HIT_RADIUS: f64 = 400.0;
/// Score floor: any reachable target outranks "don't fire" (dummy = 0).
const REACHABLE_BASE: i64 = 100_000;
const UNREACHABLE: i64 = -1_000_000;

struct Shot {
    id: Uuid,
    target: Uuid,
    position: Position,
}

struct Engager {
    ammo: usize,
    shot: Option<Shot>,
}

#[derive(Default)]
pub struct Engagements {
    engagers: HashMap<Uuid, Engager>,
    last_pos: HashMap<Uuid, Position>,
    pub neutralized: usize,
}

impl Engagements {
    pub fn reset(&mut self) {
        self.engagers.clear();
        self.last_pos.clear();
        self.neutralized = 0;
    }

    /// Keep one engager per live platform, seeding ammo from its spec.
    pub fn sync(&mut self, radars: &HashMap<Uuid, Radar>) {
        self.engagers.retain(|id, _| radars.contains_key(id));
        for (id, radar) in radars {
            self.engagers.entry(*id).or_insert(Engager {
                ammo: radar.spec().ammo,
                shot: None,
            });
        }
    }

    /// One simulation step: estimate threat velocities, fly interceptors toward
    /// their predicted intercept point, resolve impacts, then assign new shots.
    /// Returns the threat ids neutralised this tick.
    pub fn step(
        &mut self,
        radars: &HashMap<Uuid, Radar>,
        threats: &[Threat],
        engageable: &HashSet<Uuid>,
        dt: f64,
    ) -> Vec<Uuid> {
        self.sync(radars);

        // Estimate each threat's velocity vector from its last sighting.
        let mut vel: HashMap<Uuid, Speed> = HashMap::new();
        if dt > 0.0 {
            for t in threats {
                if let Some(prev) = self.last_pos.get(&t.id) {
                    vel.insert(
                        t.id,
                        Speed {
                            x: (t.position.x - prev.x) / dt,
                            y: (t.position.y - prev.y) / dt,
                        },
                    );
                }
            }
        }
        self.last_pos = threats.iter().map(|t| (t.id, t.position.clone())).collect();

        let by_id: HashMap<Uuid, &Threat> = threats.iter().map(|t| (t.id, t)).collect();
        let step = INTERCEPTOR_SPEED * dt;

        // Fly in-flight interceptors toward their predicted intercept point.
        let mut destroyed = Vec::new();
        for eng in self.engagers.values_mut() {
            let Some(shot) = &mut eng.shot else { continue };
            let Some(threat) = by_id.get(&shot.target) else {
                eng.shot = None; // target gone (leaked / already killed)
                continue;
            };
            let v = vel.get(&shot.target).cloned().unwrap_or(Speed { x: 0.0, y: 0.0 });

            if shot.position.distance(&threat.position) <= step + HIT_RADIUS {
                destroyed.push(shot.target); // impact this tick
                eng.shot = None;
                continue;
            }
            let aim = predicted_intercept(&shot.position, INTERCEPTOR_SPEED, &threat.position, &v)
                .unwrap_or_else(|| threat.position.clone()); // fallback: pure pursuit
            shot.position = shot.position.step_toward(&aim, step);
        }
        self.neutralized += destroyed.len();

        self.assign(radars, threats, engageable);
        destroyed
    }

    /// Hungarian assignment: globally optimal platform → target matching, then
    /// launch an interceptor from each newly-assigned platform.
    fn assign(&mut self, radars: &HashMap<Uuid, Radar>, threats: &[Threat], engageable: &HashSet<Uuid>) {
        let targeted: HashSet<Uuid> = self
            .engagers
            .values()
            .filter_map(|e| e.shot.as_ref().map(|s| s.target))
            .collect();

        let free: Vec<Uuid> = self
            .engagers
            .iter()
            .filter(|(_, e)| e.shot.is_none() && e.ammo > 0)
            .map(|(id, _)| *id)
            .collect();
        let targets: Vec<&Threat> = threats
            .iter()
            .filter(|t| engageable.contains(&t.id) && !targeted.contains(&t.id))
            .collect();
        if free.is_empty() || targets.is_empty() {
            return;
        }

        let n = free.len().max(targets.len());
        let rows: Vec<Vec<i64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| self.score(radars, free.get(i), targets.get(j).copied()))
                    .collect()
            })
            .collect();
        let (_, assignment) = kuhn_munkres(&Matrix::from_rows(rows).expect("square matrix"));

        for (i, &j) in assignment.iter().enumerate() {
            let (Some(&pid), Some(&t)) = (free.get(i), targets.get(j)) else {
                continue;
            };
            let Some(radar) = radars.get(&pid) else { continue };
            if radar.spec().position.distance(&t.position) > radar.spec().reach {
                continue; // dummy / out-of-range match
            }
            if let Some(eng) = self.engagers.get_mut(&pid) {
                eng.shot = Some(Shot {
                    id: Uuid::new_v4(),
                    target: t.id,
                    position: radar.spec().position.clone(),
                });
                eng.ammo -= 1;
            }
        }
    }

    /// Engagement value of platform `pid` firing on `threat` (dummy cells = 0).
    fn score(&self, radars: &HashMap<Uuid, Radar>, pid: Option<&Uuid>, threat: Option<&Threat>) -> i64 {
        let (Some(pid), Some(threat)) = (pid, threat) else {
            return 0;
        };
        let Some(radar) = radars.get(pid) else { return UNREACHABLE };
        let spec = radar.spec();
        let d = spec.position.distance(&threat.position);
        if d > spec.reach {
            return UNREACHABLE;
        }
        REACHABLE_BASE + (threat.threat_level as i64) * 1000 - (d as i64) / 10
    }

    pub fn ammo(&self, platform_id: &Uuid) -> usize {
        self.engagers.get(platform_id).map_or(0, |e| e.ammo)
    }

    /// Firing lines (platform → its target) for the operator view.
    pub fn lines(&self) -> Vec<Engagement> {
        self.engagers
            .iter()
            .filter_map(|(pid, e)| {
                e.shot.as_ref().map(|s| Engagement {
                    platform_id: *pid,
                    threat_id: s.target,
                })
            })
            .collect()
    }

    /// Interceptors currently in flight, for the dashboard to animate.
    pub fn interceptors(&self) -> Vec<FlyingInterceptor> {
        self.engagers
            .values()
            .filter_map(|e| {
                e.shot.as_ref().map(|s| FlyingInterceptor {
                    id: s.id,
                    position: s.position.clone(),
                    target_id: s.target,
                })
            })
            .collect()
    }
}
