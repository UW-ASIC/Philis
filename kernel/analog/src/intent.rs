//! Extraction's contract (EXT-12): batch identity and provenance (EXT-10),
//! and everything the annotator infers about a circuit (matched sets,
//! symmetry compounds, the group tree, net and device facts), independent of
//! how placement turns it into rules. Reached by path only: `verify::Intent`
//! shares the name.

pub use crate::matching::class::{Family, MatchClass, MatchKind};
pub use crate::placement::symmetry::AxisDir;
use pnr_core::ids::{AxisId, DeviceId, NetId};

/// Dense id within one `annotator::Problem`, assigned in emission order:
/// permutation-invariant because emission order is (EXT-06).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ConstraintId(pub u32);

/// Where a batch or an inferred constraint came from. `Copy` because
/// `rule::Tagged` copies its [`BatchMeta`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    /// Emitted for a recognised block of this template.
    Pattern { template: &'static str },
    /// Emitted from net classes, the substrate, or a cross-block rule.
    NetClass,
    /// Propagated from a symmetry seed (EXT-14).
    Symmetry { seed: ConstraintId },
    /// A shared-bias group (EXT-15).
    SharedBias,
    /// A passive or bipolar set rule (EXT-19), e.g. `"divider"`, `"dac_bank"`.
    PassiveSet { rule: &'static str },
    /// Index into the user sidecar (EXT-26).
    User { index: u32 },
}

/// A batch's stable identity and provenance; prices and reports re-find a
/// batch by `id`, never by its position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BatchMeta {
    /// Unique within one `annotator::Problem`.
    pub id: ConstraintId,
    /// Why the batch exists.
    pub origin: Origin,
}

/// Who set a matched set's [`MatchClass`]. User and Spec count as explicit
/// for MAT-08 (`MatchedSet::class_explicit = source != Role`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassSource {
    /// The user sidecar named it.
    User,
    /// Derived from a performance spec (EXT-16).
    Spec,
    /// The default for the devices' role; not binding as a limit.
    Role,
}

/// Which side of a symmetric set a member sits on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Half {
    A,
    B,
}

/// Terminal names, the pin-table columns of `annotator::pattern`: MOS
/// `G D S B`, bipolar `C E` (and `B`), two-terminal `P N`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Term {
    G,
    D,
    S,
    B,
    C,
    E,
    P,
    N,
}

/// How a matched set's units are arranged in its array.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrayStyle {
    /// Each member's units contiguous, members side by side.
    Adjacent,
    /// Members' units alternate along one row.
    Interdigitated,
    /// One row whose members share a centroid.
    CommonCentroid1d,
    /// A 2-D grid whose members share a centroid.
    CommonCentroid2d,
    /// No preference; the cell generator chooses.
    Any,
}

/// One device of a matched set, in units of the set's [`UnitGeom`].
#[derive(Clone, Copy, Debug)]
pub struct Member {
    /// Schematic device id.
    pub device: DeviceId,
    /// Units in parallel (≥ 1).
    pub parallel: u16,
    /// Units in series (≥ 1).
    pub series: u16,
    /// Side of a symmetric set; `None` outside one.
    pub half: Option<Half>,
}

/// The unit every member is built from.
#[derive(Clone, Copy, Debug)]
pub struct UnitGeom {
    /// Unit width, nm.
    pub w_nm: i32,
    /// Unit length, nm.
    pub l_nm: i32,
    /// Device model index, shared by every unit.
    pub model: u16,
}

/// What extraction knows about one matched set. The rule that scores it is MAT-04's
/// `analog::placement::matched_set::MatchedSet`, built from this by EXT-20 (different type, different module).
#[derive(Clone, Debug)]
pub struct MatchSpec {
    /// Stable id of this set.
    pub id: ConstraintId,
    /// Why the set was inferred.
    pub origin: Origin,
    /// At least two devices of one family.
    pub members: Vec<Member>,
    /// A mirror's diode reference or a ratio bank's unit member (index into `members`); MAT-04 puts it in slot 0.
    pub reference: Option<usize>,
    /// Device family of every member.
    pub family: Family,
    /// What the set matches (voltage, current, ratio).
    pub kind: MatchKind,
    /// How tightly it matches.
    pub class: MatchClass,
    /// Who set `class`.
    pub class_source: ClassSource,
    /// `None`: the ratio is not an integer multiple of one unit (CELL/MAT handle it).
    pub unit: Option<UnitGeom>,
    /// 1σ systematic allowance for the whole set (mV for Voltage, % for Current/Ratio), from EXT-21. `None` = not allocated.
    pub allowance: Option<f32>,
    /// Share of the spec variance this set explains, 0..=1; `None` without sensitivities.
    pub weight: Option<f32>,
    /// Requested array arrangement.
    pub style: ArrayStyle,
    /// Index into `Intent.compounds` of the compound holding a member.
    pub compound: Option<u16>,
}

