use crate::types::NetConstraint;

/// Net that must route as one straight segment.
///
/// Placement aligns pins along the perpendicular axis so the
/// router can emit a single segment.
#[derive(Debug, Clone)]
pub struct StraightNet {
    pub net: String,
    /// `true`: pins share x (vertical route); `false`: pins share y.
    pub vertical: bool,
}

impl NetConstraint for StraightNet {
    fn net_name(&self) -> &str { &self.net }
}
