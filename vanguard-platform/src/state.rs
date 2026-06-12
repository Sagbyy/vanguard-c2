use std::collections::HashMap;

use uuid::Uuid;
use vanguard_core::{DetectedThreat, Platform};

pub struct PlatformState {
    pub platform: Platform,
    pub threats: HashMap<Uuid, DetectedThreat>,
}

impl PlatformState {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            threats: HashMap::new(),
        }
    }
}