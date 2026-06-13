use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::position::{Position, Speed};

#[derive(Clone, Debug)]
pub struct PlatformInterceptor {
    pub id: Uuid,
    pub name: String,
    pub position: Position,
    pub interceptors: Vec<Interceptor>,
    pub range: f64,
    pub neighbor_platforms: Vec<NeighborPlatform>,
}

#[derive(Clone, Debug)]
pub struct NeighborPlatform {
    pub id: Uuid,
    pub position: Position,
    pub interceptors_remaining: usize,
}

// possibilité: étendre le lien par la suite
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interceptor {
    pub id: Uuid,
    pub position: Position,
    pub state: InterceptorState,
    pub assigned_track: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterceptorState {
    Idle,
    MovingTo(Position),
    Intercepting(Uuid),
    Destroyed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterceptorReport {
    pub platform_id: Uuid,
    pub threats: Vec<DetectedThreat>,
    pub interceptors: Vec<Interceptor>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DetectedThreat {
    pub id: Uuid,
    pub position: Position,
    pub speed: Speed,
    pub threat_level: usize,
    pub classification: ThreatClassification,
    pub confidence: f64,
    pub detected_at: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ThreatClassification {
    Unknown,
    Drone,
    FPVDrone,
    Helicopter,
    Aircraft,
    CruiseMissile,
    BallisticMissile,
    Friendly,
    Civilian,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreatTrack {
    pub track_id: Uuid,
    pub position: Position,
    pub velocity: Speed,
    pub confidence: f64,
    pub threat_level: usize,
    pub last_update: f64,
    pub source_platforms: Vec<Uuid>,
}
