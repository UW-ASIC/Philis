//! The sidecar registry's `required` set is the set generators read with a
//! compiled default, both ways.
//!
//! A `required` key missing from a sidecar fails `Pdk::load`; a key read with
//! a compiled default but not required would silently build to that default
//! on a deck that omits it, and a required key nobody reads is a demand with
//! no consumer; a name in no registry row that the loaded PDK does not state
//! builds to its default on every deck. This runs `enumerate` + `draw` of
//! every generator on sky130 through a `Process` that records each `rule()`
//! name, and compares. Not reached: post_cell.rs `guard_ring_merge_gap_nm`
//! (placement-time design policy, default 2000).

use std::cell::RefCell;
use std::collections::BTreeSet;

use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::{bjt::Bjt, cap_array::CapArray, capacitor::Capacitor, diode::Diode, finfet::FinFet, inductor::Inductor, mosfet::Mosfet, resistor::Resistor, Cell};
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, LayerId, Process};
use verify::sidecar::KEYS;

/// `pdk`, recording every `rule()` name asked of it.
struct Recording<'a>(&'a verify::Pdk, RefCell<BTreeSet<String>>);

impl Process for Recording<'_> {
    fn layer(&self, role: &str) -> Option<LayerId> {
        self.0.layer(role)
    }
    fn rule(&self, name: &str, default: i32) -> i32 {
        self.1.borrow_mut().insert(name.to_string());
        self.0.rule(name, default)
    }
    fn grid(&self) -> i32 {
        self.0.grid()
    }
    fn sheet_ohm(&self, role: &str) -> Option<f32> {
        self.0.sheet_ohm(role)
    }
    fn space(&self, role: &str) -> Option<i32> {
        self.0.space(role)
    }
    fn eol_space(&self, role: &str) -> Option<i32> {
        self.0.eol_space(role)
    }
    fn width(&self, role: &str) -> Option<i32> {
        Process::width(self.0, role)
    }
    fn enclosure(&self, outer: &str, inner: &str) -> Option<i32> {
        self.0.enclosure(outer, inner)
    }
    fn endcap(&self, outer: &str, inner: &str) -> Option<i32> {
        self.0.endcap(outer, inner)
    }
    fn extension(&self, outer: &str, inner: &str) -> Option<i32> {
        self.0.extension(outer, inner)
    }
    fn area(&self, role: &str) -> Option<i64> {
        self.0.area(role)
    }
    fn space_between(&self, a: &str, b: &str) -> Option<i32> {
        self.0.space_between(a, b)
    }
}

/// `n` matched devices of `kind`, `dev_nf` units of `w`×`l` nm each (as in
/// cell_selfcheck.rs `group_of`, with the size and unit counts free).
fn group_of(kind: DeviceKind, dev_nf: &[u16], w: i32, l: i32, dummy_required: bool, sp: SeriesParallel) -> (DeviceGroup, Constraints) {
    let group = DeviceGroup { devices: (0..dev_nf.len()).map(|i| DeviceId(i as u16)).collect() };
    let mut c = Constraints::default();
    c.unitization.push(Unitization {
        devices: group.devices.clone(),
        device_type: kind,
        dev_nf: dev_nf.to_vec(),
        target_ratio: dev_nf.to_vec(),
        unit_w: w,
        unit_l: l,
        series_parallel: sp,
        same_variant_required: true,
        dummy_required,
        route_matching_required: dummy_required,
    });
    (group, c)
}

/// Every variant `G` offers for the group, drawn; how many.
fn draw_all<G: Cell>(p: &dyn Process, kind: DeviceKind, dev_nf: &[u16], w: i32, l: i32, sp: SeriesParallel) -> usize {
    let mut n = 0;
    for dummies in [false, true] {
        let (g, c) = group_of(kind, dev_nf, w, l, dummies, sp);
        for v in G::enumerate(&g, &c, p) {
            v.draw(&g, &c, p);
            n += 1;
        }
    }
    n
}

