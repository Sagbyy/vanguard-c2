pub mod control;
pub mod events;
pub mod interceptor;
pub mod kalman;
pub mod position;
pub mod radar;
pub mod solver;
pub mod subjects;
pub mod threat;

pub use kalman::KalmanTrack;
pub use subjects::*;

pub use control::{
    CONTROL_RESET, ENGAGEMENTS, Engagement, EngagementReport, FlyingInterceptor, INTERCEPTOR_ABORT,
    INTERCEPTOR_RETARGET, INTERCEPTORS, LEAKER_EVENT, MAP_CONFIG, MapConfig, PLATFORM_ADD,
    PLATFORM_REMOVE, PlatformSpec, RetargetCommand, THREAT_DESTROYED, ThreatDestroyed,
};
pub use events::{Assignment, Message};
pub use interceptor::{
    DetectedThreat, Interceptor, InterceptorReport, InterceptorState, NeighborPlatform,
    PlatformInterceptor, ThreatClassification, ThreatTrack,
};
pub use position::{Position, Speed, predicted_intercept};
pub use radar::Radar;
pub use solver::{SOLVE_ASSIGNMENT, SolveArc, SolveRequest, SolveResponse};
pub use threat::Threat;
pub use uuid::Uuid;

/// NATS subject where the map publishes the ground-truth threat list.
pub const THREATS_SUBJECT: &str = "map.threats";

/// Wildcard matching every platform's radar reports (orchestrator side).
pub const REPORTS_SUBJECT_WILDCARD: &str = "platform.*.report";

/// Subject where one platform publishes its radar reports.
pub fn report_subject(platform_id: &Uuid) -> String {
    format!("platform.{platform_id}.report")
}
