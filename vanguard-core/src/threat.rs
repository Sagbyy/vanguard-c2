use crate::Position;

pub type InterceptorId = uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DetectedThreat {
    pub id: InterceptorId,
    pub position: Position,
    pub threat_level: usize,
}

