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
    /// Bias line: gates only (plus diode D=G), or the gate of a current source/cascode (EXT-18).
    Bias,
    /// Reference voltage: a bandgap core node, a cascoded reference, a DAC reference plate.
    Reference,
    /// Logic level that does not toggle with a clock (logic G/D nets).
    DigitalStatic,
    /// Logic driven by a clock or by other switching logic: an aggressor.
    DigitalSwitching,
    /// Noisy node (charge-pump output, `_n` suffix without a `_p` twin): an aggressor.
    Noisy,
}
