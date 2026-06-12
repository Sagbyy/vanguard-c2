#[derive(Clone, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn distance(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

#[derive(Clone, Debug)]
pub struct Threat {
    pub id: usize,
    pub position: Position,
    pub threat_level: usize,
}

#[derive(Clone, Debug)]
pub struct Interceptor {
    pub id: usize,
    pub position: Position,
    pub sight_angle: f64,
    pub sight_reach: f64,
    pub turn_speed: f64,
    pub ammo_remaining: usize,
}

#[derive(Clone, Debug)]
pub struct InterceptorReport {
    pub interceptor_id: usize,
    pub position: Position,
    pub threats: Vec<Threat>,
}