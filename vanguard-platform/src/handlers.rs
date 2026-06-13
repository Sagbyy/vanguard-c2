//! Message-driven logic for [`Platform`]: sensor fusion, engagement decisions
//! and the per-variant handlers dispatched from the run loop.

use anyhow::Result;
use uuid::Uuid;

use crate::platform::Platform;
use crate::state::TrackedThreat;

use vanguard_core::{
    DetectedThreat, Interceptor, InterceptorState, KalmanTrack, Message, NeighborPlatform,
    Position, ThreatTrack, interceptor::TrackStatus, subjects::*,
};

impl Platform {
    pub async fn detect_threat(&mut self, threat: DetectedThreat) -> Result<()> {
        println!("[{}] DETECT {}", self.state.platform.name, threat.id,);

        let is_new_track = self.handle_threat_detected(threat.clone())?;

        self.state.threats.insert(threat.id, threat.clone());

        self.publish(
            THREAT_DETECTED,
            Message::ThreatDetected {
                threat: threat.clone(),
                source_platform: self.state.platform.id,
            },
        )
        .await?;

        if is_new_track && self.is_best_platform(&threat) {
            self.engage_threat(threat.id).await?;
        }

        Ok(())
    }

    fn is_best_platform(&self, threat: &DetectedThreat) -> bool {
        let my_distance = self.state.platform.position.distance(&threat.position);
        if my_distance > self.state.platform.reach {
            return false;
        }

        self.state
            .platform
            .neighbor_platforms
            .iter()
            .filter(|n| n.interceptors_remaining > 0)
            .all(|neighbor| neighbor.position.distance(&threat.position) >= my_distance)
    }

    async fn engage_threat(&mut self, threat_id: Uuid) -> Result<()> {
        if self.state.engaged_threats.contains(&threat_id) {
            return Ok(());
        }

        let Some(interceptor) = self
            .state
            .platform
            .interceptors
            .iter()
            .find(|i| matches!(i.state, InterceptorState::Idle))
        else {
            return Ok(());
        };

        let interceptor_id = interceptor.id;

        self.state.engaged_threats.insert(threat_id);

        self.publish(
            INTERCEPTOR_TARGET_ASSIGNED,
            Message::InterceptorTargetAssigned {
                interceptor_id,
                threat_id,
            },
        )
        .await?;

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

    fn handle_threat_detected(&mut self, threat: DetectedThreat) -> Result<bool> {
        match self.state.tracks.get_mut(&threat.id) {
            Some(track) => {
                track.kalman.update(threat.position.x, threat.position.y);

                let (x, y) = track.kalman.position();

                let (vx, vy) = track.kalman.velocity();

                track.track.position.x = x;
                track.track.position.y = y;

                track.track.velocity.x = vx;
                track.track.velocity.y = vy;

                track.track.confidence = threat.confidence;

                track.track.threat_level = threat.threat_level;

                Ok(false)
            }

            None => {
                self.state.tracks.insert(
                    threat.id,
                    TrackedThreat {
                        track: ThreatTrack {
                            threat_id: threat.id,
                            position: threat.position.clone(),
                            velocity: threat.speed.clone(),
                            confidence: threat.confidence,
                            threat_level: threat.threat_level,
                            last_update: threat.detected_at,
                            source_platforms: vec![self.state.platform.id],
                            status: TrackStatus::Detected,
                            engaged_by: None,
                        },
                        kalman: KalmanTrack::new(
                            threat.position.x,
                            threat.position.y,
                            threat.speed.x,
                            threat.speed.y,
                        ),
                    },
                );

                Ok(true)
            }
        }
    }

    fn handle_threat_engaged(&mut self, threat_id: Uuid, _interceptor_id: Uuid) {
        self.state.engaged_threats.insert(threat_id);

        self.state.threats.remove(&threat_id);

        if let Some(track) = self.state.tracks.get_mut(&threat_id) {
            track.track.status = TrackStatus::Engaged;
        }
    }

    fn handle_neighbor_update(
        &mut self,
        platform_id: Uuid,
        position: Position,
        reach: f64,
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
            neighbor.reach = reach;
            neighbor.interceptors_remaining = interceptors_remaining;

            return;
        }

        println!(
            "[{}] neighbor added {}",
            self.state.platform.name, platform_id
        );

        self.state
            .platform
            .neighbor_platforms
            .push(NeighborPlatform {
                id: platform_id,
                position,
                reach,
                interceptors_remaining,
            });
    }

