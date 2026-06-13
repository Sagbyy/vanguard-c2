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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Speed {
    pub x: f64,
    pub y: f64,
}

/// Predicted Intercept Point: where a `speed`-fast interceptor launched from
/// `shooter` should aim to hit a target at `target` moving at constant velocity
/// `vel`. Solves |target + vel·t − shooter| = speed·t (a quadratic in t) and
/// returns the future target position at the earliest positive intercept time.
/// Returns `None` if the target is uncatchable (interceptor too slow).
pub fn predicted_intercept(
    shooter: &Position,
    speed: f64,
    target: &Position,
    vel: &Speed,
) -> Option<Position> {
    let rx = target.x - shooter.x;
    let ry = target.y - shooter.y;

    let a = vel.x * vel.x + vel.y * vel.y - speed * speed;
    let b = 2.0 * (rx * vel.x + ry * vel.y);
    let c = rx * rx + ry * ry;

    let t = if a.abs() < 1e-6 {
        // Target speed ≈ interceptor speed → linear equation.
        if b.abs() < 1e-9 {
            return None;
        }
        -c / b
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sq = disc.sqrt();
        let t1 = (-b - sq) / (2.0 * a);
        let t2 = (-b + sq) / (2.0 * a);
        // Earliest strictly-positive root.
        [t1, t2]
            .into_iter()
            .filter(|t| *t > 0.0)
            .fold(f64::INFINITY, f64::min)
    };

    if !t.is_finite() || t <= 0.0 {
        return None;
    }
    Some(Position {
        x: target.x + vel.x * t,
        y: target.y + vel.y * t,
    })
}
