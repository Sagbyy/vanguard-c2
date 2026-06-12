use uuid::Uuid;

pub type PlatformId = Uuid;
pub type InterceptorId = Uuid;
pub type ThreatId = Uuid;

#[derive(Clone, Debug)]
pub enum Message {
    ThreatDetected {
        threat_id: ThreatId,
        source_platform: PlatformId,
    },

    ThreatEngaged {
        threat_id: ThreatId,
        platform_id: PlatformId,
        interceptor_id: InterceptorId,
    },

    PlatformStatus {
        platform_id: PlatformId,
        available_interceptors: usize,
    },

    NeighborUpdate {
        platform_id: PlatformId,
        neighbors: Vec<PlatformId>,
    },

    //placeholder
    StrategyUpdate {
        strategy: String,
    },
}