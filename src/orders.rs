use crate::models::Position;

#[derive(Clone, Debug)]
pub enum Order {
    Idle,
    MoveTo(Position),
    TrackThreat(usize),
    InterceptThreat(usize),
}

#[derive(Clone, Debug)]
pub struct InterceptorCommand {
    pub interceptor_id: usize,
    pub order: Order,
}