//! Analog primitive pattern catalog.
//!
//! 120+ structural patterns for automatic netlist annotation.
//! Derived from ALIGN's basic_template library, Razavi/Allen-Holberg/
//! Gray-Meyer canonical topologies, and the GANA primitive taxonomy.
//!
//! Add a new pattern: define a `const Pattern` here, add it to `PATTERNS`.
//! The engine in `pattern.rs` picks it up automatically — no switch
//! statements, no registration, no match arms.
//!
//! ## Priority scheme
//!
//! Larger patterns (more devices) get higher priority so they match first,
//! preventing their sub-structures from being consumed as smaller patterns.
//!
//!   8-device composites : 50–59
//!   6-device composites : 40–49
//!   5-device composites : 30–39
//!   4-device composites : 20–29
//!   3-device patterns   : 14–19
//!   2-device patterns   :  5–13
//!
//! Within a size tier, more-constrained patterns (exact sizing, diode
//! requirements) get higher priority than loosely-constrained ones.

use crate::pattern::{DiodeReq, Pattern, PinLink, PinRel, SizeMatch, Slot, SlotKind};

// ── Shorthand constructors ──

const S_ANY: Slot = Slot {
    kind: SlotKind::AnyFet,
    size_match: SizeMatch::Any,
    diode: DiodeReq::Any,
    gate_is_signal: false,
};

/// Same type as slot 0, exact W+L match, no diode constraint.
const fn same0_exact() -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(0),
        size_match: SizeMatch::ExactAs(0),
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Same type as slot 0, same L only.
const fn same0_samel() -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(0),
        size_match: SizeMatch::SameLAs(0),
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Same type as slot 0, any sizing.
const fn same0() -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(0),
        size_match: SizeMatch::Any,
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Same type as a given slot, any sizing.
const fn same_as(r: u8) -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(r),
        size_match: SizeMatch::Any,
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Same type as a given slot, exact W+L match with that slot.
const fn same_exact(r: u8) -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(r),
        size_match: SizeMatch::ExactAs(r),
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Same type as a given slot, same L with that slot.
const fn same_samel(r: u8) -> Slot {
    Slot {
        kind: SlotKind::SameTypeAs(r),
        size_match: SizeMatch::SameLAs(r),
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Complement (NMOS<->PMOS) of a given slot.
const fn comp(r: u8) -> Slot {
    Slot {
        kind: SlotKind::ComplementOf(r),
        size_match: SizeMatch::Any,
        diode: DiodeReq::Any,
        gate_is_signal: false,
    }
}

/// Shorthand link: two pins on the same net.
const fn eq(a: u8, pa: &'static str, b: u8, pb: &'static str) -> PinLink {
    PinLink { a, pin_a: pa, b, pin_b: pb, rel: PinRel::Same }
}

/// Shorthand link: two pins on different nets.
const fn ne(a: u8, pa: &'static str, b: u8, pb: &'static str) -> PinLink {
    PinLink { a, pin_a: pa, b, pin_b: pb, rel: PinRel::Diff }
}

// ═══════════════════════════════════════════════════════════════════════
//  Category 1: Two-device transistor-level primitives (priority 5–13)
// ═══════════════════════════════════════════════════════════════════════

// ── 1.1 Differential pairs ──

/// Classic differential pair: same-type, exact W+L match, shared source,
/// different gates (signal inputs), different drains.
/// Explicitly excludes cross-coupled configurations (G_A != D_B).
/// Matches both NMOS and PMOS variants.
/// ALIGN: DP_NMOS / DP_PMOS
pub const DIFF_PAIR: Pattern = Pattern {
    name: "diff_pair",
    priority: 10,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        ne(0, "G", 1, "D"),  // exclude cross-coupled
        ne(1, "G", 0, "D"),  // exclude cross-coupled
    ],
};

/// Differential pair with separate sources (e.g. degenerated or with
/// individual source resistors). Same gates-differ, drains-differ,
/// but sources also differ.
/// ALIGN: DP with split sources (CMC_S variant wiring)
pub const DIFF_PAIR_SPLIT_SOURCE: Pattern = Pattern {
    name: "diff_pair_split_source",
    priority: 9,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
    ],
    links: &[
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        ne(0, "S", 1, "S"),
    ],
};

// ── 1.2 Current mirrors ──

/// Simple current mirror: diode-connected reference + output transistor.
/// Shared gate and source, reference is diode-connected.
/// ALIGN: SCM_NMOS / SCM_PMOS
pub const CURRENT_MIRROR: Pattern = Pattern {
    name: "current_mirror",
    priority: 8,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
    ],
};

/// Current mirror output pair (no diode): two transistors sharing gate
/// and source, both driven by an external bias. Neither is diode-connected.
/// ALIGN: CMC_NMOS / CMC_PMOS
pub const CURRENT_MIRROR_OUTPUT_PAIR: Pattern = Pattern {
    name: "current_mirror_output_pair",
    priority: 7,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

/// Current mirror output pair with split sources. Two same-type FETs,
/// shared gate, separate sources and drains. Used in cascode mirrors
/// where the bottom pair has distinct source routing.
/// ALIGN: CMC_S_NMOS / CMC_S_PMOS
pub const MIRROR_PAIR_SPLIT_SOURCE: Pattern = Pattern {
    name: "mirror_pair_split_source",
    priority: 6,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        ne(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

// ── 1.3 Cross-coupled pairs ──

/// Cross-coupled pair: gate of A = drain of B and vice versa.
/// Forms positive feedback (latch). Shared source.
/// ALIGN: CCP_NMOS / CCP_PMOS
pub const CROSS_COUPLED: Pattern = Pattern {
    name: "cross_coupled",
    priority: 9,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::ExactAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "D"),
        eq(0, "D", 1, "G"),
        eq(0, "S", 1, "S"),
    ],
};

/// Cross-coupled pair with split sources: gate-drain cross but sources
/// on different nets (e.g. in SRAM cells or sense amps).
/// ALIGN: CCP_S_NMOS / CCP_S_PMOS
pub const CROSS_COUPLED_SPLIT_SOURCE: Pattern = Pattern {
    name: "cross_coupled_split_source",
    priority: 9,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::ExactAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "D"),
        eq(0, "D", 1, "G"),
        ne(0, "S", 1, "S"),
    ],
};

// ── 1.4 Cascode structures ──

/// Simple cascode: drain of bottom connects to source of top.
/// Same type. Used for high output impedance.
pub const CASCODE: Pattern = Pattern {
    name: "cascode",
    priority: 7,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "S"),
    ],
};

/// Cascode with same L constraint. Bottom and top share channel length
/// (typical in matched cascode stacks).
pub const CASCODE_MATCHED: Pattern = Pattern {
    name: "cascode_matched",
    priority: 8,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "S"),
        ne(0, "G", 1, "G"),
    ],
};

// ── 1.5 Active loads / matched pairs ──

/// Active load pair: same-type, exact match, shared gate+source,
/// different drains, neither diode-connected. Biased by external voltage.
/// ALIGN: CMC with non-signal gate
pub const ACTIVE_LOAD: Pattern = Pattern {
    name: "active_load",
    priority: 6,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

/// Diode-connected load pair: two diode-connected FETs sharing source,
/// different drains. Used as matched diode loads in DAC reference ladders.
pub const DIODE_LOAD_PAIR: Pattern = Pattern {
    name: "diode_load_pair",
    priority: 8,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

// ── 1.6 Source follower / common drain ──

/// Source follower pair: two same-type FETs where drain of bottom
/// connects to source of top AND top has signal gate input.
/// The top FET is the follower, bottom is the current source.
pub const SOURCE_FOLLOWER: Pattern = Pattern {
    name: "source_follower",
    priority: 8,
    slots: &[
        S_ANY, // current source (bottom)
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Forbidden,
            gate_is_signal: true,
        }, // follower (top)
    ],
    links: &[
        eq(0, "D", 1, "S"), // current source drain = follower source
    ],
};

// ── 1.7 Complementary pairs ──

/// CMOS inverter: complementary pair sharing gate and drain.
/// ALIGN: inv
pub const CMOS_INVERTER: Pattern = Pattern {
    name: "cmos_inverter",
    priority: 11,
    slots: &[
        S_ANY,
        comp(0),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "D", 1, "D"),
    ],
};

/// Transmission gate: complementary pair sharing drain and source
/// but with different gates (complementary clock signals).
/// ALIGN: tgate
pub const TRANSMISSION_GATE: Pattern = Pattern {
    name: "transmission_gate",
    priority: 11,
    slots: &[
        S_ANY,
        comp(0),
    ],
    links: &[
        eq(0, "D", 1, "D"),
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
    ],
};

