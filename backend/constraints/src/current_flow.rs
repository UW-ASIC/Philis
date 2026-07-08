use crate::types::{CurrentFlowDir, DeviceConstraint, DeviceId};

/// Current flow direction annotation for orientation constraints.
#[derive(Debug, Clone)]
pub struct CurrentFlowTag {
    pub device_id: DeviceId,
    pub direction: CurrentFlowDir,
}

impl DeviceConstraint for CurrentFlowTag {
    fn device_id(&self) -> DeviceId { self.device_id }
}
