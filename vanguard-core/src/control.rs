use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::position::Position;

/// Live-tunable swarm/simulation parameters, published by the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapConfig {
    pub decoy_ratio: f64,
    pub swarm_min: usize,
    pub swarm_max: usize,
    pub spawn_interval_s: f64,
    pub zone_radius: f64,
    pub max_active: usize,
    /// Simulation speed multiplier (1.0 = real time). Accelerates threats,
    /// spawns and interceptors together so the engagement stays consistent.
    pub time_scale: f64,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            decoy_ratio: 0.4,
            swarm_min: 6,
            swarm_max: 12,
            spawn_interval_s: 45.0,
            zone_radius: 6_000.0,
            max_active: 40,
            time_scale: 1.0,
        }
    }
}

/// A platform the control host should run, defined from the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformSpec {
    pub id: Uuid,
    pub name: String,
    pub position: Position,
    pub reach: f64,
    pub ammo: usize,
}

/// UI → map: update simulation parameters.
pub const MAP_CONFIG: &str = "control.map.config";
/// UI → control host: add a platform.
pub const PLATFORM_ADD: &str = "control.platform.add";
/// UI → control host: remove a platform (payload = platform id string).
pub const PLATFORM_REMOVE: &str = "control.platform.remove";
/// UI → map + host: reset to the baseline scenario (default config, preset
/// platforms, cleared threats). Payload is ignored.
pub const CONTROL_RESET: &str = "control.reset";

/// Host → map + UI: a threat was neutralised (payload = `ThreatDestroyed`).
pub const THREAT_DESTROYED: &str = "control.threat.destroyed";
/// Map → UI: a threat reached its impact point (payload = the `Threat`).
pub const LEAKER_EVENT: &str = "control.leaker";

/// A neutralised threat, with the position where it was killed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreatDestroyed {
    pub id: Uuid,
    pub position: Position,
}
/// Host → UI: current firing picture (who engages what + kill count).
pub const ENGAGEMENTS: &str = "control.engagements";
/// Host → UI: positions of interceptors currently in flight.
pub const INTERCEPTORS: &str = "control.interceptors";
/// UI → host: redirect an in-flight interceptor to another target (`RetargetCommand`).
pub const INTERCEPTOR_RETARGET: &str = "control.interceptor.retarget";
/// UI → host: abort an in-flight interceptor (payload = interceptor id string).
pub const INTERCEPTOR_ABORT: &str = "control.interceptor.abort";

/// One interceptor (munition) in flight toward its target (or diverting to the
/// safe drop zone after an abort).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlyingInterceptor {
    pub id: Uuid,
    pub position: Position,
    pub target_id: Uuid,
    pub diverting: bool,
}

/// UI → host: send interceptor `interceptor_id` onto threat `target_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetargetCommand {
    pub interceptor_id: Uuid,
    pub target_id: Uuid,
}

/// One active engagement: platform `platform_id` is firing on `threat_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Engagement {
    pub platform_id: Uuid,
    pub threat_id: Uuid,
}

/// Firing picture published each tick for the operator view.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngagementReport {
    pub lines: Vec<Engagement>,
    pub neutralized: usize,
    /// Safe drop zones where aborted interceptors self-destruct (drawn on the map).
    pub safe_zones: Vec<Position>,
}