/// Complementary common-source pair: NMOS and PMOS sharing gate,
/// different drains and sources. Push-pull output stage element.
pub const PUSH_PULL_PAIR: Pattern = Pattern {
    name: "push_pull_pair",
    priority: 10,
    slots: &[
        S_ANY,
        comp(0),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        ne(0, "S", 1, "S"),
    ],
};

// ── 1.8 Latch / self-biased structures ──

/// Latch half: M0 is diode-connected (gate=drain), M0's drain drives
/// M1's gate, separate sources. Used in sense amp latches.
/// ALIGN: LS_S_NMOS / LS_S_PMOS
pub const LATCH_HALF: Pattern = Pattern {
    name: "latch_half",
    priority: 8,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::ExactAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "G"), // diode drain drives mirror gate
        ne(0, "S", 1, "S"), // separate sources
        ne(0, "D", 1, "D"), // separate drains
    ],
};

// ── 1.9 Series-connected FETs ──

/// Series stack: same-type, source of top = drain of bottom,
/// SAME gate (series combination for effective L doubling or
/// high-voltage tolerance). Different from cascode where gates differ.
pub const SERIES_STACK: Pattern = Pattern {
    name: "series_stack",
    priority: 6,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::ExactAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(0, "G", 1, "G"),
    ],
};

/// Degeneration pair: two same-type FETs sharing drain but with
/// different gates and different sources. Used when a degeneration
/// resistor is replaced by a FET in triode region.
pub const DEGENERATION_PAIR: Pattern = Pattern {
    name: "degeneration_pair",
    priority: 5,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "D"),
        ne(0, "G", 1, "G"),
        ne(0, "S", 1, "S"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 2: Three-device patterns (priority 14–19)
// ═══════════════════════════════════════════════════════════════════════

// ── 2.1 Mirrors with 3 devices ──

/// Wilson current mirror: M0 diode ref, M1 mirror, M2 cascode with
/// feedback from M2's gate to M0's drain.
/// Topology: M0+M1 share gate+source; M1.D = M2.S; M0.D = M2.G.
pub const WILSON_MIRROR: Pattern = Pattern {
    name: "wilson_mirror",
    priority: 16,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(1, "D", 2, "S"),
        eq(0, "D", 2, "G"),
    ],
};

/// Improved Wilson mirror: like Wilson but M2 is diode-connected and
/// M0 is NOT diode-connected. M2.D = M2.G provides the feedback,
/// M0 and M1 share gate (driven by M2), share source.
/// Gives better output impedance than basic Wilson.
pub const IMPROVED_WILSON_MIRROR: Pattern = Pattern {
    name: "improved_wilson_mirror",
    priority: 17,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY }, // M0: input, not diode
        same0_samel(),                                  // M1: output, same L
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required, // M2: feedback diode
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),  // M0 M1 share gate
        eq(0, "S", 1, "S"),  // M0 M1 share source
        eq(0, "D", 2, "S"),  // M0.D = M2.S
        eq(2, "D", 0, "G"),  // M2.D(=M2.G) drives M0+M1 gates
    ],
};

/// Three-output current mirror: diode ref + 2 output transistors.
/// All three share gate and source. M0 is diode-connected.
pub const CURRENT_MIRROR_3: Pattern = Pattern {
    name: "current_mirror_3",
    priority: 14,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        same0_samel(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "G", 2, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        ne(0, "D", 1, "D"),
        ne(0, "D", 2, "D"),
        ne(1, "D", 2, "D"),
    ],
};

// ── 2.2 Diff pair combinations ──

/// Diff pair with tail current source.
/// M0+M1 form diff pair, M2 provides tail current (its drain = shared source).
pub const DIFF_PAIR_WITH_TAIL: Pattern = Pattern {
    name: "diff_pair_with_tail",
    priority: 18,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot { kind: SlotKind::SameTypeAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(0, "S", 2, "D"),
    ],
};

/// Diff pair with diode-connected load on one side.
/// M0+M1 diff pair, M2 diode load on M0's drain (complementary type).
pub const DIFF_PAIR_DIODE_LOAD: Pattern = Pattern {
    name: "diff_pair_diode_load",
    priority: 15,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(0, "D", 2, "D"),
    ],
};

// ── 2.3 Cascode combinations ──

/// Regulated cascode: M0 common-source, M1 cascode on top,
/// M2 amplifier providing feedback to M1's gate.
/// M0.D = M1.S, M0.D = M2.G (sense), M2.D = M1.G (regulate).
pub const REGULATED_CASCODE: Pattern = Pattern {
    name: "regulated_cascode",
    priority: 17,
    slots: &[
        S_ANY,         // M0: bottom CS
        same0(),       // M1: cascode (top)
        same0(),       // M2: regulation amp
    ],
    links: &[
        eq(0, "D", 1, "S"),  // cascode stack
        eq(0, "D", 2, "G"),  // M2 senses M0 drain voltage
        eq(2, "D", 1, "G"),  // M2 output drives cascode gate
    ],
};

/// Source-degenerated cascode: M0 bottom, M1 top cascode,
/// M2 source degeneration (gate tied to supply/bias, in triode).
/// M2.D = M0.S, M0.D = M1.S.
pub const CASCODE_WITH_DEGENERATION: Pattern = Pattern {
    name: "cascode_with_degeneration",
    priority: 15,
    slots: &[
        S_ANY,
        same0(),
        same0(),
    ],
    links: &[
        eq(0, "D", 1, "S"),  // cascode stack
        eq(2, "D", 0, "S"),  // degeneration device under bottom
    ],
};

// ── 2.4 Source follower with bias ──

/// Source follower with current mirror bias: M0 is the follower (signal gate),
/// M1 is a diode-connected current source, M2 mirrors M1.
/// M0.S = M2.D (current source output), M1.G = M2.G (mirror gate).
pub const SOURCE_FOLLOWER_WITH_MIRROR: Pattern = Pattern {
    name: "source_follower_with_mirror",
    priority: 15,
    slots: &[
        Slot {
            kind: SlotKind::AnyFet,
            size_match: SizeMatch::Any,
            diode: DiodeReq::Forbidden,
            gate_is_signal: true,
        },
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(1), size_match: SizeMatch::SameLAs(1), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 2, "D"),
        eq(1, "G", 2, "G"),
        eq(1, "S", 2, "S"),
    ],
};

/// Flipped voltage follower: M0 (input), M1 (current source),
/// M2 (feedback amplifier). M0.S is the output, M1 forces constant
/// current. M1.D = M0.S, M2.G = M0.S (sense), M2.D = M1.G (feedback).
pub const FLIPPED_VOLTAGE_FOLLOWER: Pattern = Pattern {
    name: "flipped_voltage_follower",
    priority: 16,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY }, // M0: input device
        same0(),                                  // M1: current source
        same0(),                                  // M2: feedback amp
    ],
    links: &[
        eq(1, "D", 0, "S"),  // M1 drives M0's source
        eq(0, "S", 2, "G"),  // M2 senses output (M0.S)
        eq(2, "D", 1, "G"),  // M2 regulates M1's gate
    ],
};

// ── 2.5 Three-output structures ──

/// Three-output active load: three same-type FETs, all sharing gate
/// and source, different drains. None diode-connected.
pub const ACTIVE_LOAD_3: Pattern = Pattern {
    name: "active_load_3",
    priority: 14,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "G", 2, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        ne(0, "D", 1, "D"),
        ne(0, "D", 2, "D"),
        ne(1, "D", 2, "D"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 3: Four-device patterns (priority 20–29)
// ═══════════════════════════════════════════════════════════════════════

// ── 3.1 Cascode mirrors ──

/// Cascode current mirror (standard): M0 diode + M1 output form the
/// bottom mirror, M2 diode + M3 cascode form the top pair.
/// M0.D = M2.S, M1.D = M3.S, M2.G = M3.G.
pub const CASCODE_MIRROR: Pattern = Pattern {
    name: "cascode_mirror",
    priority: 24,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },          // M0: bottom ref (diode)
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), ..S_ANY }, // M1: bottom out
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY }, // M2: top ref (diode)
        same0(),                                                // M3: top out
    ],
    links: &[
        eq(0, "G", 1, "G"),  // bottom pair share gate
        eq(0, "S", 1, "S"),  // bottom pair share source
        eq(0, "D", 2, "S"),  // M0 stacks under M2
        eq(1, "D", 3, "S"),  // M1 stacks under M3
        eq(2, "G", 3, "G"),  // top pair share gate
    ],
};

/// Wide-swing cascode mirror: like cascode mirror but the top gate bias
/// is derived differently. M0 diode, M1 output, M2 cascode (diode),
/// M3 cascode output. The key difference is M2's gate connects to M0's
/// drain (not to M3's gate independently) providing wide output swing.
/// M2.G = M0.D = M2.S is what makes it "wide-swing" — the diode cascode
/// self-biases at VGS above the bottom mirror.
pub const WIDE_SWING_CASCODE_MIRROR: Pattern = Pattern {
    name: "wide_swing_cascode_mirror",
    priority: 25,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),  // M0 drain = M2 source
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(2, "D", 0, "G"),  // M2.D feeds back to bottom mirror gate (wide-swing bias)
    ],
};

