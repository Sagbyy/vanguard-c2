use std::time::Duration;

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use uuid::Uuid;

use crate::state::InterceptorRuntimeState;

use vanguard_core::{InterceptorState, Message, subjects::*};

const SPEED: f64 = 100.0;
const INTERCEPT_DISTANCE: f64 = 25.0;

pub struct InterceptorAgent {
    pub state: InterceptorRuntimeState,
    pub nats: Client,
}

impl InterceptorAgent {
    pub fn new(state: InterceptorRuntimeState, nats: Client) -> Self {
        Self { state, nats }
    }

    async fn publish(&self, subject: &str, msg: Message) -> Result<()> {
        self.nats
            .publish(subject.to_string(), serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    fn update_motion(&mut self) -> Option<Uuid> {
        let target_id = self.state.target_id?;

        let Some(track) = self.state.tracks.get(&target_id) else {
            println!(
                "[INTERCEPTOR {}] target {} assigned but no track found",
                self.state.interceptor.id, target_id
            );

            return None;
        };

        let distance_before = self.state.interceptor.position.distance(&track.position);

        self.state.interceptor.position = self
            .state
            .interceptor
            .position
            .step_toward(&track.position, SPEED);

        let distance_after = self.state.interceptor.position.distance(&track.position);

        println!(
            "[INTERCEPTOR {}] moving toward threat {} ({:.1} -> {:.1})",
            self.state.interceptor.id, target_id, distance_before, distance_after
        );

        if self.state.interceptor.position.distance(&track.position) < INTERCEPT_DISTANCE {
            self.state.target_id = None;

            self.state.interceptor.assigned_track = None;

            self.state.interceptor.state = InterceptorState::Idle;

            return Some(target_id);
        }

        None
    }
    async fn publish_update(&self) -> Result<()> {
        self.publish(
            INTERCEPTOR_UPDATE,
            Message::InterceptorUpdate {
                platform_id: self.state.platform_id,
                interceptor: self.state.interceptor.clone(),
            },
        )
        .await
    }

    pub async fn run(mut self) -> Result<()> {
        let mut assignment_sub = self.nats.subscribe(INTERCEPTOR_TARGET_ASSIGNED).await?;

        let mut track_sub = self.nats.subscribe(TRACK_UPDATED).await?;

        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {

                    if let Some(threat_id) =
                        self.update_motion()
                    {
                        self.publish(
                            THREAT_DESTROYED,
                            Message::ThreatDestroyed {
                                threat_id,
                                platform_id:
                                    self.state.platform_id,
                                interceptor_id:
                                    self.state.interceptor.id,
                            },
                        )
                        .await?;
                    }

                    self.publish_update()
                        .await?;
                }

                Some(msg) =
                    assignment_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    if let Message::InterceptorTargetAssigned {
                        interceptor_id,
                        threat_id,
                    } = msg {

                        if interceptor_id
                            != self.state.interceptor.id
                        {
                            continue;
                        }

                        self.state.target_id =
                            Some(threat_id);

                        self.state.interceptor.assigned_track =
                            Some(threat_id);

                        self.state.interceptor.state =
                            InterceptorState::Intercepting(
                                threat_id,
                            );
                    }
                }

                Some(msg) =
                    track_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    if let Message::TrackUpdated {
                        track,
                    } = msg {

                        self.state
                            .tracks
                            .insert(
                                track.threat_id,
                                track,
                            );
                    }
                }
            }
        }
    }
}