/// How a symmetric pair is drawn about its axis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymKind {
    /// Reflected: one half is the other's mirror image.
    Mirror,
    /// Translated: both halves drawn in the same orientation.
    Perfect,
}

/// Devices symmetric about one axis (EXT-14).
#[derive(Clone, Debug)]
pub struct Compound {
    /// Stable id of this compound.
    pub id: ConstraintId,
    /// The shared symmetry axis.
    pub axis: AxisId,
    /// Axis direction.
    pub dir: AxisDir,
    /// Drawing of the pairs about the axis.
    pub kind: SymKind,
    /// Mirror couples, half A first.
    pub pairs: Vec<(DeviceId, DeviceId)>,
    /// Devices on the axis.
    pub selfs: Vec<DeviceId>,
    /// Mirrored nets, half A first.
    pub net_pairs: Vec<(NetId, NetId)>,
    /// Self-symmetric non-rail nets.
    pub self_nets: Vec<NetId>,
    /// Indices into `Intent.sets` of spec pairs that mirror each other (nested symmetry).
    pub set_pairs: Vec<(u16, u16)>,
}

/// Requirement types in importance order (survey §3.2.1): `MatchSym < MatchBlock < ProxBlock < Sym < ProxNet`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ReqType {
    MatchSym,
    MatchBlock,
    ProxBlock,
    Sym,
    ProxNet,
}

/// What binds the members of a [`GroupNode`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GroupKind {
    /// A matched set.
    Matching,
    /// A symmetry compound.
    Symmetry,
    /// Devices that should sit close together.
    Proximity,
    /// The whole circuit.
    Root,
}

/// One node of the HSMPG tree (EXT-13). `devices` lists only the node's
/// direct device members; `children` index group nodes in `Intent.tree`.
#[derive(Clone, Debug)]
pub struct GroupNode {
    /// What binds the node.
    pub kind: GroupKind,
    /// Direct device members only.
    pub devices: Vec<DeviceId>,
    /// Indices of child nodes in `Intent.tree`.
    pub children: Vec<u32>,
}

/// A placement order (EXT-28, EXT-27 arrays, sidecar `Order`). `steps[0]` sits at the low
/// coordinate of `dir` (bottom for `V`, left for `H`); each step is a set of devices
/// (PLC-25 takes the bbox of a step's devices).
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    /// Device sets, low coordinate first.
    pub steps: Vec<Vec<DeviceId>>,
    /// Axis the steps advance along.
    pub dir: AxisDir,
    /// The sense along `dir` is placement's choice (extracted orders); `false` for a user order.
    pub reversible: bool,
    /// Path current / the largest extracted path current, 0..=1; 1.0 without an op point.
    pub weight: f32,
}

/// A MOS device's operating region at the operating point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    /// No operating point, or not a MOS device.
    Unknown,
    Off,
    Subthreshold,
    Triode,
    Saturation,
}

/// The circuit function extraction assigns a device.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceRole {
    Unknown,
    Amplifier,
    CurrentSource,
    Cascode,
    Load,
    Switch,
    /// Diode-connected (D = G).
    Diode,
    /// Resistor, capacitor or inductor.
    Passive,
}

/// Per-device facts, indexed by `DeviceId` in `Intent.devices`.
#[derive(Clone, Copy, Debug)]
pub struct DeviceFacts {
    pub region: Region,
    pub role: DeviceRole,
}

/// Which parasitics a net is sensitive to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RcClass {
    /// Not determined.
    Unknown,
    /// Insensitive to both.
    None,
    /// Resistance-sensitive (carries current).
    R,
    /// Capacitance-sensitive (high impedance).
    C,
    /// Sensitive to both.
    Rc,
}

/// Which source decided a net's class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EvidenceLevel {
    User,
    Port,
    Name,
    Testbench,
    Structure,
    OpPoint,
    Default,
}