/// Low-voltage cascode mirror: M0+M1 bottom pair, M2+M3 top pair.
/// Bottom gates are driven by the cascode voltage (M2 or M3 drain).
/// Allows operation at lower supply than standard cascode.
/// M0.G = M1.G (shared), M2.G = M3.G, M0.D=M2.S, M1.D=M3.S.
/// The distinction: neither M2 nor M3 needs to be diode-connected.
pub const LOW_VOLTAGE_CASCODE_MIRROR: Pattern = Pattern {
    name: "low_voltage_cascode_mirror",
    priority: 23,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
    ],
};

// ── 3.2 Wilson mirrors (4-device) ──

/// Four-transistor Wilson mirror: M0+M1 bottom pair (cross-coupled gates
/// to drains), M2+M3 top cascode pair.
/// M0.G = M3.D, M1.G = M2.D, M0.D = M2.S, M1.D = M3.S.
pub const WILSON_MIRROR_4: Pattern = Pattern {
    name: "wilson_mirror_4",
    priority: 24,
    slots: &[
        S_ANY,
        same0_samel(),
        same0(),
        same0(),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(0, "G", 3, "D"),  // cross-feedback
        eq(1, "G", 2, "D"),  // cross-feedback
    ],
};

/// Improved Wilson mirror (4T): M0+M1 bottom, M2+M3 top.
/// M2 is diode-connected. M0.G = M1.G = M2.D.
/// M0.D = M2.S, M1.D = M3.S, M2.G = M3.G.
pub const IMPROVED_WILSON_MIRROR_4: Pattern = Pattern {
    name: "improved_wilson_mirror_4",
    priority: 25,
    slots: &[
        S_ANY,
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(2, "D", 0, "G"),  // M2 diode drives bottom gates
    ],
};

// ── 3.3 Diff pair with loads ──

/// Diff pair + active load: M0+M1 diff pair (same type, signal gates),
/// M2+M3 active load (complement type, shared gate, biased).
pub const DIFF_PAIR_WITH_ACTIVE_LOAD: Pattern = Pattern {
    name: "diff_pair_with_active_load",
    priority: 22,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
        eq(2, "G", 3, "G"),
    ],
};

/// Diff pair + current mirror load: M0+M1 diff pair, M2 diode load
/// on one drain, M3 mirrors M2 on the other drain.
/// This is the classic single-ended output diff amp.
pub const DIFF_PAIR_WITH_MIRROR_LOAD: Pattern = Pattern {
    name: "diff_pair_with_mirror_load",
    priority: 23,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "D"),  // M2 diode on M0's drain
        eq(1, "D", 3, "D"),  // M3 mirror on M1's drain
        eq(2, "G", 3, "G"),  // mirror gate tie
        eq(2, "S", 3, "S"),  // mirror source tie
    ],
};

// ── 3.4 Cross-coupled structures ──

/// Cross-coupled inverter pair (SRAM latch core): two CMOS inverters
/// where output of each drives input of the other.
/// M0(N)+M2(P) = inv1, M1(N)+M3(P) = inv2.
/// M0.D = M2.D = M1.G = M3.G, M1.D = M3.D = M0.G = M2.G.
pub const CROSS_COUPLED_INVERTERS: Pattern = Pattern {
    name: "cross_coupled_inverters",
    priority: 26,
    slots: &[
        S_ANY,
        same0_exact(),
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 2, "D"),  // inv1 output
        eq(1, "D", 3, "D"),  // inv2 output
        eq(0, "G", 2, "G"),  // inv1 input
        eq(1, "G", 3, "G"),  // inv2 input
        eq(0, "D", 1, "G"),  // cross-couple 1
        eq(1, "D", 0, "G"),  // cross-couple 2
    ],
};

/// Cross-coupled diff pair with shared source: M0+M1 cross-coupled
/// (same type), M2+M3 cross-coupled (complement type). All share
/// respective sources. Used in VCO/oscillator cores.
pub const CROSS_COUPLED_COMPLEMENTARY: Pattern = Pattern {
    name: "cross_coupled_complementary",
    priority: 25,
    slots: &[
        S_ANY,
        same0_exact(),
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "D"),
        eq(0, "D", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(2, "G", 3, "D"),
        eq(2, "D", 3, "G"),
        eq(2, "S", 3, "S"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
    ],
};

// ── 3.5 Cascode pairs ──

/// Matched cascode pair: two independent cascode stacks (4 FETs total)
/// where the bottom pair shares gate/source and the top pair shares gate.
/// This is the core of a cascode amplifier stage.
/// M0+M2 = stack A (M0 bottom, M2 top), M1+M3 = stack B.
pub const CASCODE_PAIR: Pattern = Pattern {
    name: "cascode_pair",
    priority: 22,
    slots: &[
        S_ANY,                     // M0: bottom A
        same0_exact(),             // M1: bottom B
        same0(),                   // M2: top A
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),  // bottom pair share gate
        eq(0, "S", 1, "S"),  // bottom pair share source
        eq(0, "D", 2, "S"),  // stack A
        eq(1, "D", 3, "S"),  // stack B
        eq(2, "G", 3, "G"),  // top pair share gate
        ne(2, "D", 3, "D"),  // different outputs
    ],
};

/// Symmetric cascode pair with split sources on top: bottom pair shares
/// gate+source, top pair shares gate but has separate outputs AND sources
/// coming from different bottom devices.
pub const CASCODE_PAIR_SYMMETRIC: Pattern = Pattern {
    name: "cascode_pair_symmetric",
    priority: 21,
    slots: &[
        S_ANY,
        same0_exact(),
        same0(),
        same_exact(2),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        ne(0, "G", 2, "G"),
    ],
};

// ── 3.6 Level shifter ──

/// Level shifter: M0 (input, signal gate) stacked on M1 (current source),
/// M2 (complementary, diode load) stacked on M3 (supply).
/// M0.D = M2.D (shared output node). Used to shift DC level between stages.
pub const LEVEL_SHIFTER: Pattern = Pattern {
    name: "level_shifter",
    priority: 20,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY }, // M0: input device
        same0(),                                  // M1: current source
        comp(0),                                  // M2: diode load (complement)
        Slot { kind: SlotKind::SameTypeAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "D"),  // M0 stacks on M1
        eq(0, "D", 2, "D"),  // shared output node
        eq(2, "S", 3, "D"),  // M2 stacks on M3
    ],
};

// ── 3.7 Charge pump ──

/// Basic charge pump cell: M0 (PMOS switch, up), M1 (current source up),
/// M2 (NMOS switch, down), M3 (current source down).
/// M0.D = M2.D (output), M0.S = M1.D (up current), M2.S = M3.D (down current).
pub const CHARGE_PUMP_CELL: Pattern = Pattern {
    name: "charge_pump_cell",
    priority: 22,
    slots: &[
        S_ANY,    // M0: up switch
        same0(),  // M1: up current source
        comp(0),  // M2: down switch
        Slot { kind: SlotKind::SameTypeAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 2, "D"),  // shared output
        eq(0, "S", 1, "D"),  // up path
        eq(2, "S", 3, "D"),  // down path
        ne(0, "G", 2, "G"),  // complementary clocks
    ],
};

// ── 3.8 Four-output mirror ──

/// Four-output current mirror: diode ref + 3 matched outputs.
pub const CURRENT_MIRROR_4: Pattern = Pattern {
    name: "current_mirror_4",
    priority: 20,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        same0_samel(),
        same0_samel(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "G", 2, "G"),
        eq(0, "G", 3, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        eq(0, "S", 3, "S"),
        ne(0, "D", 1, "D"),
        ne(0, "D", 2, "D"),
        ne(0, "D", 3, "D"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 4: Five-device patterns (priority 30–39)
// ═══════════════════════════════════════════════════════════════════════

/// Five-transistor OTA: M0+M1 diff pair, M2+M3 active load (mirror load),
/// M4 tail current source.
/// Classic topology from Razavi Ch.9 / Allen-Holberg Ch.6.
pub const FIVE_TRANSISTOR_OTA: Pattern = Pattern {
    name: "five_transistor_ota",
    priority: 34,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },  // M0: diff pair A
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },                                         // M1: diff pair B
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },                                         // M2: diode load
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY }, // M3: mirror load
        same0(),                                    // M4: tail
    ],
    links: &[
        eq(0, "S", 1, "S"),  // diff pair shared source
        ne(0, "G", 1, "G"),  // different inputs
        eq(0, "D", 2, "D"),  // M2 loads M0
        eq(1, "D", 3, "D"),  // M3 loads M1
        eq(2, "G", 3, "G"),  // mirror gate
        eq(2, "S", 3, "S"),  // mirror source
        eq(0, "S", 4, "D"),  // tail drives diff pair
    ],
};

