//! Distributed multi-platform track fusion with a Kalman filter.
//!
//! Each tick: predict every existing track forward, then for every threat fold
//! in the platforms' measurements. Several platforms seeing the same target are
//! combined by **averaging their simultaneous measurements** (independent noise
//! → error shrinks by √N), then a single Kalman `update` smooths the track over
//! time. The fused (denoised) position replaces the ground-truth position the
//! engagement reasons about; the threat's identity, level and real/decoy nature
//! still come from the simulation (the latter is only revealed to the
//! interceptor's seeker in the terminal phase, handled in `engagement.rs`).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;
use vanguard_core::{InterceptorReport, KalmanTrack, Position, Threat};

#[derive(Default)]
pub struct TrackFuser {
    tracks: HashMap<Uuid, KalmanTrack>,
}

impl TrackFuser {
    pub fn reset(&mut self) {
        self.tracks.clear();
    }

    /// Fuse the platforms' noisy reports into one track per threat and return
    /// the ground-truth threats with their position replaced by the fused
    /// estimate. `sdt` is simulated dt (real dt × time_scale), matching how far
    /// the threats actually moved this tick.
    pub fn fuse(&mut self, truth: &[Threat], reports: &[InterceptorReport], sdt: f64) -> Vec<Threat> {
        if sdt > 0.0 {
            for track in self.tracks.values_mut() {
                track.predict(sdt);
            }
        }

        // Average all platforms' simultaneous measurements of each threat, then
        // run one Kalman update per track with that fused measurement.
        let mut measured: HashMap<Uuid, (f64, f64, u32)> = HashMap::new();
        for report in reports {
            for contact in &report.threats {
                let acc = measured.entry(contact.id).or_insert((0.0, 0.0, 0));
                acc.0 += contact.position.x;
                acc.1 += contact.position.y;
                acc.2 += 1;
            }
        }
        for (id, (sx, sy, n)) in measured {
            let (mx, my) = (sx / n as f64, sy / n as f64);
            self.tracks
                .entry(id)
                .and_modify(|t| t.update(mx, my))
                .or_insert_with(|| KalmanTrack::new(mx, my, 0.0, 0.0));
        }

        let alive: HashSet<Uuid> = truth.iter().map(|t| t.id).collect();
        self.tracks.retain(|id, _| alive.contains(id));

        truth
            .iter()
            .map(|t| match self.tracks.get(&t.id) {
                Some(track) => {
                    let (x, y) = track.position();
                    Threat { position: Position { x, y }, ..t.clone() }
                }
                // Never measured (out of every radar's range): keep ground truth.
                None => t.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vanguard_core::{DetectedThreat, Speed, ThreatClassification};

    fn noise(seed: u64) -> f64 {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z as f64 / u64::MAX as f64) * 2.0 - 1.0) * 50.0
    }

    fn report(pid: Uuid, tid: Uuid, x: f64, y: f64) -> InterceptorReport {
        InterceptorReport {
            platform_id: pid,
            name: String::new(),
            position: Position { x: 0.0, y: 0.0 },
            reach: 1e9,
            threats: vec![DetectedThreat {
                id: tid,
                position: Position { x, y },
                speed: Speed { x: 0.0, y: 0.0 },
                threat_level: 3,
                classification: ThreatClassification::Unknown,
                confidence: 0.3,
                detected_at: 0.0,
            }],
            interceptors_remaining: 0,
            timestamp: 0,
        }
    }

    /// Three platforms measure the same moving target with independent noise.
    /// The fused track must, on average, be closer to the truth than a single
    /// raw measurement.
    #[test]
    fn fusion_reduces_position_error() {
        let tid = Uuid::from_u128(1);
        let plats = [Uuid::from_u128(10), Uuid::from_u128(11), Uuid::from_u128(12)];
        let mut fuser = TrackFuser::default();
        let (dt, speed) = (0.25, 120.0);

        let (mut raw_err, mut raw_n) = (0.0, 0.0);
        let (mut fused_err, mut fused_n) = (0.0, 0.0);

        for step in 0..40u64 {
            let (tx, ty) = (speed * dt * step as f64, 0.0);
            let truth = vec![Threat {
                id: tid,
                position: Position { x: tx, y: ty },
                speed,
                threat_level: 3,
                is_decoy: false,
            }];
            let reports: Vec<_> = plats
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let s = step.wrapping_mul(97).wrapping_add(i as u64 * 7919);
                    report(p, tid, tx + noise(s), ty + noise(s ^ 0xABCD))
                })
                .collect();

            for r in &reports {
                let c = &r.threats[0];
                raw_err += (c.position.x - tx).hypot(c.position.y - ty);
                raw_n += 1.0;
            }

            let fused = fuser.fuse(&truth, &reports, dt);
            if step >= 5 {
                let f = &fused[0].position;
                fused_err += (f.x - tx).hypot(f.y - ty);
                fused_n += 1.0;
            }
        }

        let raw_mean = raw_err / raw_n;
        let fused_mean = fused_err / fused_n;
        assert!(fused_mean < raw_mean, "fused {fused_mean:.2} m should beat raw {raw_mean:.2} m");
    }
}
