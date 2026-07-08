use std::collections::HashMap;

use crate::types::{BlockType, CurrentFlowDir, DeviceId, GroupConstraint};

/// Recognized analog building block (diff pair, mirror, cascode, etc.).
#[derive(Debug, Clone)]
pub struct AnalogBlock {
    pub block_id: String,
    pub block_type: BlockType,
    pub devices: Vec<DeviceId>,
    pub terminal_roles: HashMap<String, String>,
    pub unit_count: HashMap<String, u32>,
    pub axes: Vec<f64>,
    pub current_flow_roles: HashMap<String, CurrentFlowDir>,
}

impl GroupConstraint for AnalogBlock {
    fn devices(&self) -> &[DeviceId] { &self.devices }
}