/// Diff pair + cascode load: M0+M1 diff pair, M2+M3 cascode load pair
/// (diff pair drains connect to cascode SOURCES), M4 tail.
/// Used in high-gain single-stage amplifiers. The key distinction from
/// an active load is the drain-to-source (cascode) connection.
pub const DIFF_PAIR_CASCODE_LOAD: Pattern = Pattern {
    name: "diff_pair_cascode_load",
    priority: 33,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),                                          // M2: cascode load A
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
        same0(),                                           // M4: tail
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),  // drain-to-source = cascode connection
        eq(1, "D", 3, "S"),  // drain-to-source = cascode connection
        eq(2, "G", 3, "G"),
        eq(0, "S", 4, "D"),
    ],
};

/// Diff pair with tail and cross-coupled load: M0+M1 diff pair,
/// M2+M3 cross-coupled load (complement), M4 tail.
pub const DIFF_PAIR_CROSS_COUPLED_LOAD: Pattern = Pattern {
    name: "diff_pair_cross_coupled_load",
    priority: 35,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
        same0(),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
        eq(2, "G", 3, "D"),  // cross-coupled
        eq(2, "D", 3, "G"),  // cross-coupled
        eq(2, "S", 3, "S"),
        eq(0, "S", 4, "D"),
    ],
};

/// Wide-swing cascode mirror (5T): standard cascode mirror (4T) plus
/// a bias device. M0 diode, M1 output, M2 cascode diode, M3 cascode out,
/// M4 bias (diode, generates bottom gate voltage).
pub const WIDE_SWING_CASCODE_MIRROR_5: Pattern = Pattern {
    name: "wide_swing_cascode_mirror_5",
    priority: 30,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
        same0(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(4, "G", 0, "G"),  // M4 bias shares bottom gate
        eq(4, "S", 0, "S"),  // M4 shares source with bottom
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 5: Six-device patterns (priority 40–49)
// ═══════════════════════════════════════════════════════════════════════

/// Telescopic OTA: M0+M1 diff pair, M2+M3 NMOS cascodes on diff pair,
/// M4+M5 PMOS cascode loads.
/// Topology: M0.D=M2.S, M1.D=M3.S (input cascodes), M2.D=M4.D, M3.D=M5.D.
pub const TELESCOPIC_OTA_CORE: Pattern = Pattern {
    name: "telescopic_ota_core",
    priority: 42,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },  // M0: diff A
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },                                         // M1: diff B
        same0(),                                    // M2: cascode A
        same_exact(2),                              // M3: cascode B
        comp(0),                                    // M4: load A
        Slot { kind: SlotKind::SameTypeAs(4), size_match: SizeMatch::ExactAs(4), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(2, "D", 4, "D"),
        eq(3, "D", 5, "D"),
        eq(4, "G", 5, "G"),
        eq(4, "S", 5, "S"),
    ],
};

/// Folded cascode core: M0+M1 diff pair (e.g. PMOS), M2+M3 folding
/// devices (NMOS, cascode), M4+M5 current sources providing fold current.
/// M0.D = M2.S (fold point A), M1.D = M3.S (fold point B).
/// M4.D = M2.S, M5.D = M3.S (current injection into fold).
pub const FOLDED_CASCODE_CORE: Pattern = Pattern {
    name: "folded_cascode_core",
    priority: 43,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),                                    // M2: fold cascode A
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
        comp(0),                                    // M4: fold current src A
        Slot { kind: SlotKind::SameTypeAs(4), size_match: SizeMatch::ExactAs(4), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),  // fold point A
        eq(1, "D", 3, "S"),  // fold point B
        eq(2, "G", 3, "G"),  // cascode gate
        eq(4, "D", 2, "S"),  // current injection A
        eq(5, "D", 3, "S"),  // current injection B
        eq(4, "G", 5, "G"),
        eq(4, "S", 5, "S"),
    ],
};

/// Gilbert cell multiplier core: M0+M1 bottom diff pair (RF input),
/// M2+M3 top diff pair A (LO), M4+M5 top diff pair B (LO complement).
/// M0.D = shared source of M2+M3, M1.D = shared source of M4+M5.
/// M2.D = M5.D (IF+), M3.D = M4.D (IF-).
pub const GILBERT_CELL: Pattern = Pattern {
    name: "gilbert_cell",
    priority: 44,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },  // M0: RF+
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },                                         // M1: RF-
        same0_exact(),                              // M2: LO quad A+
        same0_exact(),                              // M3: LO quad A-
        same0_exact(),                              // M4: LO quad B+
        same0_exact(),                              // M5: LO quad B-
    ],
    links: &[
        eq(0, "S", 1, "S"),   // bottom pair source
        ne(0, "G", 1, "G"),   // RF inputs differ
        eq(0, "D", 2, "S"),   // M0 feeds quad A
        eq(0, "D", 3, "S"),
        eq(1, "D", 4, "S"),   // M1 feeds quad B
        eq(1, "D", 5, "S"),
        eq(2, "G", 3, "G"),   // quad A shared LO (actually differ for proper Gilbert)
        eq(4, "G", 5, "G"),   // quad B shared LO
        eq(2, "D", 5, "D"),   // cross-connect outputs
        eq(3, "D", 4, "D"),
    ],
};

/// Complementary cross-coupled VCO core: two NMOS cross-coupled +
/// two PMOS cross-coupled, forming LC-VCO negative resistance.
/// M0+M1 NMOS cross-coupled, M2+M3 PMOS cross-coupled,
/// M4+M5 optional tail current sources (one per type).
pub const VCO_CORE_WITH_TAILS: Pattern = Pattern {
    name: "vco_core_with_tails",
    priority: 44,
    slots: &[
        S_ANY,
        same0_exact(),
        comp(0),
        same_exact(2),
        same0(),     // M4: NMOS tail
        same_as(2),  // M5: PMOS tail
    ],
    links: &[
        eq(0, "G", 1, "D"),
        eq(0, "D", 1, "G"),
        eq(2, "G", 3, "D"),
        eq(2, "D", 3, "G"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
        eq(0, "S", 4, "D"),  // NMOS tail
        eq(1, "S", 4, "D"),
        eq(2, "S", 5, "D"),  // PMOS tail
        eq(3, "S", 5, "D"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 6: Eight-device patterns (priority 50–59)
// ═══════════════════════════════════════════════════════════════════════

/// Full telescopic OTA: 6-device telescopic core + 2 tail devices.
/// M0+M1 diff pair, M2+M3 input cascodes, M4+M5 load cascodes,
/// M6 tail current source, M7 load bias (diode-connected).
pub const TELESCOPIC_OTA_FULL: Pattern = Pattern {
    name: "telescopic_ota_full",
    priority: 52,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        same0(),
        same_exact(2),
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(4), size_match: SizeMatch::ExactAs(4), ..S_ANY },
        same0(),                    // M6: tail
        Slot {                      // M7: load cascode bias
            kind: SlotKind::SameTypeAs(4),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Any,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(2, "D", 4, "D"),
        eq(3, "D", 5, "D"),
        eq(4, "G", 5, "G"),
        eq(4, "S", 5, "S"),
        eq(0, "S", 6, "D"),  // tail
        eq(4, "S", 7, "S"),  // load bias shares supply
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 7: Additional two-device variants for completeness
// ═══════════════════════════════════════════════════════════════════════

// These catch common wiring patterns not covered by the primary set.

/// Common-gate pair: two same-type FETs sharing gate (non-signal bias)
/// and drain, with different sources. Used as common-gate input stage
/// in transimpedance amplifiers.
pub const COMMON_GATE_PAIR: Pattern = Pattern {
    name: "common_gate_pair",
    priority: 6,
    slots: &[
        S_ANY,
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::ExactAs(0), ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "D", 1, "D"),
        ne(0, "S", 1, "S"),
    ],
};

/// Switch pair: two same-type FETs, different gates (complementary
/// control), shared source, shared drain. Parallel switches for
/// reduced on-resistance or redundancy.
pub const SWITCH_PAIR: Pattern = Pattern {
    name: "switch_pair",
    priority: 5,
    slots: &[
        S_ANY,
        same0_exact(),
    ],
    links: &[
        eq(0, "D", 1, "D"),
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
    ],
};

/// Diode-connected cascode: bottom diode-connected, top also
/// diode-connected. Used in bias chains.
pub const DIODE_CASCODE: Pattern = Pattern {
    name: "diode_cascode",
    priority: 9,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "D", 1, "S"),
    ],
};

/// Current mirror with cascode output: M0 is diode-connected reference,
/// M1 is the output with its drain connected back through a cascode.
/// Shared gate, shared source, M0 diode.
/// Same as simple mirror but M1 is NOT diode-connected (explicitly forbidden).
pub const CURRENT_MIRROR_SINGLE_ENDED: Pattern = Pattern {
    name: "current_mirror_single_ended",
    priority: 8,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

/// Beta multiplier reference: M0 (diode-connected, narrow) and M1 (wide)
/// with a degeneration element between M1's source and the shared rail.
/// They share gate, but M1 is wider than M0 (SameLAs but not ExactAs).
/// The source resistor is outside the FET pattern.
pub const BETA_MULTIPLIER_CORE: Pattern = Pattern {
    name: "beta_multiplier_core",
    priority: 9,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        ne(0, "S", 1, "S"),  // different sources (degeneration resistor on M1)
        ne(0, "D", 1, "D"),
    ],
};

/// Complementary source follower: NMOS follower + PMOS follower,
/// sharing drain(=output). Class AB output.
pub const COMPLEMENTARY_SOURCE_FOLLOWER: Pattern = Pattern {
    name: "complementary_source_follower",
    priority: 10,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Forbidden,
            gate_is_signal: true,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),  // shared output at sources
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
    ],
};

