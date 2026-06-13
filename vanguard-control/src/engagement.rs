//! Engagement layer. Platforms only *detect* contacts (unknown); the
//! interceptor's seeker *recognises* real-vs-decoy in terminal flight, within
//! `recognition_range`. A real threat is killed. A recognised decoy is dropped
//! from the engageable set and its interceptor is **re-tasked to the nearest
//! remaining target** — only if none is left does it divert to the nearest
//! safe drop zone and self-destruct.
//! Assignment is Hungarian over {in-flight movers + free tubes} × contacts,
//! with hysteresis for dynamic re-tasking.

use std::collections::{HashMap, HashSet};

use pathfinding::kuhn_munkres::kuhn_munkres;
use pathfinding::matrix::Matrix;
use uuid::Uuid;
use vanguard_core::{
    Engagement, FlyingInterceptor, Position, Radar, Speed, Threat, ThreatClassification,
    predicted_intercept,
};

const INTERCEPTOR_SPEED: f64 = 800.0;
const HIT_RADIUS: f64 = 400.0;
const MAX_IN_FLIGHT: usize = 3;
/// Seconds of simulated time a platform takes to resupply one interceptor (up to
/// its initial capacity). Keeps the engagement sustainable for an open-ended demo.
const RELOAD_INTERVAL: f64 = 20.0;
const MISSILE_SPEED: f64 = 300.0;
const REACHABLE_BASE: i64 = 100_000;
const UNREACHABLE: i64 = -1_000_000;
const HYST_BONUS: i64 = 5_000;
const URGENCY_SPAN: f64 = 60_000.0;

/// Designated safe drop zones (empty areas well outside the city). An aborted
/// interceptor self-destructs at the nearest one.
enum Assignment {
    Target { id: Uuid, locked: bool },
    /// Returning to base to self-destruct. `manual` = operator-aborted, so it is
    /// excluded from auto re-tasking; an idle (auto) divert can re-engage a fresh
    /// threat that enters range.
    Divert { to: Position, manual: bool },
}

struct Shot {
    id: Uuid,
    position: Position,
    /// Launching platform position — the reachable safe point a divert flies to
    /// (an interceptor cannot fly beyond its own platform's range).
    home: Position,
    assignment: Assignment,
}

struct Engager {
    ammo: usize,
    capacity: usize,
    reload_accum: f64,
    shots: Vec<Shot>,
}

#[derive(Default)]
pub struct Engagements {
    engagers: HashMap<Uuid, Engager>,
    last_pos: HashMap<Uuid, Position>,
    recognized: HashMap<Uuid, ThreatClassification>,
    pub neutralized: usize,
}

impl Engagements {
    pub fn reset(&mut self) {
        self.engagers.clear();
        self.last_pos.clear();
        self.recognized.clear();
        self.neutralized = 0;
    }

    pub fn sync(&mut self, radars: &HashMap<Uuid, Radar>) {
        self.engagers.retain(|id, _| radars.contains_key(id));
        for (id, radar) in radars {
            self.engagers
                .entry(*id)
                .or_insert(Engager {
                    ammo: radar.spec().ammo,
                    capacity: radar.spec().ammo,
                    reload_accum: 0.0,
                    shots: Vec::new(),
                });
        }
    }

    pub fn retarget(&mut self, iid: Uuid, tid: Uuid) {
        if let Some(shot) = self.shot_mut(iid) {
            shot.assignment = Assignment::Target { id: tid, locked: true };
        }
    }

    pub fn abort(&mut self, iid: Uuid) {
        if let Some(shot) = self.shot_mut(iid) {
            shot.assignment = Assignment::Divert { to: shot.home.clone(), manual: true };
        }
    }

    fn shot_mut(&mut self, iid: Uuid) -> Option<&mut Shot> {
        self.engagers.values_mut().flat_map(|e| e.shots.iter_mut()).find(|s| s.id == iid)
    }