#[test]
fn generators_read_exactly_the_required_keys() {
    let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
    let rec = Recording(&pdk, RefCell::default());
    let p: &dyn Process = &rec;
    use DeviceKind::{Npn, Nmos, Pmos, Pnp};
    use SeriesParallel::{Parallel, Series};
    // Sizes as each generator's own self-check draws them.
    let mut drawn = Vec::new();
    for kind in [Nmos, Pmos] {
        for nf in [&[1u16][..], &[2, 2], &[4, 4, 4, 4]] {
            drawn.push(("mosfet", draw_all::<Mosfet>(p, kind, nf, 1680, 150, Parallel)));
        }
        drawn.push(("mosfet chain", draw_all::<Mosfet>(p, kind, &[1, 1], 2000, 500, Series)));
    }
    for nf in [&[1u16][..], &[1, 1], &[1, 1, 1]] {
        drawn.push(("resistor", draw_all::<Resistor>(p, DeviceKind::Resistor, nf, 500, 10_000, Parallel)));
    }
    drawn.push(("capacitor", draw_all::<Capacitor>(p, DeviceKind::Capacitor, &[4, 4], 2000, 2000, Parallel)));
    drawn.push(("cap_array", draw_all::<CapArray>(p, DeviceKind::Capacitor, &[1, 1, 2], 2000, 2000, Parallel)));
    for kind in [Npn, Pnp] {
        drawn.push(("bjt", draw_all::<Bjt>(p, kind, &[1, 8], 1000, 1000, Parallel)));
    }
    drawn.push(("diode", draw_all::<Diode>(p, DeviceKind::Diode, &[1, 1], 500, 1000, Parallel)));
    drawn.push(("inductor", draw_all::<Inductor>(p, DeviceKind::Inductor, &[1], 2000, 20_000, Parallel)));
    // sky130 has no fin layer: FinFet offers nothing here, recorded for completeness.
    draw_all::<FinFet>(p, Nmos, &[1], 1680, 150, Parallel);
    let empty: Vec<_> = drawn.iter().filter(|(_, n)| *n == 0).collect();
    assert!(empty.is_empty(), "generators that drew nothing, so recorded nothing: {empty:?}");

    let recorded = rec.1.into_inner();
    // Required keys read outside `rule()` on kernel/cells generators: `layers`
    // by verify's parse_roles, `max_finger_width` by the library's cellgen
    // `folds` (which takes a `&Pdk`, so this recording cannot reach it; its
    // default 0 there means no finger-width limit).
    let read_elsewhere = ["layers", "max_finger_width"];
    let lax: Vec<_> = read_elsewhere.iter().filter(|n| !KEYS.iter().any(|k| k.name == **n && k.required)).collect();
    assert!(lax.is_empty(), "keys read with a compiled default outside this recording, not required: {lax:?}");
    let required: BTreeSet<&str> = KEYS.iter().filter(|k| k.required && !read_elsewhere.contains(&k.name)).map(|k| k.name).collect();
    let unread: Vec<_> = required.iter().filter(|k| !recorded.contains(**k)).collect();
    assert!(unread.is_empty(), "required keys no generator read (make them optional or delete them): {unread:?}");
    // Names read with a compiled default that may still be absent. The reader
    // only raises a deck-derived value with them (`dim` in builder.rs takes
    // the max with the deck's own rule, the mosfet/bjt/diode/resistor
    // contact enclosures and spacings max them with the deck's enclosure,
    // endcap or spacing), so an absent key leaves the deck's number, not
    // Philis's; or they are a flag whose absence is the conservative choice
    // (`npn_isolation` 0 = no isolated NPN offered); or a resistor recipe's
    // own slot, whose absence (0) is the deck's square `contact`.
    let raise_deck = [
        "contact",
        "mcon_size",
        "met1_space",
        "poly_ext",
        "nwell_diff_enc",
        "m1_enc",
        "min_finger_width",
        "via_enclosure",
        "via_spacing",
        "diff_encloses_licon",
        "tap_encloses_licon_one_side",
        "poly_encloses_licon_one_side",
        "rpm_encloses_poly",
        "li_encloses_licon",
        "li_encloses_licon_one_side",
        "licon_poly_enc",
        "licon_to_gate_spacing",
        "polycon_to_diff_spacing",
        "polycon_to_pdiff_spacing",
    ];
    let flags = ["npn_isolation", "res_contact_w", "res_contact_h"];
    let optional_read: Vec<_> = KEYS
        .iter()
        .filter(|k| !k.required && recorded.contains(k.name) && !raise_deck.contains(&k.name) && !flags.contains(&k.name))
        .map(|k| k.name)
        .collect();
    assert!(optional_read.is_empty(), "keys generators read with a compiled default but not required (a deck omitting them builds to Philis's number): {optional_read:?}");
    // And a name in no registry row: it must resolve from the loaded PDK
    // (sidecar scalar or deck-derived `<role>_min_*`; `min_gate_l` from the
    // deck's channel rules), or every shipped deck builds to the default.
    let resolved: BTreeSet<&str> = pdk.rules.iter().map(|(n, _)| n.as_str()).chain(["min_gate_l"]).collect();
    let defaulted: Vec<_> = recorded
        .iter()
        .filter(|n| !resolved.contains(n.as_str()) && !raise_deck.contains(&n.as_str()) && !flags.contains(&n.as_str()))
        .collect();
    assert!(defaulted.is_empty(), "names generators read that sky130 does not state, so they build to Philis's compiled default: {defaulted:?}");

    let sidecar = pdk.cell.as_object().expect("cell section");
    let registered: BTreeSet<&str> = KEYS.iter().map(|k| k.name).collect();
    let read: Vec<&String> = recorded.iter().filter(|k| sidecar.contains_key(*k)).collect();
    assert!(!read.is_empty(), "no sidecar key read: the recording saw nothing");
    let stray: Vec<_> = read.iter().filter(|k| !registered.contains(k.as_str())).collect();
    assert!(stray.is_empty(), "sidecar keys generators read that the registry lacks: {stray:?}");
    let stale: Vec<_> = KEYS.iter().filter(|k| k.reader == "unread" && recorded.contains(k.name)).map(|k| k.name).collect();
    assert!(stale.is_empty(), "keys registered as unread that generators read: {stale:?}");
}