/// Common-mode pair with split source: two same-type, same gate,
/// different drains AND different sources. Used as input pair
/// for common-mode feedback sensing.
pub const COMMON_MODE_SENSE_PAIR: Pattern = Pattern {
    name: "common_mode_sense_pair",
    priority: 6,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        ne(0, "S", 1, "S"),
        eq(0, "D", 1, "D"),  // drains tied (sensing common mode)
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 8: Additional three-device patterns
// ═══════════════════════════════════════════════════════════════════════

/// Cascode with mirror bias: M0 is the common-source device, M1 is
/// the cascode device on top, M2 is a diode-connected bias device
/// whose gate drives M1. M2.G = M2.D = M1.G.
pub const CASCODE_WITH_MIRROR_BIAS: Pattern = Pattern {
    name: "cascode_with_mirror_bias",
    priority: 15,
    slots: &[
        S_ANY,
        same0(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(2, "D", 1, "G"),  // diode drives cascode gate
        eq(2, "S", 0, "S"),  // bias device shares source rail
    ],
};

/// Current mirror with cascode on output: M0 diode ref, M1 output,
/// M2 cascode on M1. M0.G = M1.G, M0.S = M1.S, M1.D = M2.S.
/// This is the "half" of a cascode mirror (ref side only basic mirror).
pub const MIRROR_WITH_CASCODE_OUTPUT: Pattern = Pattern {
    name: "mirror_with_cascode_output",
    priority: 15,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(1, "D", 2, "S"),
    ],
};

/// Self-biased cascode: M0 bottom (diode-connected), M1 top cascode
/// (gate driven by M0's drain = M0's gate), M2 output mirror of M0.
/// M0.D = M0.G = M1.G, M0.D = M1.S, M0.G = M2.G.
pub const SELF_BIASED_CASCODE: Pattern = Pattern {
    name: "self_biased_cascode",
    priority: 16,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0(),
        same0_samel(),
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(0, "D", 1, "G"),  // M0 diode drives M1 gate (self-bias)
        eq(0, "G", 2, "G"),
        eq(0, "S", 2, "S"),
    ],
};

/// Sooch cascode mirror: M0 diode, M1 output, M2 cascode.
/// M0 is diode, M0.G = M1.G (basic mirror), M1.D = M2.S (cascode).
/// M2.G is driven by a separate bias (not from M0 or M1).
/// Distinguished from Wilson by: M2.G is NOT connected to M0.D.
pub const SOOCH_MIRROR: Pattern = Pattern {
    name: "sooch_mirror",
    priority: 15,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(1, "D", 2, "S"),
        ne(0, "D", 2, "G"),  // NOT Wilson feedback
        ne(1, "D", 2, "G"),
    ],
};

/// Diff pair with source degeneration devices: M0+M1 diff pair,
/// M2+M3 degeneration FETs in triode (gate tied to supply) between
/// each diff pair source and the tail node.
/// But as a 3-device pattern: diff pair + single tail/degeneration.
pub const DIFF_PAIR_WITH_DEGEN: Pattern = Pattern {
    name: "diff_pair_with_degen",
    priority: 14,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        same0(),  // degeneration/tail
    ],
    links: &[
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        ne(0, "S", 1, "S"),  // split sources (through degen devices)
        eq(0, "S", 2, "D"),  // one degen device
    ],
};

/// CMOS inverter with tail current source: M0(N) + M1(P) inverter,
/// M2 tail current source below M0.
pub const INVERTER_WITH_TAIL: Pattern = Pattern {
    name: "inverter_with_tail",
    priority: 15,
    slots: &[
        S_ANY,
        comp(0),
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "D", 1, "D"),
        eq(0, "S", 2, "D"),
    ],
};

/// Push-pull output stage with bias: M0(N) and M1(P) share drain
/// (output), M2 is a bias/level-shift device connecting M0.G or M1.G.
pub const PUSH_PULL_WITH_BIAS: Pattern = Pattern {
    name: "push_pull_with_bias",
    priority: 14,
    slots: &[
        S_ANY,
        comp(0),
        same0(),  // or same_as(1) — bias device
    ],
    links: &[
        eq(0, "D", 1, "D"),  // shared output
        ne(0, "S", 1, "S"),
        eq(2, "D", 0, "G"),  // bias drives one gate
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 9: Additional four-device patterns
// ═══════════════════════════════════════════════════════════════════════

/// Diff pair with matched cascode on both outputs:
/// M0+M1 diff pair, M2 cascode on M0.D, M3 cascode on M1.D.
pub const DIFF_PAIR_WITH_CASCODES: Pattern = Pattern {
    name: "diff_pair_with_cascodes",
    priority: 22,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        same0(),
        same_exact(2),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        ne(2, "D", 3, "D"),
    ],
};

/// Current mirror with two cascoded outputs: M0 diode ref,
/// M1+M2 two output transistors, M3 cascode on one output.
/// Or alternatively: mirror pair + one cascode.
pub const MIRROR_WITH_DUAL_OUTPUT: Pattern = Pattern {
    name: "mirror_with_dual_output",
    priority: 21,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        same0_samel(),
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "G", 2, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        ne(0, "D", 1, "D"),
        ne(0, "D", 2, "D"),
        ne(1, "D", 2, "D"),
        eq(1, "D", 3, "S"),  // cascode on M1
    ],
};

/// Regulated cascode mirror: M0 diode ref, M1 output, M2 cascode,
/// M3 regulation amplifier. M0.G=M1.G, M0.S=M1.S, M1.D=M2.S,
/// M1.D=M3.G (sense), M3.D=M2.G (regulate).
pub const REGULATED_CASCODE_MIRROR: Pattern = Pattern {
    name: "regulated_cascode_mirror",
    priority: 24,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        same0(),
        same0(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(1, "D", 2, "S"),
        eq(1, "D", 3, "G"),  // sense
        eq(3, "D", 2, "G"),  // regulate
    ],
};

/// CMOS transmission gate pair: two TGs in parallel or series.
/// M0(N)+M1(P) = TG1, M2(N)+M3(P) = TG2.
pub const TRANSMISSION_GATE_PAIR: Pattern = Pattern {
    name: "transmission_gate_pair",
    priority: 21,
    slots: &[
        S_ANY,
        comp(0),
        same0_exact(),
        Slot { kind: SlotKind::SameTypeAs(1), size_match: SizeMatch::ExactAs(1), ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "D"),  // TG1 shared D
        eq(0, "S", 1, "S"),  // TG1 shared S
        ne(0, "G", 1, "G"),  // TG1 complementary clocks
        eq(2, "D", 3, "D"),  // TG2 shared D
        eq(2, "S", 3, "S"),  // TG2 shared S
        ne(2, "G", 3, "G"),  // TG2 complementary clocks
    ],
};

/// Four-transistor current-steering DAC cell: M0+M1 diff switch pair
/// (signal gates, shared source), M2 current source (tail),
/// M3 cascode on current source.
pub const DAC_CURRENT_CELL: Pattern = Pattern {
    name: "dac_current_cell",
    priority: 22,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: true,
        },
        same0(),  // M2: current source
        same0(),  // M3: cascode on source
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(0, "S", 3, "D"),  // cascode output to diff pair source
        eq(3, "S", 2, "D"),  // current source under cascode
    ],
};