/// Per-net facts, indexed by `NetId` in `Intent.nets`.
#[derive(Clone, Debug)]
pub struct NetFacts {
    /// Source that decided the net's class.
    pub evidence: EvidenceLevel,
    /// The net is a subcircuit port.
    pub port: bool,
    /// DC voltage range `(min, max)`, mV; `None` without an operating point.
    pub dc_mv: Option<(i32, i32)>,
    /// Parasitic sensitivity.
    pub rc: RcClass,
    /// Node impedance, Ω; `None` when unknown.
    pub z_ohm: Option<f32>,
    /// Quiet net a shield around this one ties to.
    pub shield_ref: Option<NetId>,
}

/// How an aggressor injects into the substrate (GAP-03 adds the minority kinds, C4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Inject {
    Switching,
    Capacitive,
    /// n+ diffusion in p-sub on a pin (forward-biased below ground).
    MinorityElectron,
    /// p+ in n-well (above supply).
    MinorityHole,
}

/// A device that injects noise into the substrate.
#[derive(Clone, Debug)]
pub struct Aggressor {
    pub device: DeviceId,
    pub inject: Inject,
    /// Short tag of the evidence, for reports.
    pub reason: &'static str,
}

/// A device sensitive to substrate noise.
#[derive(Clone, Debug)]
pub struct Victim {
    pub device: DeviceId,
    /// Relative sensitivity, larger = more sensitive (a set's spec weight,
    /// else a class rank).
    pub weight: f32,
    /// Short tag of the evidence, for reports.
    pub reason: &'static str,
}

/// A net shared by both halves of a matched set: the `a` and `b` branches
/// must see equal wiring to `term`.
#[derive(Clone, Debug)]
pub struct CommonNodeReq {
    pub net: NetId,
    /// Index into `Intent.sets`.
    pub set: u16,
    /// Half-A devices on the net.
    pub a: Vec<DeviceId>,
    /// Half-B devices on the net.
    pub b: Vec<DeviceId>,
    /// Terminal the devices connect by.
    pub term: Term,
}

/// A net routed as a star: every branch joins at `root`.
#[derive(Clone, Debug)]
pub struct StarReq {
    pub net: NetId,
    /// Star point; `None` when the net is a port (the port is the star point).
    pub root: Option<(DeviceId, Term)>,
    pub branches: Vec<(DeviceId, Term)>,
}

/// A force/sense split: the `sense` pins tap `device`'s `term` directly,
/// not through the force wiring.
#[derive(Clone, Debug)]
pub struct KelvinReq {
    pub device: DeviceId,
    pub term: Term,
    pub sense: Vec<(DeviceId, Term)>,
}

/// A finding extraction reports instead of guessing, e.g. `"ambiguous_symmetry"`.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// Stable machine-readable tag.
    pub kind: &'static str,
    /// Devices the finding concerns.
    pub devices: Vec<DeviceId>,
    /// Human-readable explanation.
    pub message: String,
}

/// Everything extraction infers about one circuit. `nets` and `devices` are
/// indexed by `NetId` / `DeviceId`; the other tables are lists.
#[derive(Clone, Debug, Default)]
pub struct Intent {
    /// Matched sets, in emission order.
    pub sets: Vec<MatchSpec>,
    /// Symmetry compounds.
    pub compounds: Vec<Compound>,
    /// HSMPG tree, `Root` last (EXT-13).
    pub tree: Vec<GroupNode>,
    /// User orders, current paths, signal stages, then instance arrays (EXT-27/28).
    pub order: Vec<Order>,
    /// One per net.
    pub nets: Vec<NetFacts>,
    /// One per device.
    pub devices: Vec<DeviceFacts>,
    pub aggressors: Vec<Aggressor>,
    pub victims: Vec<Victim>,
    pub common_nodes: Vec<CommonNodeReq>,
    pub stars: Vec<StarReq>,
    pub kelvins: Vec<KelvinReq>,
    pub diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_class_orders() {
        assert!(MatchClass::Minimal < MatchClass::Moderate && MatchClass::Moderate < MatchClass::Exceptional);
        assert_eq!(MatchClass::default(), MatchClass::Moderate);
    }

    #[test]
    fn req_type_importance() {
        assert!(
            ReqType::MatchSym < ReqType::MatchBlock
                && ReqType::MatchBlock < ReqType::ProxBlock
                && ReqType::ProxBlock < ReqType::Sym
                && ReqType::Sym < ReqType::ProxNet
        );
    }
}
