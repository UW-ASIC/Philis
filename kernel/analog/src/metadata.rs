//! Net classification tags the annotator turns into routing rules.

use pnr_core::ids::NetId;

/// A net's functional class and the routing budgets it implies, aF;
/// `None` = unbudgeted.
#[derive(Clone, Debug)]
pub struct NetClassification {
    pub net: NetId,
    pub class: NetClass,
    /// Max wire capacitance to ground (lowered to a drawn-length cap).
    pub c_budget_af: Option<i64>,
    /// Max total coupling onto this net.
    pub max_coupling_af: Option<i64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetClass {
    Signal,
    Clock,
    Supply,
    Ground,
    /// Sensitive reference (bias, bandgap, ADC reference).
    Sensitive,
    Substrate,
}
