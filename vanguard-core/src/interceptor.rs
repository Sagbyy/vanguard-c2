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
    pub neighbor_platforms: Vec<NeighborPlatform>
}

#[derive(Clone, Debug)]
pub struct NeighborPlatform {
    pub id: Uuid,
    pub position: Position,
    pub interceptors_remaining: usize,
}

// possibilité: étendre le lien par la suite
#[derive(Clone, Debug)]
pub struct Interceptor {
    pub id: Uuid,
    pub position: Position,
    pub state: InterceptorState,
}

#[derive(Clone, Debug)]
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
    pub interceptors_remaining: usize,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetectedThreat {
    pub id: Uuid,
    pub position: Position,
    pub speed: Speed,
    pub threat_level: usize,
}
