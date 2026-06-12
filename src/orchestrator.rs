use crate::models::*;
use crate::orders::*;

pub struct OrchestratorState {
    pub threats: Vec<Threat>,
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
    ) -> Vec<InterceptorCommand> {
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
            }

            self.threats.extend(report.threats.iter().cloned());
        }
    }

    fn assign(&self) -> Vec<InterceptorCommand> {
        let mut commands = Vec::new();
        let mut used = Vec::new();

        let mut threats = self.threats.clone();

        threats.sort_by(|a, b| b.threat_level.cmp(&a.threat_level));

        for threat in threats {
            let interceptor = self
                .interceptors
                .iter()
                .filter(|i| {
                    i.ammo_remaining > 0
                        && !used.contains(&i.id)
                })
                .min_by(|a, b| {
                    a.position
                        .distance(&threat.position)
                        .partial_cmp(
                            &b.position.distance(&threat.position),
                        )
                        .unwrap()
                });

            if let Some(interceptor) = interceptor {
                used.push(interceptor.id);

                commands.push(InterceptorCommand {
                    interceptor_id: interceptor.id,
                    order: Order::InterceptThreat(threat.id),
                });
            }
        }

        for interceptor in &self.interceptors {
            if !used.contains(&interceptor.id) {
                commands.push(InterceptorCommand {
                    interceptor_id: interceptor.id,
                    order: Order::Idle,
                });
            }
        }

        commands
    }

    pub fn display(&self) {
        println!("Threats: {}", self.threats.len());
        println!("Interceptors: {}", self.interceptors.len());
    }
}