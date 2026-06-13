use std::time::Duration;

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use uuid::Uuid;

use crate::state::PlatformState;

const THREAT_DETECTED: &str = "vanguard.threat.detected";

const THREAT_ENGAGED: &str = "vanguard.threat.engaged";

const NEIGHBOR_UPDATE: &str = "vanguard.neighbor.update";

const STRATEGY_UPDATE: &str = "vanguard.strategy.update";

use vanguard_core::{DetectedThreat, InterceptorState, Message, NeighborPlatform, Position};

pub struct Platform {
    pub state: PlatformState,
    pub nats: Client,
}

impl Platform {
    async fn publish(&self, subject: &'static str, msg: Message) -> Result<()> {
        self.nats
            .publish(subject, serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    pub async fn publish_neighbor_update(&self) -> Result<()> {
        let available = self
            .state
            .platform
            .interceptors
            .iter()
            .filter(|i| matches!(i.state, InterceptorState::Idle))
            .count();

        self.publish(
            NEIGHBOR_UPDATE,
            Message::NeighborUpdate {
                platform_id: self.state.platform.id,
                position: self.state.platform.position.clone(),
                interceptors_remaining: available,
            },
        )
        .await
    }

    pub async fn detect_threat(&mut self, threat: DetectedThreat) -> Result<()> {
        self.state.threats.insert(threat.id, threat.clone());

        self.publish(
            THREAT_DETECTED,
            Message::ThreatDetected {
                threat: threat.clone(),
                source_platform: self.state.platform.id,
            },
        )
        .await?;

        if self.is_best_platform(&threat) {
            self.engage_threat(threat.id).await?;
        }

        Ok(())
    }

    fn is_best_platform(&self, threat: &DetectedThreat) -> bool {
        let my_distance = self.state.platform.position.distance(&threat.position);

        self.state
            .platform
            .neighbor_platforms
            .iter()
            .filter(|n| n.interceptors_remaining > 0)
            .all(|neighbor| neighbor.position.distance(&threat.position) >= my_distance)
    }

    async fn engage_threat(&mut self, threat_id: Uuid) -> Result<()> {
        let interceptor_id = {
            let Some(interceptor) = self
                .state
                .platform
                .interceptors
                .iter_mut()
                .find(|i| matches!(i.state, InterceptorState::Idle))
            else {
                return Ok(());
            };

            interceptor.state = InterceptorState::Intercepting(threat_id);

            interceptor.id
        };

        self.publish(
            THREAT_ENGAGED,
            Message::ThreatEngaged {
                threat_id,
                platform_id: self.state.platform.id,
                interceptor_id,
            },
        )
        .await?;

        Ok(())
    }

    async fn handle_threat_detected(
        &mut self,
        threat: DetectedThreat,
        source_platform: Uuid,
    ) -> Result<()> {
        self.state.threats.insert(threat.id, threat.clone());

        if source_platform == self.state.platform.id {
            return Ok(());
        }

        if self.is_best_platform(&threat) {
            self.engage_threat(threat.id).await?;
        }

        Ok(())
    }

    fn handle_threat_engaged(&mut self, threat_id: Uuid) {
        self.state.threats.remove(&threat_id);
    }

    fn handle_neighbor_update(
        &mut self,
        platform_id: Uuid,
        position: Position,
        interceptors_remaining: usize,
    ) {
        if platform_id == self.state.platform.id {
            return;
        }

        if let Some(neighbor) = self
            .state
            .platform
            .neighbor_platforms
            .iter_mut()
            .find(|n| n.id == platform_id)
        {
            neighbor.position = position;
            neighbor.interceptors_remaining = interceptors_remaining;

            return;
        }

        self.state
            .platform
            .neighbor_platforms
            .push(NeighborPlatform {
                id: platform_id,
                position,
                interceptors_remaining,
            });
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::ThreatDetected {
                threat,
                source_platform,
            } => {
                self.handle_threat_detected(threat, source_platform).await?;
            }

            Message::ThreatEngaged { threat_id, .. } => {
                self.handle_threat_engaged(threat_id);
            }

            Message::NeighborUpdate {
                platform_id,
                position,
                interceptors_remaining,
            } => {
                self.handle_neighbor_update(platform_id, position, interceptors_remaining);
            }

            Message::StrategyUpdate { .. } => {}
            Message::TrackUpdated { track } => {}
            Message::InterceptorUpdate {
                platform_id,
                interceptor,
            } => {}
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;

        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;

        let mut neighbor_sub = self.nats.subscribe(NEIGHBOR_UPDATE).await?;

        let mut strategy_sub = self.nats.subscribe(STRATEGY_UPDATE).await?;

        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {
                    self.publish_neighbor_update()
                        .await?;
                }

                Some(msg) = threat_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = engaged_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = neighbor_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = strategy_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use super::*;

    use vanguard_core::{
        DetectedThreat, Interceptor, InterceptorState, NeighborPlatform, PlatformInterceptor,
        Position, Speed,
    };

    #[tokio::test]
    async fn closest_platform_wins() {
        let platform = PlatformInterceptor {
            id: Uuid::new_v4(),
            name: "alpha".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            range: 1000.0,
            interceptors: vec![Interceptor {
                id: Uuid::new_v4(),
                position: Position { x: 0.0, y: 0.0 },
                state: InterceptorState::Idle,
                assigned_track: None,
            }],
            neighbor_platforms: vec![NeighborPlatform {
                id: Uuid::new_v4(),
                position: Position { x: 100.0, y: 100.0 },
                interceptors_remaining: 1,
            }],
        };

        let state = PlatformState {
            platform,
            threats: HashMap::new(),
        };

        let nats = async_nats::connect("nats://localhost:4222").await.unwrap();

        let platform = Platform { state, nats };

        let threat = DetectedThreat {
            id: Uuid::new_v4(),
            position: Position { x: 10.0, y: 10.0 },
            speed: Speed { x: 0.0, y: 0.0 },
            threat_level: 10,
            classification: vanguard_core::interceptor::ThreatClassification::Unknown,
            confidence: 1.0,
            detected_at: 3.0,
        };

        assert!(platform.is_best_platform(&threat));
    }
}