/// Feedback amplifier pair: M0+M1 common-source gain stage,
/// M2+M3 feedback devices (gate of M2 = drain of M0, etc).
/// Four same-type FETs forming a two-stage feedback loop.
pub const FEEDBACK_PAIR: Pattern = Pattern {
    name: "feedback_pair",
    priority: 20,
    slots: &[
        S_ANY,
        same0_exact(),
        same0(),
        same_exact(2),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(0, "D", 2, "G"),  // forward
        eq(1, "D", 3, "G"),  // forward
        eq(2, "S", 3, "S"),
    ],
};

/// Schmitt trigger input: M0+M1 NMOS stack, M2+M3 PMOS stack.
/// M0 and M2 share gate (input), M1 gate and M3 gate provide hysteresis
/// feedback from output.
pub const SCHMITT_TRIGGER: Pattern = Pattern {
    name: "schmitt_trigger",
    priority: 22,
    slots: &[
        S_ANY,             // M0: NMOS input
        same0(),           // M1: NMOS feedback
        comp(0),           // M2: PMOS input
        same_as(2),        // M3: PMOS feedback
    ],
    links: &[
        eq(0, "G", 2, "G"),  // shared input
        eq(0, "D", 1, "S"),  // NMOS stack
        eq(2, "D", 3, "S"),  // PMOS stack
        eq(1, "D", 3, "D"),  // shared output
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 10: Protection and biasing patterns
// ═══════════════════════════════════════════════════════════════════════

/// ESD clamp: two diode-connected FETs in series (NMOS),
/// forming a diode voltage clamp between supply and ground.
/// M0 diode source=gnd, M1 diode source=M0.drain.
pub const ESD_DIODE_CLAMP: Pattern = Pattern {
    name: "esd_diode_clamp",
    priority: 10,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "D", 1, "S"),
    ],
};

/// Bias chain: three diode-connected FETs in series.
/// M0.D = M1.S, M1.D = M2.S. Generates multi-VGS bias voltage.
pub const BIAS_CHAIN_3: Pattern = Pattern {
    name: "bias_chain_3",
    priority: 14,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(1, "D", 2, "S"),
    ],
};

/// Startup circuit element: M0 (weak device, long L or small W) with
/// gate sensing a bias voltage, providing initial current to kick-start
/// a self-biased circuit. M1 provides the mirror.
/// Structurally just a mirror but with intentionally mismatched W.
pub const STARTUP_MIRROR: Pattern = Pattern {
    name: "startup_mirror",
    priority: 7,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0), // same L, different W (ratio)
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        ne(0, "D", 1, "D"),
    ],
};

/// Bandgap core (transistor part): M0+M1 form a current mirror,
/// M2+M3 form another mirror or diff pair sensing the PTAT/CTAT voltages.
/// The resistors are outside the FET pattern.
pub const BANDGAP_MIRROR_PAIR: Pattern = Pattern {
    name: "bandgap_mirror_pair",
    priority: 20,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        comp(0),
        Slot {
            kind: SlotKind::SameTypeAs(2),
            size_match: SizeMatch::ExactAs(2),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(2, "G", 3, "G"),
        eq(2, "S", 3, "S"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 11: Additional composite patterns
// ═══════════════════════════════════════════════════════════════════════

/// Current-mirror OTA (simple): M0+M1 NMOS diff pair, M2 diode load,
/// M3 mirror load, M4 tail. Same as 5T OTA but without exact type
/// constraint on load type.
/// (Alias: this is structurally identical to FIVE_TRANSISTOR_OTA but
/// kept for naming compatibility with ALIGN's "current_mirror_ota" template.)
// NOTE: This is intentionally the same connectivity as FIVE_TRANSISTOR_OTA.
// The pattern engine will only match one since devices get consumed.

/// Diff pair with diode load + mirror load + tail (5T variant where
/// loads are same type as diff pair but diode/mirror).
pub const OTA_SELF_BIASED_LOAD: Pattern = Pattern {
    name: "ota_self_biased_load",
    priority: 32,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(2), ..S_ANY },
        comp(0), // tail is complementary type
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(0, "D", 2, "D"),
        eq(1, "D", 3, "D"),
        eq(2, "G", 3, "G"),
        eq(2, "S", 3, "S"),
        eq(0, "S", 4, "D"),
    ],
};

/// Cascode current mirror OTA: M0+M1 diff pair, M2+M3 cascode mirrors
/// as loads, M4 tail. Diff pair drains connect to cascode sources.
pub const CASCODE_MIRROR_OTA: Pattern = Pattern {
    name: "cascode_mirror_ota",
    priority: 33,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),             // M2: cascode load A
        same_exact(2),       // M3: cascode load B
        same0(),             // M4: tail
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        eq(0, "S", 4, "D"),
    ],
};

/// Complementary diff pair: NMOS diff pair + PMOS diff pair sharing
/// the same input signals but providing rail-to-rail input range.
/// M0+M1 NMOS DP, M2+M3 PMOS DP.
pub const COMPLEMENTARY_DIFF_PAIR: Pattern = Pattern {
    name: "complementary_diff_pair",
    priority: 23,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::SameTypeAs(2),
            size_match: SizeMatch::ExactAs(2),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(2, "S", 3, "S"),
        eq(0, "G", 2, "G"),  // same signal inputs
        eq(1, "G", 3, "G"),
        ne(2, "D", 3, "D"),
    ],
};

