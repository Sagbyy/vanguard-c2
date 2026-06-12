pub mod events;
pub mod interceptor;
pub mod position;
pub mod threat;

pub use events::Message;
pub use interceptor::{
    DetectedThreat, Interceptor, InterceptorReport, InterceptorState, NeighborPlatform,
    PlatformInterceptor,
};
pub use position::{Position, Speed};
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
