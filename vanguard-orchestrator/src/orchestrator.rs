use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;

use crate::{
    assignment::compute_assignments,
    state::{InterceptorInfo, OrchestratorState},
    subjects::*,
    tracks::{cleanup_tracks, update_track},
};

use vanguard_core::{Assignment, Message, NeighborPlatform};

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
        self.nats
            .publish(subject, serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    async fn publish_strategy(&self, assignments: Vec<Assignment>) -> Result<()> {
        self.publish(STRATEGY_UPDATE, Message::StrategyUpdate { assignments })
            .await
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;

        let mut neighbor_sub = self.nats.subscribe(NEIGHBOR_UPDATE).await?;

        let mut interceptor_sub = self.nats.subscribe(INTERCEPTOR_UPDATE).await?;

        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {

                    cleanup_tracks(
                        &mut self.state.tracks,
                        0.0,
                    );

                    let tracks =
                        self.state
                            .tracks
                            .values()
                            .cloned()
                            .collect::<Vec<_>>();

                    let interceptors =
                        self.state
                            .interceptors
                            .values()
                            .cloned()
                            .collect::<Vec<_>>();

                    let assignments =
                        compute_assignments(
                            &tracks,
                            &interceptors,
                        );

                    self.publish_strategy(
                        assignments,
                    )
                    .await?;
                }

                Some(msg) =
                    threat_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::ThreatDetected {
                        threat,
                        source_platform,
                    } = msg {

                        let track =
                            update_track(
                                &mut self.state.tracks,
                                threat,
                                source_platform,
                            );

                        self.publish(
                            TRACK_UPDATED,
                            Message::TrackUpdated {
                                track,
                            },
                        )
                        .await?;
                    }
                }

                Some(msg) =
                    neighbor_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::NeighborUpdate {
                        platform_id,
                        position,
                        interceptors_remaining,
                    } = msg {

                        self.state
                            .platforms
                            .insert(
                                platform_id,
                                NeighborPlatform {
                                    id: platform_id,
                                    position,
                                    interceptors_remaining,
                                },
                            );
                    }
                }

                Some(msg) =
                    interceptor_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::InterceptorUpdate {
                        platform_id,
                        interceptor,
                    } = msg {

                        self.state
                            .interceptors
                            .insert(
                                interceptor.id,
                                InterceptorInfo {
                                    platform_id,
                                    interceptor,
                                },
                            );
                    }
                }
            }
        }
    }
}