/// Common-mode feedback sensing pair with mirror: M0+M1 sense the two
/// diff outputs (gate=outp, gate=outn), M2 is a diode-connected reference.
/// All three share drain (summing node).
pub const CMFB_SENSE_PAIR: Pattern = Pattern {
    name: "cmfb_sense_pair",
    priority: 15,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        same0_exact(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
    ],
    links: &[
        eq(0, "D", 1, "D"),  // summing node
        eq(0, "D", 2, "D"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        ne(0, "G", 1, "G"),
    ],
};

/// Two-stage miller compensation core: M0 first-stage CS device,
/// M1 second-stage CS device, M2 compensation cap device (FET as cap).
/// M0.D = M2.S (compensation node), M1.D = M2.G (output).
/// Note: the actual cap is typically a passive, but some PDKs use
/// MOS caps. This pattern catches the FET-based variant.
pub const MILLER_COMP_FETS: Pattern = Pattern {
    name: "miller_comp_fets",
    priority: 14,
    slots: &[
        S_ANY,
        S_ANY,
        S_ANY,
    ],
    links: &[
        eq(0, "D", 2, "S"),  // comp node
        eq(1, "D", 2, "G"),  // output drives cap gate
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 12: Switched-capacitor and clock-related
// ═══════════════════════════════════════════════════════════════════════

/// Non-overlapping clock generator element: two cross-coupled NAND gates
/// implemented at transistor level. M0+M1 = first NAND (NMOS),
/// M2+M3 = second NAND (PMOS portion). Structural match only.
pub const NAND_CROSS_COUPLED: Pattern = Pattern {
    name: "nand_cross_coupled",
    priority: 22,
    slots: &[
        S_ANY,          // M0: NAND1 pull-down A
        same0(),        // M1: NAND1 pull-down B (series)
        comp(0),        // M2: NAND1 pull-up A
        same_as(2),     // M3: NAND1 pull-up B
    ],
    links: &[
        eq(0, "D", 1, "S"),  // series NMOS
        eq(1, "D", 2, "D"),  // output node
        eq(1, "D", 3, "D"),
        eq(2, "S", 3, "S"),
        eq(0, "G", 2, "G"),  // input A shared
    ],
};

/// Sample-and-hold switch: transmission gate (M0+M1) with a bootstrap
/// device (M2) that boosts the gate voltage of the NMOS switch.
pub const BOOTSTRAPPED_SWITCH: Pattern = Pattern {
    name: "bootstrapped_switch",
    priority: 15,
    slots: &[
        S_ANY,
        comp(0),
        same0(),
    ],
    links: &[
        eq(0, "D", 1, "D"),
        eq(0, "S", 1, "S"),
        eq(2, "D", 0, "G"),  // bootstrap drives NMOS gate
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 13: More five+ device composite patterns
// ═══════════════════════════════════════════════════════════════════════

/// Symmetrical OTA (Razavi Fig. 9.35): M0+M1 diff pair, M2+M3
/// same-type cascode on diff outputs, M4 tail. Cascoded diff pair
/// for higher gain.
pub const CASCODED_DIFF_PAIR_WITH_TAIL: Pattern = Pattern {
    name: "cascoded_diff_pair_with_tail",
    priority: 32,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        same0(),
        same_exact(2),
        same0(), // tail
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        ne(2, "D", 3, "D"),
        eq(0, "S", 4, "D"),
    ],
};

/// Fully differential amplifier output stage: two complementary
/// source followers (M0+M1 for out+, M2+M3 for out-), biased by M4.
/// Outputs are at the source-source junction (complementary followers).
pub const DIFF_OUTPUT_STAGE: Pattern = Pattern {
    name: "diff_output_stage",
    priority: 30,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Forbidden,
            gate_is_signal: true,
        },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        Slot { kind: SlotKind::SameTypeAs(1), size_match: SizeMatch::ExactAs(1), ..S_ANY },
        same0(), // tail/bias
    ],
    links: &[
        eq(0, "S", 1, "S"),  // out+ (source followers share output at source)
        eq(2, "S", 3, "S"),  // out-
        eq(0, "G", 1, "G"),  // same gate signal drives complementary pair
        eq(2, "G", 3, "G"),  // same gate signal drives complementary pair
        ne(0, "G", 2, "G"),  // different signal inputs
        ne(0, "D", 1, "D"),  // different supply rails
        ne(0, "S", 2, "S"),  // different outputs
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 14: Additional two-device variants
// ═══════════════════════════════════════════════════════════════════════

/// Cascode with diode on top: bottom device (not diode-connected),
/// top device is diode-connected. Used for generating cascode gate bias.
pub const CASCODE_DIODE_TOP: Pattern = Pattern {
    name: "cascode_diode_top",
    priority: 9,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::SameLAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "D", 1, "S"),
    ],
};

/// Anti-parallel pair: two same-type FETs where source of A = drain of B
/// AND drain of A = source of B. Bidirectional switch.
pub const ANTI_PARALLEL_SWITCH: Pattern = Pattern {
    name: "anti_parallel_switch",
    priority: 10,
    slots: &[
        S_ANY,
        same0_exact(),
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(0, "S", 1, "D"),
        ne(0, "G", 1, "G"),
    ],
};

/// Triode-biased load pair: two same-type FETs where gate is tied to
/// supply (via the source rail), acting as linear resistors.
/// Shared source, different drains, shared gate = source.
pub const TRIODE_LOAD_PAIR: Pattern = Pattern {
    name: "triode_load_pair",
    priority: 6,
    slots: &[
        S_ANY,
        same0_exact(),
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "G", 0, "S"),  // gate tied to source (triode bias)
        ne(0, "D", 1, "D"),
    ],
};

/// Differential switch (analog mux): two same-type FETs sharing source,
/// different gates (select signals), different drains (outputs).
/// Similar to diff pair but gates are digital select, not analog signal.
/// (Structurally matches diff_pair if gates happen to be signal; this
/// catches the non-signal-gate case.)
pub const DIFF_SWITCH: Pattern = Pattern {
    name: "diff_switch",
    priority: 8,
    slots: &[
        Slot { gate_is_signal: false, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        ne(0, "G", 1, "D"),  // exclude cross-coupled
        ne(1, "G", 0, "D"),  // exclude cross-coupled
    ],
};

/// Dummy pair: two identical FETs with all terminals shorted together
/// (or gate/source tied). Used for matching/symmetry in layout.
/// Both are "diode-connected" equivalent (gate=drain or gate=source).
pub const DUMMY_PAIR: Pattern = Pattern {
    name: "dummy_pair",
    priority: 5,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        eq(0, "D", 1, "D"),
        eq(0, "G", 1, "G"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 15: Additional three-device patterns
// ═══════════════════════════════════════════════════════════════════════

/// Triple cascode stack: three same-type FETs in series.
/// M0.D = M1.S, M1.D = M2.S. All different gates.
/// Used in high-voltage or ultra-high-output-impedance designs.
pub const TRIPLE_CASCODE: Pattern = Pattern {
    name: "triple_cascode",
    priority: 16,
    slots: &[
        S_ANY,
        same0(),
        same0(),
    ],
    links: &[
        eq(0, "D", 1, "S"),
        eq(1, "D", 2, "S"),
        ne(0, "G", 1, "G"),
        ne(1, "G", 2, "G"),
    ],
};

/// Cascode mirror with diode bias: M0 diode ref, M1 output mirror,
/// M2 cascode on output with gate driven by M0's drain.
/// M0.G=M1.G, M0.S=M1.S, M1.D=M2.S, M0.D=M2.G.
/// (This is the same as Wilson mirror but with explicit naming.)
// Different from Wilson in that M2.D is the final output (no feedback
// from M2.D back into the circuit).

/// Current mirror with two outputs: M0 diode ref, M1+M2 two output
/// transistors sharing gate with M0, all sharing source.
/// Distinct from CURRENT_MIRROR_3 in that M1 and M2 may have
/// different W (only same L required).
pub const CURRENT_MIRROR_1_TO_2: Pattern = Pattern {
    name: "current_mirror_1_to_2",
    priority: 14,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
        Slot { kind: SlotKind::SameTypeAs(0), size_match: SizeMatch::SameLAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "G", 2, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "S", 2, "S"),
        ne(0, "D", 1, "D"),
        ne(0, "D", 2, "D"),
    ],
};

/// Super source follower: M0 (follower, signal gate), M1 (current source),
/// M2 (feedback amp sensing M0.S and driving M1.G).
/// Provides very low output impedance.
pub const SUPER_SOURCE_FOLLOWER: Pattern = Pattern {
    name: "super_source_follower",
    priority: 16,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },  // M0: follower
        same0(),                                    // M1: current source
        same0(),                                    // M2: feedback amp
    ],
    links: &[
        eq(1, "D", 0, "S"),  // M1 current source to follower source
        eq(0, "S", 2, "S"),  // M2 source = output (sensing)
        eq(2, "D", 1, "G"),  // M2 output drives M1 gate
    ],
};

/// Complementary self-biased inverter chain: M0(N)+M1(P) inverter,
/// M2 feedback from output to... actually let's do a useful one:
///
/// Three-transistor current source: M0 diode ref, M1 mirror output,
/// M2 cascode on M0 (diode-connected). Provides cascoded reference.
pub const CASCODED_REFERENCE: Pattern = Pattern {
    name: "cascoded_reference",
    priority: 15,
    slots: &[
        Slot { diode: DiodeReq::Required, ..S_ANY },
        same0_samel(),
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Required, ..S_ANY },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),  // cascode on ref
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 16: Additional four-device patterns
// ═══════════════════════════════════════════════════════════════════════

/// NAND gate (transistor level): M0+M1 series NMOS, M2+M3 parallel PMOS.
/// M0.G = M2.G (input A), M1.G = M3.G (input B).
/// Output at M1.D = M2.D = M3.D.
pub const NAND_GATE: Pattern = Pattern {
    name: "nand_gate",
    priority: 22,
    slots: &[
        S_ANY,          // M0: NMOS A
        same0(),        // M1: NMOS B
        comp(0),        // M2: PMOS A
        same_as(2),     // M3: PMOS B
    ],
    links: &[
        eq(0, "D", 1, "S"),   // series NMOS
        eq(1, "D", 2, "D"),   // output
        eq(1, "D", 3, "D"),   // output
        eq(0, "G", 2, "G"),   // input A
        eq(1, "G", 3, "G"),   // input B
        eq(2, "S", 3, "S"),   // shared PMOS source (VDD)
    ],
};

/// NOR gate (transistor level): M0+M1 parallel NMOS, M2+M3 series PMOS.
/// M0.G = M2.G (input A), M1.G = M3.G (input B).
/// Output at M0.D = M1.D = M3.D.
pub const NOR_GATE: Pattern = Pattern {
    name: "nor_gate",
    priority: 22,
    slots: &[
        S_ANY,          // M0: NMOS A
        same0(),        // M1: NMOS B
        comp(0),        // M2: PMOS A
        same_as(2),     // M3: PMOS B
    ],
    links: &[
        eq(0, "S", 1, "S"),   // parallel NMOS share source
        eq(0, "D", 1, "D"),   // parallel NMOS share drain
        eq(2, "D", 3, "S"),   // series PMOS
        eq(3, "D", 0, "D"),   // output = NMOS drain = bottom PMOS drain
        eq(0, "G", 2, "G"),   // input A
        eq(1, "G", 3, "G"),   // input B
    ],
};

/// Cascode diff pair with independent biases: M0+M1 diff pair,
/// M2 cascode on M0 with bias voltage A, M3 cascode on M1 with
/// different bias voltage B. Used when cascode devices need
/// different bias points.
pub const DIFF_PAIR_WITH_SPLIT_CASCODES: Pattern = Pattern {
    name: "diff_pair_with_split_cascodes",
    priority: 21,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        same0(),
        same_exact(2),
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        ne(2, "G", 3, "G"),  // different cascode biases
        ne(2, "D", 3, "D"),
    ],
};

/// Folded pair: diff pair where outputs connect to sources (not drains)
/// of complement-type devices. M0+M1 diff (same type),
/// M2+M3 complement loads. M0.D = M2.S, M1.D = M3.S.
pub const FOLDED_LOAD_PAIR: Pattern = Pattern {
    name: "folded_load_pair",
    priority: 22,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: true,
        },
        comp(0),
        Slot { kind: SlotKind::SameTypeAs(2), size_match: SizeMatch::ExactAs(2), ..S_ANY },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        eq(0, "D", 2, "S"),  // folded connection
        eq(1, "D", 3, "S"),  // folded connection
        eq(2, "G", 3, "G"),
    ],
};

