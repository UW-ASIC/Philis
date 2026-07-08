use crate::types::{DeviceId, PairConstraint};

/// DTI (Deep Trench Isolation) pair constraint.
#[derive(Debug, Clone)]
pub struct DtiPair {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    pub s_max: f64,
    pub d_dti: f64,
}

impl PairConstraint for DtiPair {
    fn device_a(&self) -> DeviceId { self.device_a }
    fn device_b(&self) -> DeviceId { self.device_b }
    fn distance_budget_um(&self) -> f64 { self.d_dti }
}
