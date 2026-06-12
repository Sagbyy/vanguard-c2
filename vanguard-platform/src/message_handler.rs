use vanguard_core::Message;

use crate::state::PlatformState;

pub fn handle_message(
    state: &mut PlatformState,
    message: Message,
) {
    match message {
        Message::ThreatDetected { .. } => {}

        Message::ThreatEngaged { .. } => {}

        Message::PlatformStatus { .. } => {}

        Message::NeighborUpdate { .. } => {}

        Message::StrategyUpdate { strategy } => {
            state.current_strategy = strategy;
        }
    }
}