use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;
use uuid::Uuid;

use vanguard_core::{
    DetectedThreat, Interceptor, InterceptorState, PlatformInterceptor, Position, Speed,
    ThreatClassification,
};
use vanguard_platform::{Platform, PlatformState};

pub const WORLD_THREAT_DETECTED: &str = "world.threat.detected";

#[tokio::main]
async fn main() -> Result<()> {
    let nats = async_nats::connect("localhost").await?;

    let orchestrator = vanguard_orchestrator::Orchestrator::new(nats.clone());

    tokio::spawn(async move {
        let _ = orchestrator.run().await;
    });

    let platform_a = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: "Alpha".to_string(),
        position: Position { x: 0.0, y: 0.0 },
        reach: 250.0,
        interceptors: (0..5)
            .map(|_| interceptor(Position { x: 0.0, y: 0.0 }))
            .collect(),
        neighbor_platforms: vec![],
    };

    let platform_b = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: "Bravo".to_string(),
        position: Position { x: 600.0, y: 0.0 },
        reach: 350.0,
        interceptors: (0..5)
            .map(|_| interceptor(Position { x: 600.0, y: 0.0 }))
            .collect(),
        neighbor_platforms: vec![],
    };

    let platform_a = Platform::new(PlatformState::new(platform_a), nats.clone());

    let platform_b = Platform::new(PlatformState::new(platform_b), nats.clone());

    tokio::spawn(async move {
        let _ = platform_a.run().await;
    });

    tokio::spawn(async move {
        let _ = platform_b.run().await;
    });

    sleep(Duration::from_secs(2)).await;

    let mut x = 1000.0;

    let threat_id = Uuid::new_v4();
    loop {
        x -= 50.0;

        let threat = DetectedThreat {
            id: threat_id,
            position: Position { x, y: 0.0 },
            speed: Speed { x: -50.0, y: 0.0 },
            threat_level: 10,
            classification: ThreatClassification::CruiseMissile,
            confidence: 1.0,
            detected_at: 0.0,
        };

        println!(
            "[SIM] threat at ({}, {})",
            threat.position.x, threat.position.y,
        );

        nats.publish(WORLD_THREAT_DETECTED, serde_json::to_vec(&threat)?.into())
            .await?;

        sleep(Duration::from_secs(1)).await;
    }
}

fn interceptor(position: Position) -> Interceptor {
    Interceptor {
        id: Uuid::new_v4(),
        position,
        state: InterceptorState::Idle,
        assigned_track: None,
    }
}