/// Two-stage inverter buffer: inv1 (M0+M1) drives inv2 (M2+M3).
/// M0.D = M1.D = M2.G = M3.G.
pub const INVERTER_BUFFER: Pattern = Pattern {
    name: "inverter_buffer",
    priority: 23,
    slots: &[
        S_ANY,          // M0: inv1 N
        comp(0),        // M1: inv1 P
        same0(),        // M2: inv2 N
        same_as(1),     // M3: inv2 P
    ],
    links: &[
        eq(0, "G", 1, "G"),    // inv1 input
        eq(0, "D", 1, "D"),    // inv1 output
        eq(2, "G", 3, "G"),    // inv2 input
        eq(2, "D", 3, "D"),    // inv2 output
        eq(0, "D", 2, "G"),    // chain
    ],
};

/// Cascode current source pair: two independent cascode current sources
/// (4 FETs) where bottom pair shares gate+source and top pair shares gate.
/// All four are NOT diode-connected (externally biased).
pub const CASCODE_CURRENT_SOURCE_PAIR: Pattern = Pattern {
    name: "cascode_current_source_pair",
    priority: 21,
    slots: &[
        Slot { diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
        Slot { kind: SlotKind::SameTypeAs(0), diode: DiodeReq::Forbidden, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(2),
            size_match: SizeMatch::ExactAs(2),
            diode: DiodeReq::Forbidden,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "S", 1, "S"),
        eq(0, "D", 2, "S"),
        eq(1, "D", 3, "S"),
        eq(2, "G", 3, "G"),
        ne(2, "D", 3, "D"),
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Category 17: More three-device patterns
// ═══════════════════════════════════════════════════════════════════════

/// Active-loaded inverter: CMOS inverter (M0+M1) with current source tail
/// on the PMOS side. M0(N)+M1(P) share gate and drain, M2(P) provides
/// bias current with its drain = M1's source.
pub const ACTIVE_LOADED_INVERTER: Pattern = Pattern {
    name: "active_loaded_inverter",
    priority: 15,
    slots: &[
        S_ANY,
        comp(0),
        same_as(1),  // same type as M1
    ],
    links: &[
        eq(0, "G", 1, "G"),
        eq(0, "D", 1, "D"),
        eq(2, "D", 1, "S"),
    ],
};

/// Diff pair with common-mode input: M0+M1 diff pair where one input
/// gate is driven by a diode-connected M2 (reference level).
pub const DIFF_PAIR_WITH_REFERENCE: Pattern = Pattern {
    name: "diff_pair_with_reference",
    priority: 15,
    slots: &[
        Slot { gate_is_signal: true, ..S_ANY },
        Slot {
            kind: SlotKind::SameTypeAs(0),
            size_match: SizeMatch::ExactAs(0),
            diode: DiodeReq::Any,
            gate_is_signal: false,
        },
        Slot {
            kind: SlotKind::ComplementOf(0),
            size_match: SizeMatch::Any,
            diode: DiodeReq::Required,
            gate_is_signal: false,
        },
    ],
    links: &[
        eq(0, "S", 1, "S"),
        ne(0, "G", 1, "G"),
        ne(0, "D", 1, "D"),
        eq(1, "G", 2, "D"),  // M2 diode drives M1 gate
    ],
};

// ═══════════════════════════════════════════════════════════════════════
//  Master list — engine iterates this automatically
// ═══════════════════════════════════════════════════════════════════════

/// All registered patterns, ordered by priority (highest first).
/// The engine applies greedy non-overlapping selection, so larger/
/// more-specific patterns consume devices before smaller ones can.
pub const PATTERNS: &[Pattern] = &[
    // ── 8-device composites (50–59) ──
    TELESCOPIC_OTA_FULL,

    // ── 6-device composites (40–49) ──
    GILBERT_CELL,
    VCO_CORE_WITH_TAILS,
    FOLDED_CASCODE_CORE,
    TELESCOPIC_OTA_CORE,

    // ── 5-device composites (30–39) ──
    DIFF_PAIR_CROSS_COUPLED_LOAD,
    FIVE_TRANSISTOR_OTA,
    DIFF_PAIR_CASCODE_LOAD,
    CASCODE_MIRROR_OTA,
    OTA_SELF_BIASED_LOAD,
    CASCODED_DIFF_PAIR_WITH_TAIL,
    WIDE_SWING_CASCODE_MIRROR_5,
    DIFF_OUTPUT_STAGE,

    // ── 4-device composites (20–29) ──
    CROSS_COUPLED_INVERTERS,
    CROSS_COUPLED_COMPLEMENTARY,
    WIDE_SWING_CASCODE_MIRROR,
    IMPROVED_WILSON_MIRROR_4,
    CASCODE_MIRROR,
    REGULATED_CASCODE_MIRROR,
    WILSON_MIRROR_4,
    INVERTER_BUFFER,
    DIFF_PAIR_WITH_MIRROR_LOAD,
    COMPLEMENTARY_DIFF_PAIR,
    DIFF_PAIR_WITH_ACTIVE_LOAD,
    DIFF_PAIR_WITH_CASCODES,
    FOLDED_LOAD_PAIR,
    CASCODE_PAIR,
    CHARGE_PUMP_CELL,
    DAC_CURRENT_CELL,
    SCHMITT_TRIGGER,
    NAND_GATE,
    NOR_GATE,
    NAND_CROSS_COUPLED,
    LOW_VOLTAGE_CASCODE_MIRROR,
    CASCODE_PAIR_SYMMETRIC,
    DIFF_PAIR_WITH_SPLIT_CASCODES,
    CASCODE_CURRENT_SOURCE_PAIR,
    TRANSMISSION_GATE_PAIR,
    MIRROR_WITH_DUAL_OUTPUT,
    LEVEL_SHIFTER,
    CURRENT_MIRROR_4,
    BANDGAP_MIRROR_PAIR,
    FEEDBACK_PAIR,

    // ── 3-device patterns (14–19) ──
    DIFF_PAIR_WITH_TAIL,
    IMPROVED_WILSON_MIRROR,
    WILSON_MIRROR,
    TRIPLE_CASCODE,
    REGULATED_CASCODE,
    SUPER_SOURCE_FOLLOWER,
    FLIPPED_VOLTAGE_FOLLOWER,
    SELF_BIASED_CASCODE,
    SOURCE_FOLLOWER_WITH_MIRROR,
    DIFF_PAIR_DIODE_LOAD,
    DIFF_PAIR_WITH_REFERENCE,
    MIRROR_WITH_CASCODE_OUTPUT,
    CASCODED_REFERENCE,
    SOOCH_MIRROR,
    CASCODE_WITH_MIRROR_BIAS,
    CASCODE_WITH_DEGENERATION,
    INVERTER_WITH_TAIL,
    ACTIVE_LOADED_INVERTER,
    BOOTSTRAPPED_SWITCH,
    CMFB_SENSE_PAIR,
    CURRENT_MIRROR_3,
    CURRENT_MIRROR_1_TO_2,
    ACTIVE_LOAD_3,
    DIFF_PAIR_WITH_DEGEN,
    PUSH_PULL_WITH_BIAS,
    BIAS_CHAIN_3,
    MILLER_COMP_FETS,

    // ── 2-device patterns (5–13) ──
    CMOS_INVERTER,
    TRANSMISSION_GATE,
    DIFF_PAIR,
    PUSH_PULL_PAIR,
    COMPLEMENTARY_SOURCE_FOLLOWER,
    ANTI_PARALLEL_SWITCH,
    DIODE_CASCODE,
    CASCODE_DIODE_TOP,
    CROSS_COUPLED,
    CROSS_COUPLED_SPLIT_SOURCE,
    DIFF_PAIR_SPLIT_SOURCE,
    BETA_MULTIPLIER_CORE,
    ESD_DIODE_CLAMP,
    CASCODE_MATCHED,
    CURRENT_MIRROR,
    CURRENT_MIRROR_SINGLE_ENDED,
    DIFF_SWITCH,
    CURRENT_MIRROR_OUTPUT_PAIR,
    SOURCE_FOLLOWER,
    ACTIVE_LOAD,
    DIODE_LOAD_PAIR,
    LATCH_HALF,
    STARTUP_MIRROR,
    COMMON_MODE_SENSE_PAIR,
    COMMON_GATE_PAIR,
    TRIODE_LOAD_PAIR,
    MIRROR_PAIR_SPLIT_SOURCE,
    SERIES_STACK,
    CASCODE,
    DEGENERATION_PAIR,
    SWITCH_PAIR,
    DUMMY_PAIR,
];
