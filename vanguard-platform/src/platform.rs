use uuid::Uuid;

use crate::{
    state::PlatformState,
    strategy::select_interceptor,
};

use vanguard_core::{
    DetectedThreat,
    InterceptorState,
    Message,
};

pub fn detect_threat(
    state: &mut PlatformState,
    threat: DetectedThreat,
) -> Vec<Message> {
    state.threats.insert(threat.id, threat.clone());

    vec![
        Message::ThreatDetected {
            threat_id: threat.id,
            source_platform: state.platform.id,
        }
    ]
}