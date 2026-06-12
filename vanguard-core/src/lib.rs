pub mod events;
pub mod interceptor;
pub mod position;
pub mod threat;

pub use events::Message;
pub use interceptor::{
    DetectedThreat, Interceptor, InterceptorReport, InterceptorState, PlatformInterceptor,
};
pub use position::{Position, Speed};
pub use threat::Threat;
pub use uuid::Uuid;

/// NATS subject where the map publishes the ground-truth threat list.
pub const THREATS_SUBJECT: &str = "map.threats";
