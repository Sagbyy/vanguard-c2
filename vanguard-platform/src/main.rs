mod handlers;
mod platform;
mod state;
use platform::Platform;
use state::PlatformState;

use async_nats::connect;
use uuid::Uuid;

use vanguard_core::{Interceptor, InterceptorState, PlatformInterceptor, Position};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let nats = connect("nats://localhost:4222").await?;

    let platform = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: "Alpha".to_string(),
        position: Position { x: 0.0, y: 0.0 },
        reach: 1000.0,

        interceptors: vec![
            Interceptor {
                id: Uuid::new_v4(),
                position: Position { x: 0.0, y: 0.0 },
                state: InterceptorState::Idle,
                assigned_track: None,
            },
            Interceptor {
                id: Uuid::new_v4(),
                position: Position { x: 0.0, y: 0.0 },
                state: InterceptorState::Idle,
                assigned_track: None,
            },
        ],

        neighbor_platforms: vec![],
    };

    let state = PlatformState::new(platform);

    let platform = Platform::new(state, nats);

    platform.run().await
}
