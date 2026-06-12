#[derive(Clone, Debug, PartialEq)]
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