use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;

use crate::{
    assignment::compute_assignments,
    state::{InterceptorInfo, OrchestratorState},
    tracks::{cleanup_tracks, update_track},
};

use vanguard_core::{
    Assignment, Message, NeighborPlatform, interceptor::TrackStatus, subjects::*,
};

pub struct Orchestrator {
    pub state: OrchestratorState,
    pub nats: Client,
}

impl Orchestrator {
    pub fn new(nats: Client) -> Self {
        Self {
            state: OrchestratorState::new(),
            nats,
        }
    }

    async fn publish(&self, subject: &'static str, msg: Message) -> Result<()> {
        println!("[{}] PUB {} {:?}", "Orchestrator", subject, msg);
        self.nats
            .publish(subject, serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    async fn publish_strategy(&self, assignments: Vec<Assignment>) -> Result<()> {
        self.publish(STRATEGY_UPDATE, Message::StrategyUpdate { assignments })
            .await
    }

    /// Periodic recompute: prune stale tracks, then publish a fresh assignment.
    async fn tick(&mut self) -> Result<()> {
        cleanup_tracks(&mut self.state.tracks, 0.0);

        let tracks = self
            .state
            .tracks
            .values()
            .map(|t| t.track.clone())
            .collect::<Vec<_>>();

        let interceptors = self
            .state
            .interceptors
            .values()
            .cloned()
            .collect::<Vec<_>>();

        let assignments = compute_assignments(&tracks, &interceptors);

        self.publish_strategy(assignments).await
    }

    async fn on_threat_destroyed(&mut self, payload: &[u8]) -> Result<()> {
        let Message::ThreatDestroyed { threat_id, .. } = serde_json::from_slice(payload)? else {
            return Ok(());
        };

        if let Some(track) = self.state.tracks.get_mut(&threat_id) {
            track.track.status = TrackStatus::Destroyed;
            let track = track.track.clone();
            self.publish(TRACK_UPDATED, Message::TrackUpdated { track })
                .await?;
        }

        self.state.tracks.remove(&threat_id);
        Ok(())
    }

    async fn on_threat_detected(&mut self, payload: &[u8]) -> Result<()> {
        let Message::ThreatDetected {
            threat,
            source_platform,
        } = serde_json::from_slice(payload)?
        else {
            return Ok(());
        };

        let track = update_track(&mut self.state.tracks, threat, source_platform);
        self.publish(TRACK_UPDATED, Message::TrackUpdated { track })
            .await
    }

    async fn on_threat_engaged(&mut self, payload: &[u8]) -> Result<()> {
        let Message::ThreatEngaged {
            threat_id,
            platform_id,
            ..
        } = serde_json::from_slice(payload)?
        else {
            return Ok(());
        };

        if let Some(track) = self.state.tracks.get_mut(&threat_id) {
            track.track.status = TrackStatus::Engaged;
            track.track.engaged_by = Some(platform_id);
            println!(
                "[Orchestrator] Track {} engaged by {}",
                threat_id, platform_id
            );
            let track = track.track.clone();
            self.publish(TRACK_UPDATED, Message::TrackUpdated { track })
                .await?;
        }

        Ok(())
    }

    async fn on_neighbor_update(&mut self, payload: &[u8]) -> Result<()> {
        let Message::NeighborUpdate {
            platform_id,
            position,
            reach: _,
            interceptors_remaining,
        } = serde_json::from_slice(payload)?
        else {
            return Ok(());
        };

        if let Some(platform) = self.state.platforms.get_mut(&platform_id) {
            platform.position = position;
            platform.interceptors_remaining = interceptors_remaining;
        }

        Ok(())
    }

    /// Register a freshly announced platform and cross-notify every neighbour
    /// whose radar bubble overlaps it.
    async fn on_new_platform(&mut self, payload: &[u8]) -> Result<()> {
        let Message::NewPlatform {
            platform_id,
            position,
            reach,
        } = serde_json::from_slice(payload)?
        else {
            return Ok(());
        };

        let neighbors: Vec<_> = self
            .state
            .platforms
            .values()
            .cloned()
            .filter(|other| other.position.distance(&position) <= (other.reach + reach) as f64)
            .collect();

        self.state.platforms.insert(
            platform_id,
            NeighborPlatform {
                id: platform_id,
                position: position.clone(),
                reach,
                interceptors_remaining: 0,
            },
        );

        for neighbor in neighbors {
            let to_neighbor = Message::NeighborUpdate {
                platform_id,
                position: position.clone(),
                reach,
                interceptors_remaining: 0,
            };
            self.nats
                .publish(
                    vanguard_core::neighbor_subject(&neighbor.id),
                    serde_json::to_vec(&to_neighbor)?.into(),
                )
                .await?;

            let to_new = Message::NeighborUpdate {
                platform_id: neighbor.id,
                position: neighbor.position,
                reach: neighbor.reach,
                interceptors_remaining: neighbor.interceptors_remaining,
            };
            self.nats
                .publish(
                    vanguard_core::neighbor_subject(&platform_id),
                    serde_json::to_vec(&to_new)?.into(),
                )
                .await?;
        }

        Ok(())
    }

    async fn on_interceptor_update(&mut self, payload: &[u8]) -> Result<()> {
        let Message::InterceptorUpdate {
            platform_id,
            interceptor,
        } = serde_json::from_slice(payload)?
        else {
            return Ok(());
        };

        self.state.interceptors.insert(
            interceptor.id,
            InterceptorInfo {
                platform_id,
                interceptor,
            },
        );

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;
        let mut destroyed_sub = self.nats.subscribe(THREAT_DESTROYED).await?;
        let mut neighbor_sub = self.nats.subscribe(NEIGHBOR_UPDATE).await?;
        let mut interceptor_sub = self.nats.subscribe(INTERCEPTOR_UPDATE).await?;
        let mut new_platform_sub = self.nats.subscribe(NEW_PLATFORM).await?;
        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;

        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = heartbeat.tick() => self.tick().await?,
                Some(msg) = destroyed_sub.next() => self.on_threat_destroyed(&msg.payload).await?,
                Some(msg) = threat_sub.next() => self.on_threat_detected(&msg.payload).await?,
                Some(msg) = engaged_sub.next() => self.on_threat_engaged(&msg.payload).await?,
                Some(msg) = neighbor_sub.next() => self.on_neighbor_update(&msg.payload).await?,
                Some(msg) = new_platform_sub.next() => self.on_new_platform(&msg.payload).await?,
                Some(msg) = interceptor_sub.next() => self.on_interceptor_update(&msg.payload).await?,
            }
        }
    }
}
