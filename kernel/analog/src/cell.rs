//! Cell-tier structural directives, read cold by `cells` (not scored per move).

use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;

/// A latch-up guard ring around `device`, tied to `connection_net`.
#[derive(Clone, Copy)]
pub struct GuardRingRequirement {
    /// Schematic device the ring encloses.
    pub device: DeviceId,
    /// Diffusion and well stack of the ring.
    pub ring_type: GuardRingType,
    /// May merge with a neighbouring ring of the same class.
    pub shareable: bool,
    /// Min ring metal width, nm.
    pub min_width_nm: i32,
    /// Max ring resistance, mΩ.
    pub max_ring_resistance_mohm: i64,
    /// Net the ring taps (substrate/well rail).
    pub connection_net: NetId,
    /// Why the ring exists; rings of different roles never merge (post_cell).
    pub role: RingRole,
}

/// What a guard ring protects against (REL-07): an injector's minority
/// carriers, an aggressor's majority-carrier noise, or a victim's exposure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RingRole {
    /// Encloses a device that can inject minority carriers (a forward-biased junction).
    Injector,
    /// Encloses a noisy device to collect its majority-carrier substrate current.
    Aggressor,
    /// Encloses a sensitive device to shield it from substrate noise.
    Victim,
}

/// The diffusion/well construction of a guard ring.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuardRingType {
    /// Majority-carrier tap ring: p+ in substrate, or n+ in n-well when
    /// `in_well` (the device's own bulk tap).
    Tap { in_well: bool },
    /// Electron-collecting: n+ ring in its own n-well band, tied to a
    /// supply, collects electrons (the band well is drawn separately).
    Ecgr,
    /// User-declared isolated tub (GAP-14): n+ ring in an n-well band over a deep n-well, tied to a quiet supply;
    /// the devices' bulk is the isolated p-well. `id` keeps distinct tubs from merging (distinct p-wells).
    Tub { id: u16 },
    /// Hole-collecting: p+ collecting ring, needs a retrograde/isolated well
    /// the deck declares.
    Hcgr,
}

/// Decompose `devices` into identical unit fingers/segments so ratios are
/// exact integers. Every instance shares the unit geometry (and, for FETs, the
/// variant). The layout pattern is chosen by the cell generator.
#[derive(Clone)]
pub struct Unitization {
    /// Members of the set, schematic ids; every per-member `Vec` below is parallel to it.
    pub devices: Vec<DeviceId>,
    /// Device kind shared by every member.
    pub device_type: DeviceKind,
    /// Unit count per device.
    pub dev_nf: Vec<u16>,
    /// Integer ratio between instances, e.g. `[1, 4]`.
    pub target_ratio: Vec<u16>,
    /// Unit width, nm.
    pub unit_w: i32,
    /// Unit length, nm.
    pub unit_l: i32,
    /// How a member's units compose.
    pub series_parallel: SeriesParallel,
    /// Dummy units must flank the array.
    pub dummy_required: bool,
    /// Members' connections must be routed alike.
    pub route_matching_required: bool,
    /// Match class of the set (EXT-16 from a spec, or the user); `None` = not given, matched-cell readers use
    /// `unwrap_or(Moderate)` (C16). Read by `cells::cap_array` (GAP-18): an Exceptional binary bank lists its
    /// variants best-matching first.
    pub class: Option<crate::intent::MatchClass>,
    /// Match kind of the set (EXT-15/16 from the set); None = unknown, no aspect limit (GAP-11).
    pub kind: Option<crate::intent::MatchKind>,
    /// Per member; empty = all 1.
    pub series: Vec<u16>,
    /// Requested array arrangement; `None` = the cell generator's default choice.
    pub style: Option<crate::intent::ArrayStyle>,
}

/// How units compose into one instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesParallel {
    /// Units in parallel: more units, more current (W adds).
    Parallel,
    /// Units in series: more units, longer device (L or R adds).
    Series,
}
