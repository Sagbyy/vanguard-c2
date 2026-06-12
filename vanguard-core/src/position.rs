use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn distance(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;

        (dx * dx + dy * dy).sqrt()
    }

    /// Moves `distance` toward `target`, without overshooting it.
    pub fn step_toward(&self, target: &Position, distance: f64) -> Position {
        let total = self.distance(target);
        if total <= distance {
            return target.clone();
        }

        let ratio = distance / total;
        Position {
            x: self.x + (target.x - self.x) * ratio,
            y: self.y + (target.y - self.y) * ratio,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Speed {
    pub x: f64,
    pub y: f64,
}
