mod cli;

use clap::Parser;
use uuid::Uuid;
use vanguard_core::{Interceptor, Position};

use crate::cli::Args;

const SIGHT_ANGLE: f64 = 120.0;
const DETECTION_RANGE: f64 = 1_500.0;
const TURN_SPEED: f64 = 90.0;

fn main() {
    let args = Args::parse();

    let system = Interceptor {
        id: Uuid::new_v4(),
        name: args.name,
        position: Position { x: args.x, y: args.y, z: args.z },
        sight_angle: SIGHT_ANGLE,
        detection_range: DETECTION_RANGE,
        turn_speed: TURN_SPEED,
        ammo_remaining: args.interceptors,
        current_order: None,
    };

    println!(
        "{} (id {}) online at ({:.0}, {:.0}, {:.0}) — detection range {:.0} m, {} interceptor(s) ready",
        system.name,
        system.id,
        system.position.x,
        system.position.y,
        system.position.z,
        system.detection_range,
        system.ammo_remaining,
    );
}
