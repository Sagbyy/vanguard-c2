mod models;
mod orders;
mod orchestrator;

use models::*;
fn main() {
    let interceptors = vec![
        Interceptor {
            id: 1,
            position: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            sight_angle: 120.0,
            sight_reach: 500.0,
            turn_speed: 30.0,
            ammo_remaining: 10,
        },
    ];

    let reports = vec![
        InterceptorReport {
            interceptor_id: 1,
            position: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            threats: vec![Threat {
                id: 100,
                position: Position {
                    x: 50.0,
                    y: 10.0,
                    z: 0.0,
                },
                threat_level: 5,
            }],
        },
    ];

    let mut orchestrator =
        OrchestratorState::new(interceptors);

    let commands = orchestrator.tick(&reports);
    for command in commands {
        println!("{command:?}");
    }
}