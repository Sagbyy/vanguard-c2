use std::collections::HashMap;

use uuid::Uuid;

use vanguard_core::{DetectedThreat, PlatformInterceptor};

pub struct PlatformState {
    pub platform: PlatformInterceptor,
    pub threats: HashMap<Uuid, DetectedThreat>,
}

impl PlatformState {
    pub fn new(platform: PlatformInterceptor) -> Self {
        Self {
            platform,
            threats: HashMap::new(),
        }
    }
}