    pub fn recognized(&self) -> &HashMap<Uuid, ThreatClassification> {
        &self.recognized
    }

    pub fn step(
        &mut self,
        radars: &HashMap<Uuid, Radar>,
        threats: &[Threat],
        engageable: &HashSet<Uuid>,
        dt: f64,
        time_scale: f64,
        recognition_range: f64,
    ) -> Vec<Uuid> {
        self.sync(radars);

        // Slow resupply: each platform regains one interceptor every
        // RELOAD_INTERVAL of simulated time, capped at its initial capacity.
        let sdt = dt * time_scale.max(0.0);
        for eng in self.engagers.values_mut() {
            if eng.ammo >= eng.capacity {
                eng.reload_accum = 0.0;
                continue;
            }
            eng.reload_accum += sdt;
            while eng.reload_accum >= RELOAD_INTERVAL && eng.ammo < eng.capacity {
                eng.reload_accum -= RELOAD_INTERVAL;
                eng.ammo += 1;
            }
        }

        let mut vel: HashMap<Uuid, Speed> = HashMap::new();
        if dt > 0.0 {
            for t in threats {
                if let Some(prev) = self.last_pos.get(&t.id) {
                    vel.insert(
                        t.id,
                        Speed { x: (t.position.x - prev.x) / dt, y: (t.position.y - prev.y) / dt },
                    );
                }
            }
        }
        self.last_pos = threats.iter().map(|t| (t.id, t.position.clone())).collect();

        let alive: HashSet<Uuid> = threats.iter().map(|t| t.id).collect();
        for eng in self.engagers.values_mut() {
            for shot in &mut eng.shots {
                if let Assignment::Target { id, .. } = shot.assignment {
                    if !alive.contains(&id) {
                        shot.assignment = Assignment::Divert { to: shot.home.clone(), manual: false };
                    }
                }
            }
        }

        // --- Terminal recognition by the interceptor seeker. A recognised decoy
        // is just recorded (→ excluded from engageable); its interceptor is left
        // as a free mover so the next retask sends it to the nearest target.
        let known: HashSet<Uuid> = self.recognized.keys().copied().collect();
        let by_id: HashMap<Uuid, &Threat> = threats.iter().map(|t| (t.id, t)).collect();
        let mut newly: Vec<(Uuid, ThreatClassification)> = Vec::new();
        for eng in self.engagers.values() {
            for shot in &eng.shots {
                let Assignment::Target { id, .. } = shot.assignment else { continue };
                if known.contains(&id) || newly.iter().any(|(t, _)| *t == id) {
                    continue;
                }
                if let Some(threat) = by_id.get(&id) {
                    if shot.position.distance(&threat.position) <= recognition_range {
                        newly.push((id, classify(threat)));
                    }
                }
            }
        }
        for (id, class) in newly {
            self.recognized.insert(id, class);
        }
        self.recognized.retain(|id, _| alive.contains(id));

        self.retask(radars, threats, engageable);

        // --- Advance + resolve.
        let int_speed = INTERCEPTOR_SPEED * time_scale.max(0.0);
        let step = int_speed * dt;
        let mut destroyed = Vec::new();
        for eng in self.engagers.values_mut() {
            eng.shots.retain_mut(|shot| {
                // Snapshot the assignment so we can then mutate shot.position.
                let target = match &shot.assignment {
                    Assignment::Divert { to, .. } => Err(to.clone()),
                    Assignment::Target { id, .. } => Ok(*id),
                };
                match target {
                    Err(to) => {
                        if shot.position.distance(&to) <= step + HIT_RADIUS {
                            return false;
                        }
                        shot.position = shot.position.step_toward(&to, step);
                        true
                    }
                    Ok(id) => {
                        let Some(threat) = by_id.get(&id) else { return false };
                        if shot.position.distance(&threat.position) <= step + HIT_RADIUS {
                            destroyed.push(id);
                            return false;
                        }
                        let v = vel.get(&id).cloned().unwrap_or(Speed { x: 0.0, y: 0.0 });
                        let aim = predicted_intercept(&shot.position, int_speed, &threat.position, &v)
                            .unwrap_or_else(|| threat.position.clone());
                        shot.position = shot.position.step_toward(&aim, step);
                        true
                    }
                }
            });
        }
        self.neutralized += destroyed.len();
        destroyed
    }