    fn handle_track_updated(&mut self, track: ThreatTrack) -> Result<()> {
        println!(
            "[{}] TRACK {} {:?}",
            self.state.platform.name, track.threat_id, track.status,
        );

        let status = track.status.clone();

        if let Some(local_track) = self.state.tracks.get_mut(&track.threat_id) {
            local_track.track.status = status.clone();

            local_track.track.engaged_by = track.engaged_by;
        }

        match status {
            TrackStatus::Detected => {}

            TrackStatus::Engaged => {
                self.state.engaged_threats.insert(track.threat_id);
            }

            TrackStatus::Destroyed => {
                self.state.engaged_threats.remove(&track.threat_id);

                self.state.tracks.remove(&track.threat_id);

                self.state.threats.remove(&track.threat_id);
            }
        }

        Ok(())
    }

    fn handle_interceptor_update(&mut self, interceptor: Interceptor) {
        if let Some(local) = self
            .state
            .platform
            .interceptors
            .iter_mut()
            .find(|i| i.id == interceptor.id)
        {
            *local = interceptor.clone();
        }

        self.state
            .known_interceptors
            .insert(interceptor.id, interceptor);
    }

    fn handle_threat_destroyed(&mut self, threat_id: Uuid, interceptor_id: Uuid) {
        self.state.engaged_threats.remove(&threat_id);

        self.state.tracks.remove(&threat_id);

        self.state.threats.remove(&threat_id);

        if let Some(interceptor) = self
            .state
            .platform
            .interceptors
            .iter_mut()
            .find(|i| i.id == interceptor_id)
        {
            interceptor.state = InterceptorState::Idle;

            interceptor.assigned_track = None;
        }
    }

    /// World feed: a ground-truth threat entered the world. Only react if it
    /// falls within this platform's radar reach.
    pub(crate) async fn on_world_threat(&mut self, payload: &[u8]) -> Result<()> {
        let threat: DetectedThreat = serde_json::from_slice(payload)?;

        println!(
            "[{}] WORLD {} ({})",
            self.state.platform.name, threat.id, threat.position.x,
        );

        if self.state.platform.position.distance(&threat.position) <= self.state.platform.reach {
            // Use detect_threat so that engagement logic is triggered
            self.detect_threat(threat).await?;
        }

        Ok(())
    }

    pub(crate) async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::ThreatDetected {
                threat,
                source_platform: _,
            } => {
                let _ = self.handle_threat_detected(threat);
            }

            Message::ThreatDestroyed {
                threat_id,
                interceptor_id,
                ..
            } => {
                self.handle_threat_destroyed(threat_id, interceptor_id);
            }

            Message::ThreatEngaged {
                threat_id,
                platform_id: _,
                interceptor_id,
            } => {
                self.handle_threat_engaged(threat_id, interceptor_id);
            }

            Message::NeighborUpdate {
                platform_id,
                position,
                reach,
                interceptors_remaining,
            } => {
                self.handle_neighbor_update(platform_id, position, reach, interceptors_remaining);
            }

            Message::StrategyUpdate { .. } => {}
            Message::TrackUpdated { track } => {
                let _ = self.handle_track_updated(track);
            }
            Message::NewPlatform {
                platform_id: _,
                position: _,
                reach: _,
            } => {}
            Message::InterceptorUpdate {
                platform_id: _,
                interceptor,
            } => self.handle_interceptor_update(interceptor),
            _ => {}
        }

        Ok(())
    }
}
