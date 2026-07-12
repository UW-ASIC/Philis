use crate::types::{DeviceConstraint, DeviceId};

/// Bias current annotation for a device.
#[derive(Debug, Clone)]
pub struct BiasCurrentTag {
    pub device_id: DeviceId,
    /// Drain current (mA).
    pub id_ma: f64,
    /// Overdrive voltage Vgs - Vth (mV).
    pub vgs_minus_vth_mv: f64,
}

impl DeviceConstraint for BiasCurrentTag {
    fn device_id(&self) -> DeviceId {
        self.device_id
    }
}