    fn retask(&mut self, radars: &HashMap<Uuid, Radar>, threats: &[Threat], engageable: &HashSet<Uuid>) {
        let locked_targets: HashSet<Uuid> = self
            .engagers
            .values()
            .flat_map(|e| &e.shots)
            .filter_map(|s| match s.assignment {
                Assignment::Target { id, locked: true } => Some(id),
                _ => None,
            })
            .collect();

        // (shot_id, position, home). An auto-diverting interceptor is re-taskable:
        // a fresh threat entering range pulls it back. A manual abort is not.
        let mut movers: Vec<(Uuid, Position, Position)> = Vec::new();
        let mut tubes: Vec<Uuid> = Vec::new();
        for (pid, e) in &self.engagers {
            for s in &e.shots {
                let retaskable = matches!(
                    s.assignment,
                    Assignment::Target { locked: false, .. } | Assignment::Divert { manual: false, .. }
                );
                if retaskable {
                    movers.push((s.id, s.position.clone(), s.home.clone()));
                }
            }
            let capacity = MAX_IN_FLIGHT.saturating_sub(e.shots.len()).min(e.ammo);
            for _ in 0..capacity {
                tubes.push(*pid);
            }
        }

        let mover_target: HashMap<Uuid, Uuid> = self
            .engagers
            .values()
            .flat_map(|e| &e.shots)
            .filter_map(|s| match s.assignment {
                Assignment::Target { id, locked: false } => Some((s.id, id)),
                _ => None,
            })
            .collect();

        let targets: Vec<&Threat> = threats
            .iter()
            .filter(|t| engageable.contains(&t.id) && !locked_targets.contains(&t.id))
            .collect();
        // Fallback for an in-flight interceptor the Hungarian leaves unmatched.
        // Priority: nearest non-decoy threat (real OR not-yet-identified, even if
        // out of radar range) > nearest recognised decoy (last resort) > RTB.
        let is_decoy = |t: &&Threat| matches!(self.recognized.get(&t.id), Some(ThreatClassification::Decoy));
        let nondecoy: Vec<(Uuid, Position)> =
            threats.iter().filter(|t| !is_decoy(t)).map(|t| (t.id, t.position.clone())).collect();
        let decoys: Vec<(Uuid, Position)> =
            threats.iter().filter(is_decoy).map(|t| (t.id, t.position.clone())).collect();
        let nearest = |pool: &[(Uuid, Position)], pos: &Position| {
            pool.iter()
                .min_by(|a, b| pos.distance(&a.1).total_cmp(&pos.distance(&b.1)))
                .map(|(id, _)| *id)
        };
        let fallback = |pos: &Position, home: &Position| {
            nearest(&nondecoy, pos)
                .or_else(|| nearest(&decoys, pos))
                .map(|id| Assignment::Target { id, locked: false })
                .unwrap_or_else(|| Assignment::Divert { to: home.clone(), manual: false })
        };

        if (movers.is_empty() && tubes.is_empty()) || targets.is_empty() {
            // No engageable target: free movers fall back (non-decoy > decoy > RTB).
            for (sid, pos, home) in &movers {
                let assignment = fallback(pos, home);
                if let Some(shot) = self.shot_mut(*sid) {
                    shot.assignment = assignment;
                }
            }
            return;
        }

        let n = (movers.len() + tubes.len()).max(targets.len());
        let rows: Vec<Vec<i64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let Some(threat) = targets.get(j) else { return 0 };
                        if i < movers.len() {
                            let (sid, pos, _) = &movers[i];
                            let keep = mover_target.get(sid) == Some(&threat.id);
                            engage_value(pos, threat) + if keep { HYST_BONUS } else { 0 }
                        } else if i - movers.len() < tubes.len() {
                            self.tube_value(radars, &tubes[i - movers.len()], threat)
                        } else {
                            0
                        }
                    })
                    .collect()
            })
            .collect();
        let (_, assignment) = kuhn_munkres(&Matrix::from_rows(rows).expect("square"));

        for (i, &j) in assignment.iter().enumerate() {
            let threat = targets.get(j).copied();
            if i < movers.len() {
                let (sid, pos, home) = movers[i].clone();
                if let Some(shot) = self.shot_mut(sid) {
                    shot.assignment = match threat {
                        Some(t) => Assignment::Target { id: t.id, locked: false },
                        None => fallback(&pos, &home),
                    };
                }
            } else {
                let ti = i - movers.len();
                let (Some(&pid), Some(t)) = (tubes.get(ti), threat) else { continue };
                let Some(radar) = radars.get(&pid) else { continue };
                if radar.spec().position.distance(&t.position) > radar.spec().reach {
                    continue;
                }
                if let Some(eng) = self.engagers.get_mut(&pid) {
                    if eng.ammo == 0 {
                        continue;
                    }
                    eng.shots.push(Shot {
                        id: Uuid::new_v4(),
                        position: radar.spec().position.clone(),
                        home: radar.spec().position.clone(),
                        assignment: Assignment::Target { id: t.id, locked: false },
                    });
                    eng.ammo -= 1;
                }
            }
        }
    }

    fn tube_value(&self, radars: &HashMap<Uuid, Radar>, pid: &Uuid, threat: &Threat) -> i64 {
        let Some(radar) = radars.get(pid) else { return UNREACHABLE };
        let spec = radar.spec();
        if spec.position.distance(&threat.position) > spec.reach {
            return UNREACHABLE;
        }
        engage_value(&spec.position, threat)
    }

    pub fn ammo(&self, platform_id: &Uuid) -> usize {
        self.engagers.get(platform_id).map_or(0, |e| e.ammo)
    }

    pub fn lines(&self) -> Vec<Engagement> {
        self.engagers
            .iter()
            .flat_map(|(pid, e)| {
                e.shots.iter().filter_map(move |s| match s.assignment {
                    Assignment::Target { id, .. } => Some(Engagement { platform_id: *pid, threat_id: id }),
                    Assignment::Divert { .. } => None,
                })
            })
            .collect()
    }

    pub fn interceptors(&self) -> Vec<FlyingInterceptor> {
        self.engagers
            .values()
            .flat_map(|e| {
                e.shots.iter().map(|s| FlyingInterceptor {
                    id: s.id,
                    position: s.position.clone(),
                    target_id: match s.assignment {
                        Assignment::Target { id, .. } => id,
                        Assignment::Divert { .. } => Uuid::nil(),
                    },
                    diverting: matches!(s.assignment, Assignment::Divert { .. }),
                })
            })
            .collect()
    }
}

fn classify(threat: &Threat) -> ThreatClassification {
    if threat.is_decoy {
        ThreatClassification::Decoy
    } else if threat.speed >= MISSILE_SPEED {
        ThreatClassification::CruiseMissile
    } else {
        ThreatClassification::Drone
    }
}

/// Engagement value of a shooter at `from` against `threat`: urgency (closer to
/// the defended asset = higher) minus the flight distance to reach it.
fn engage_value(from: &Position, threat: &Threat) -> i64 {
    let to_asset = (threat.position.x * threat.position.x + threat.position.y * threat.position.y).sqrt();
    let urgency = (URGENCY_SPAN - to_asset).max(0.0) as i64;
    REACHABLE_BASE + urgency / 10 - (from.distance(&threat.position) as i64) / 10
}
