use vanguard_core::*;

pub struct OrchestratorState {
    pub threats: Vec<DetectedThreat>,
    pub platforms: Vec<PlatformInterceptor>,
}

impl OrchestratorState {
    pub fn new(platforms: Vec<PlatformInterceptor>) -> Self {
        Self {
            threats: Vec::new(),
            platforms,
        }
    }

    pub fn tick(&mut self, reports: &[InterceptorReport]) -> Vec<(Uuid, InterceptorState)> {
        self.update(reports);
        self.assign()
    }

    fn update(&mut self, reports: &[InterceptorReport]) {
        self.threats.clear();

        for report in reports {
            self.threats.extend(report.threats.iter().cloned());
        }
    }

    fn assign(&mut self) -> Vec<(Uuid, InterceptorState)> {
        let mut updates = Vec::new();

        let Some(threat) = self.threats.iter().max_by_key(|t| t.threat_level) else {
            return updates;
        };

        for platform in &mut self.platforms {
            let already_engaged = platform.interceptors.iter().any(|i| {
                matches!(i.state, InterceptorState::Intercepting(target) if target == threat.id)
            });
            if already_engaged {
                continue;
            }

            if let Some(interceptor) = platform
                .interceptors
                .iter_mut()
                .find(|i| matches!(i.state, InterceptorState::Idle))
            {
                interceptor.state = InterceptorState::Intercepting(threat.id);
                updates.push((interceptor.id, interceptor.state.clone()));
            }
        }

        updates
    }

    pub fn display(&self) {
        println!("Threats: {}", self.threats.len());
        println!("Platforms: {}", self.platforms.len());
    }
}
