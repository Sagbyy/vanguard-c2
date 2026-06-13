use std::collections::HashMap;
use uuid::Uuid;

use vanguard_core::{Interceptor, NeighborPlatform, ThreatTrack};

#[derive(Clone, Debug)]
pub struct InterceptorInfo {
    pub platform_id: Uuid,
    pub interceptor: Interceptor,
}

pub struct OrchestratorState {
    pub tracks: HashMap<Uuid, ThreatTrack>,
    pub platforms: HashMap<Uuid, NeighborPlatform>,
    pub interceptors: HashMap<Uuid, InterceptorInfo>,
}

impl OrchestratorState {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            platforms: HashMap::new(),
            interceptors: HashMap::new(),
        }
    }
}
