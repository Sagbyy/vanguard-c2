#[derive(Clone, Debug, PartialEq)]
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
}

#[derive(Clone, Debug)]
pub struct Speed{
    pub x: f64,
    pub y: f64
}