use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::position::Position;

/// Ground-truth threat: what actually exists on the map
/// (`DetectedThreat` is what a platform sees of it).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Threat {
    pub id: Uuid,
    pub position: Position,
    pub speed: f64,
    pub threat_level: usize,
}
