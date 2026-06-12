use crate::models::*;

pub struct OrchestratorState {
    pub threats: Vec<DetectedThreat>,
    pub interceptors: Vec<Interceptor>,
}

impl OrchestratorState {
    pub fn new(interceptors: Vec<Interceptor>) -> Self {
        Self {
            threats: Vec::new(),
            interceptors,
        }
    }

    pub fn tick(
        &mut self,
        reports: &[InterceptorReport],
    ) -> Vec<(usize, InterceptorOrder)> {
        self.update(reports);
        self.assign()
    }

    fn update(&mut self, reports: &[InterceptorReport]) {
        self.threats.clear();

        for report in reports {
            if let Some(interceptor) = self
                .interceptors
                .iter_mut()
                .find(|i| i.id == report.interceptor_id)
            {
                interceptor.position = report.position.clone();
                interceptor.ammo_remaining = report.ammo_remaining;
            }

            self.threats.extend(report.threats.iter().cloned());
        }
    }

    fn assign(&mut self) -> Vec<(usize, InterceptorOrder)> {
        let mut updates = Vec::new();

        for interceptor in &mut self.interceptors {
            let order = self
                .threats
                .iter()
                .max_by_key(|t| t.threat_level)
                .map(|t| InterceptorOrder::Intercept(t.id))
                .unwrap_or(InterceptorOrder::Idle);

            if interceptor.current_order.as_ref() != Some(&order) {
                interceptor.current_order = Some(order.clone());
                updates.push((interceptor.id, order));
            }
        }

        updates
    }

    pub fn display(&self) {
        println!(
            "Threats: {}",
            self.threats.len()
        );
        println!(
            "Interceptors: {}",
            self.interceptors.len()
        );
    }
}