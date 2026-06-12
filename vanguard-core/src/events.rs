use uuid::Uuid;
pub type PlatformId = Uuid;
pub type InterceptorId = Uuid;
pub type ThreatId = Uuid;
use crate::{DetectedThreat, Position};

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
        strategy: String,
    },
}
