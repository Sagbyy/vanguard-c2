use uuid::Uuid;
pub type PlatformId = Uuid;
pub type InterceptorId = Uuid;
pub type ThreatId = Uuid;
use crate::{DetectedThreat, Interceptor, Position, ThreatTrack};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    ThreatDetected {
        threat: DetectedThreat,
        source_platform: PlatformId,
    },

    ThreatEngaged {
        threat_id: ThreatId,
        platform_id: PlatformId,
        interceptor_id: InterceptorId,
    },

    NeighborUpdate {
        platform_id: PlatformId,
        position: Position,
        interceptors_remaining: usize,
    },

    StrategyUpdate {
        assignments: Vec<Assignment>,
    },

    TrackUpdated {
        track: ThreatTrack,
    },

    InterceptorUpdate {
        platform_id: PlatformId,
        interceptor: Interceptor,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    pub platform_id: Uuid,
    pub interceptor_id: Uuid,
    pub track_id: Uuid,
}
