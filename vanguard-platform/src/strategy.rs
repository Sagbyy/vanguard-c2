use uuid::Uuid;

use crate::state::PlatformState;

pub fn choose_interceptor(
    state: &PlatformState,
    threat_id: Uuid,
) -> Option<Uuid> {
    let _ = threat_id;

    state
        .platform
        .interceptors
        .iter()
        .find(|i| matches!(i.state, vanguard_core::InterceptorState::Idle))
        .map(|i| i.id)
}