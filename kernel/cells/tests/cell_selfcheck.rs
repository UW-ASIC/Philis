//! **Per-generator self-check** — is the device correct *on its own*, before any
//! placer or router touches it?
//!
//! Without this, a signoff failure is ambiguous: 298 DRC violations and a failed
//! LVS extraction could come from the cell generators, the placer, the router, or
//! the interaction of all three, and the only way to tell is to read geometry by
//! hand. That ambiguity is expensive every time any of those three changes.
//!
//! So each generator draws exactly one device, in isolation, and the result is
//! checked against the PDK the same way signoff would check it. A failure here is
//! unambiguously the generator's fault. A *clean* result here means a later
//! failure belongs to placement or routing — which is the whole point: it turns
//! one global "something is wrong" into a localised answer.
//!
//! These are the invariants a device must satisfy by construction:
//!
//! 1. **DRC/ERC-clean in isolation** — checked in each generator's own file
//!    (`every_variant_is_drc_and_erc_clean`), not here.
//! 2. **Extractable.** A MOSFET's channel must carry the implant that names its
//!    type, or LVS extraction cannot tell an NMOS from a PMOS and gives up before
//!    comparing anything.
//! 3. **Well-hosted.** A PMOS must sit in an nwell that encloses its diffusion.

use analog::cell::{SeriesParallel, Unitization};
use cells::{Cell, mosfet::Mosfet};
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, Macro, Rect, Shape};

/// Load the sky130 deck the benchmarks use. Skips (rather than fails) when the
/// PDK is absent, so the suite still runs outside the dev shell.
fn pdk() -> Option<verify::Pdk> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let json = std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?;
    // A present-but-broken deck is a failure, not a skip.
    Some(verify::Pdk::from_json(&json).expect("pdks/sky130.json loads"))
}

/// The group + constraints for `n` matched devices of `kind`, sized so the
/// enumeration has room to offer several `nf` refolds.
///
/// The dummy count follows `dummy_required` (not a variant), so the caller
/// sweeps it: a variant nobody ever draws is a variant nobody ever checks.
fn group_of(kind: DeviceKind, n: usize, nf: u16, dummy_required: bool) -> (DeviceGroup, analog::Constraints) {
    let group = DeviceGroup { devices: (0..n).map(|i| DeviceId(i as u16)).collect() };
    let mut c = analog::Constraints::default();
    c.unitization.push(Unitization {
        devices: group.devices.clone(),
        device_type: kind,
        dev_nf: vec![nf; n],
        target_ratio: vec![1; n],
        unit_w: 1680,
        unit_l: 150,
        series_parallel: SeriesParallel::Parallel,
        same_variant_required: true,
        dummy_required,
        route_matching_required: false,
    });
    (group, c)
}

/// **Every** variant of a `n`-device group, labelled by the axis values that
/// produced it. The old helper drew `variants.first()` under default
/// constraints, which meant three things at once: only one point of the space
/// was ever checked, `kind` was ignored (so the PMOS cases below silently
/// tested an NMOS), and the sizing was degenerate. A generator self-check that
/// skips most of what the generator can emit is not a self-check.
fn variants(kind: DeviceKind, n: usize, nf: u16, pdk: &verify::Pdk) -> Vec<(String, Macro)> {
    let mut out = Vec::new();
    for dummy_required in [false, true] {
        let (group, c) = group_of(kind, n, nf, dummy_required);
        out.extend(Mosfet::enumerate(&group, &c, pdk).into_iter().map(|v| {
            let label = format!(
                "{kind:?} n={n} nf={} dummies={} {:?}",
                v.nf, v.dummies_per_edge, v.style
            );
            (label, v.draw(&group, &c, pdk))
        }));
    }
    out
}

/// Every shape the generator can emit, across both polarities and the group
/// sizes the flow actually asks for.
fn all_variants(pdk: &verify::Pdk) -> Vec<(String, Macro)> {
    let mut out = Vec::new();
    for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
        // Finger counts chosen so the centroid styles are reachable: a pair needs
        // two fingers a side before ABBA exists, a quad needs four.
        for (n, nf) in [(1usize, 1u16), (2, 2), (4, 4)] {
            out.extend(variants(kind, n, nf, pdk));
        }
    }
    out
}

/// Shapes on a named PDK layer.
fn on_layer<'a>(m: &'a Macro, pdk: &verify::Pdk, name: &str) -> Vec<&'a Shape> {
    // Shapes carry PDK-local `LayerId`s; resolve both sides through the deck so
    // the comparison is by *name*, not by whatever index the deck assigned.
    let Some(target) = pdk.gv_layer_by_name(name) else { return Vec::new() };
    m.shapes.iter().filter(|s| pdk.gv_layer(s.layer) == target).collect()
}

fn covers(outer: &Rect, inner: &Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.w >= inner.x + inner.w
        && outer.y + outer.h >= inner.y + inner.h
}

/// Does `cover` (any of them) fully contain `r`?
fn any_covers(cover: &[&Shape], r: &Rect) -> bool {
    cover.iter().any(|c| covers(&c.rect, r))
}

/// Each member's S and D pins split that terminal's DC current exactly
/// (`pnr_core::pin_shares` sums to 1): no finger counted twice, none lost to
/// the `2/n` bound, on every drawn MOS variant (REL-01).
#[test]
fn every_mos_terminal_s_pin_shares_sum_to_one() {
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    for (label, m) in all_variants(&pdk) {
        let shares = pnr_core::pin_shares(&m);
        for u in &m.units {
            for t in ["S", "D"] {
                let name = format!("d{}:{t}", u.owner);
                let sum: f32 = m.pins.iter().zip(&shares).filter(|(p, _)| p.name == name).map(|(_, s)| s).sum();
                assert!((sum - 1.0).abs() < 1e-6, "{label}: {name} pins carry {sum} of the terminal");
            }
        }
    }
}

/// The self-check is only worth its runtime if it covers the axes the placer
/// can actually move along, so pin the axis values themselves. Without this the
/// loops above stay green by covering nothing new: a dropped `dummies = 0`
/// option, or a centroid pattern that never reaches a quad, reads as a pass.
#[test]
fn the_self_check_covers_every_axis_value() {
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    let labels: Vec<String> = all_variants(&pdk).into_iter().map(|(l, _)| l).collect();
    for needle in ["dummies=0", "dummies=1", "Single", "n=2 nf=2 dummies=1 Cc1d", "n=4 nf=4 dummies=1 Cc1d"] {
        assert!(
            labels.iter().any(|l| l.contains(needle)),
            "no drawn variant carries `{needle}` — the checks above pass vacuously \
             for that point of the space. Covered: {labels:?}"
        );
    }
}

#[test]
fn a_mosfet_channel_carries_its_type_implant() {
    // The invariant LVS extraction depends on: a gate crossing diffusion is only
    // identifiable as NMOS or PMOS by the implant covering that channel. Without
    // it the extractor cannot name the device and aborts — which is exactly the
    // "gate polygon crossing channel polygon has no matching MOS type implant"
    // failure the flow hit, and why every circuit reported LVS MISMATCH.
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    for (label, m) in all_variants(&pdk) {
        let diff = on_layer(&m, &pdk, "diff");
        let poly = on_layer(&m, &pdk, "poly");
        assert!(!diff.is_empty(), "{label}: a MOSFET must draw diffusion");
        assert!(!poly.is_empty(), "{label}: a MOSFET must draw poly");

        let nsdm = on_layer(&m, &pdk, "nsdm");
        let psdm = on_layer(&m, &pdk, "psdm");
        assert!(
            !nsdm.is_empty() || !psdm.is_empty(),
            "{label}: a MOSFET must draw an implant layer (nsdm/psdm)"
        );

        // Every diffusion rectangle that a poly stripe crosses is a channel, and
        // each one needs implant cover.
        for d in &diff {
            let crossed = poly.iter().any(|p| {
                let (a, b) = (&d.rect, &p.rect);
                a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
            });
            if !crossed {
                continue;
            }
            let implanted = any_covers(&nsdm, &d.rect) || any_covers(&psdm, &d.rect);
            assert!(
                implanted,
                "{label}: channel diffusion {:?} is crossed by poly but no implant \
                 covers it — LVS extraction cannot determine the MOS type",
                d.rect
            );
        }
    }
}

#[test]
fn every_variant_extracts_unambiguously() {
    // Extraction must resolve a gate-over-diffusion crossing to exactly ONE
    // device type. Two ways to fail, and the flow hit both in turn:
    //   * no implant covers the channel  -> "no matching MOS type implant"
    //   * implants of BOTH polarities do -> "ambiguously matches [nmos, pmos]"
    // Running it against an empty reference is enough: a comparison mismatch is
    // expected and ignored, but an *extraction* failure is the generator's fault
    // and is what this pins.
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    // `device_count` is extract-only: `None` is the extraction abort the old
    // `run_lvs(..).reason` check watched for (implantless or double-implanted
    // channels). The count itself is not pinned here — how many fingers merge
    // into how many devices is LVS's job, not the generator's.
    let mut checker = verify::Checker::new(&pdk, false).expect("deck loads");
    for (label, m) in all_variants(&pdk) {
        assert!(
            checker.device_count(&m.shapes).is_some(),
            "{label}: a device must extract without aborting"
        );
    }
}

#[test]
fn a_pmos_sits_in_a_well_that_encloses_its_diffusion() {
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    // Every PMOS variant, not just the first: the well is derived from the bulk
    // tap span, which is the one thing `dummies_per_edge` moves, so a well that
    // encloses the diffusion at two dummies can still fall short at zero.
    for (n, nf) in [(1usize, 1u16), (2, 2), (4, 4)] {
        for (label, m) in variants(DeviceKind::Pmos, n, nf, &pdk) {
            let nwell = on_layer(&m, &pdk, "nwell");
            assert!(!nwell.is_empty(), "{label}: a PMOS must draw an nwell");
            for d in on_layer(&m, &pdk, "diff") {
                assert!(
                    any_covers(&nwell, &d.rect),
                    "{label}: diffusion {:?} is not enclosed by any nwell rectangle",
                    d.rect
                );
            }
        }
    }
}

#[test]
fn the_bbox_contains_every_drawn_shape() {
    // Placement separates devices by bounding box, so any shape outside the bbox
    // is invisible to the placer and will collide with a neighbour no matter how
    // the legalizer spaces things.
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    for (label, m) in all_variants(&pdk) {
        for s in &m.shapes {
            assert!(
                covers(&m.bbox, &s.rect),
                "{label}: shape {:?} on layer {:?} escapes the macro bbox {:?} — \
                 placement cannot account for geometry it cannot see",
                s.rect,
                s.layer,
                m.bbox
            );
        }
    }
}

/// A PNP's base is an n-well; an NPN's n-well is only its isolation ring,
/// always over a deep n-well (a plain n-well under an NPN would short its
/// base to the collector).
#[test]
fn a_bipolar_draws_its_well_only_as_its_construction() {
    let Some(pdk) = pdk() else { return };
    for n in [1usize, 2, 4] {
        for kind in [DeviceKind::Npn, DeviceKind::Pnp] {
            let (group, c) = group_of(kind, n, 1, false);
            for v in cells::bjt::Bjt::enumerate(&group, &c, &pdk) {
                let label = format!("{kind:?} n={n} cols={}", v.columns);
                let m = v.draw(&group, &c, &pdk);
                let (nwell, deep) = (on_layer(&m, &pdk, "nwell"), on_layer(&m, &pdk, "dnwell"));
                if kind == DeviceKind::Npn {
                    assert!(nwell.is_empty() || !deep.is_empty(), "{label}: an NPN's well only isolates, over a deep well");
                    continue;
                }
                assert!(!nwell.is_empty() && deep.is_empty(), "{label}: a PNP draws the n-well that is its base");
            }
        }
    }
}


/// The MOS generator on every deck: its process numbers come from the deck,
/// so no variant may be clean only on the one it was written against.
///
/// The other generators: [`every_generator_is_clean_on_every_deck`].
/// `generic_finfet` runs in release only: its ERC trips an engine debug
/// assertion (GPurify `electrical.rs`) unrelated to the geometry.
#[cfg(not(debug_assertions))]
#[test]
fn the_mosfet_is_clean_on_every_deck() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirty = Vec::new();
    for deck in ["sky130", "gf180mcu", "ihp_sg13g2", "generic_finfet"] {
        let Ok(json) = std::fs::read_to_string(root.join(format!("pdks/{deck}.json"))) else { continue };
        let pdk = verify::Pdk::from_json(&json).expect("deck loads");
        // A fin deck draws its MOS with the FinFET generator (`cellgen`).
        let fin = pnr_core::Process::layer(&pdk, "fin").is_some();
        let check_mos = |kind, n, nf, w, l, pdk: &verify::Pdk, sp| {
            if fin { check::<cells::finfet::FinFet>(kind, n, nf, w, l, pdk, sp) } else { check::<Mosfet>(kind, n, nf, w, l, pdk, sp) }
        };
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            // The widest finger the deck allows (the flow folds to it).
            let wide = pdk.p2p_max_ohm().zip(pnr_core::Process::sheet_ohm(&pdk, "poly")).map_or(10_000, |(r, sq)| ((0.55 * r / sq * 500.0) as i32).min(10_000));
            for (n, nf, w, l) in [(1usize, 1u16, 1680, 500), (2, 2, 1680, 500), (2, 2, 5000, 1000), (1, 2, wide, 500)] {
                dirty.extend(check_mos(kind, n, nf, w, l, &pdk, SeriesParallel::Parallel).into_iter().map(|d| format!("{deck} {d}")));
            }
            // Series stacks: the chain row.
            for (n, nf) in [(2usize, 1u16), (4, 1), (2, 3)] {
                dirty.extend(check_mos(kind, n, nf, 2000, 500, &pdk, SeriesParallel::Series).into_iter().map(|d| format!("{deck} chain {d}")));
            }
        }
    }
    assert!(dirty.is_empty(), "{}", dirty.join("\n"));
}

/// Every other generator on every deck: resistors, capacitor banks and
/// single capacitors, bipolars, and minimum-size MOS at the deck's own
/// shortest gate.
#[cfg(not(debug_assertions))]
#[test]
fn every_generator_is_clean_on_every_deck() {
    use cells::{bjt::Bjt, cap_array::CapArray, capacitor::Capacitor, resistor::Resistor};
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirty = Vec::new();
    for deck in ["sky130", "gf180mcu", "ihp_sg13g2"] {
        let Ok(json) = std::fs::read_to_string(root.join(format!("pdks/{deck}.json"))) else { continue };
        let pdk = verify::Pdk::from_json(&json).expect("deck loads");
        let lmin = pnr_core::Process::rule(&pdk, "min_gate_l", 150);
        let mut add = |tag: &str, d: Vec<String>| dirty.extend(d.into_iter().map(|d| format!("{deck} {tag} {d}")));
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            for (n, nf, w) in [(1usize, 1u16, 500), (2, 1, 500), (2, 2, 1000)] {
                add("mos", check::<cells::mosfet::Mosfet>(kind, n, nf, w, lmin, &pdk, SeriesParallel::Parallel));
            }
        }
        // Every resistor recipe, drawn as the flow draws it.
        let recipes: Vec<String> = pdk.cell.get("resistors").and_then(|r| r.get("recipes")).and_then(|r| r.as_object()).map_or(Vec::new(), |o| o.values().filter_map(|v| v.get("model")?.as_str().map(String::from)).collect());
        for model in recipes {
            let ov = verify::pdk::Overlay { pdk: &pdk, recipe: pdk.recipe("resistor", &model).expect("recipe") };
            add(&format!("res {model}"), check_on::<Resistor>(DeviceKind::Resistor, vec![1], 500, 2000, &ov, SeriesParallel::Parallel));
            add(&format!("res {model}"), check_on::<Resistor>(DeviceKind::Resistor, vec![2, 2], 500, 4000, &ov, SeriesParallel::Parallel));
        }
        add("cap", check_nf::<Capacitor>(DeviceKind::Capacitor, vec![1], 2000, 2000, &pdk, SeriesParallel::Parallel));
        add("bank", check_nf::<CapArray>(DeviceKind::Capacitor, vec![1, 1, 2, 4, 8], 2000, 2000, &pdk, SeriesParallel::Parallel));
        add("bank", check_nf::<CapArray>(DeviceKind::Capacitor, vec![2, 2], 2000, 2000, &pdk, SeriesParallel::Parallel));
        for kind in [DeviceKind::Pnp, DeviceKind::Npn] {
            add("bjt", check_nf::<Bjt>(kind, vec![1, 8], 5000, 5000, &pdk, SeriesParallel::Parallel));
        }
    }
    assert!(dirty.is_empty(), "{} findings:\n{}", dirty.len(), dirty.join("\n"));
}

/// DRC/ERC findings of every variant of a `n`-member group (dummies on),
/// density and a lone cell's `floating_gate` excepted.
#[cfg(not(debug_assertions))]
fn check<G: Cell>(kind: DeviceKind, n: usize, nf: u16, w: i32, l: i32, pdk: &verify::Pdk, series_parallel: SeriesParallel) -> Vec<String> {
    check_nf::<G>(kind, vec![nf; n], w, l, pdk, series_parallel)
}

#[cfg(not(debug_assertions))]
fn check_nf<G: Cell>(kind: DeviceKind, dev_nf: Vec<u16>, w: i32, l: i32, pdk: &verify::Pdk, series_parallel: SeriesParallel) -> Vec<String> {
    check_on::<G>(kind, dev_nf, w, l, pdk, series_parallel)
}

/// [`check_nf`] drawn through any process view (a recipe overlay), checked
/// against its deck.
#[cfg(not(debug_assertions))]
fn check_on<G: Cell>(kind: DeviceKind, dev_nf: Vec<u16>, w: i32, l: i32, view: &dyn Checked, series_parallel: SeriesParallel) -> Vec<String> {
    let (process, pdk) = (view.process(), view.deck());
    let n = dev_nf.len();
    let nf = dev_nf[0];
    let group = DeviceGroup { devices: (0..n).map(|i| DeviceId(i as u16)).collect() };
    let mut c = analog::Constraints::default();
    c.unitization.push(Unitization {
        devices: group.devices.clone(),
        device_type: kind,
        dev_nf: dev_nf.clone(),
        target_ratio: dev_nf.clone(),
        unit_w: w,
        unit_l: l,
        series_parallel,
        same_variant_required: true,
        dummy_required: true,
        route_matching_required: false,
    });
    G::enumerate(&group, &c, process)
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let m = v.draw(&group, &c, process);
            let mut labels: Vec<verify::LabeledPin> = Vec::new();
            for p in &m.pins {
                let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
                if !labels.iter().any(|q| (q.x, q.y) == (x, y)) {
                    // A terminal in the substrate (a vertical PNP's collector,
                    // a substrate diode's anode) is one net.
                    let name = match p.name.split_once(':') {
                        Some((_, t @ ("G" | "S" | "B"))) => t.to_string(),
                        Some((_, "C")) if kind == DeviceKind::Pnp => "C".to_string(),
                        Some((_, "P")) if kind == DeviceKind::Diode && process.layer("diode_mk").is_none() => "P".to_string(),
                        _ => p.name.replace(':', "_"),
                    };
                    labels.push(verify::LabeledPin { name, layer: p.layer.0, x, y });
                }
            }
            let rules: Vec<String> = verify::drc(&m.shapes, &labels, pdk)
                .into_iter()
                .chain(verify::erc(&m.shapes, &labels, pdk))
                .filter(|f| !f.rule.contains("density") && f.rule != "floating_gate" && !f.rule.starts_with("soft_connection"))
                .map(|f| format!("{}:{}", f.rule, f.layer))
                .collect();
            (!rules.is_empty()).then(|| format!("{kind:?} n={n} nf={nf} w={w} #{i}: {rules:?}"))
        })
        .collect()
}

/// A process view and the deck it is checked against.
#[cfg(not(debug_assertions))]
trait Checked {
    fn process(&self) -> &dyn pnr_core::Process;
    fn deck(&self) -> &verify::Pdk;
}
#[cfg(not(debug_assertions))]
impl Checked for verify::Pdk {
    fn process(&self) -> &dyn pnr_core::Process {
        self
    }
    fn deck(&self) -> &verify::Pdk {
        self
    }
}
#[cfg(not(debug_assertions))]
impl Checked for verify::pdk::Overlay<'_> {
    fn process(&self) -> &dyn pnr_core::Process {
        self
    }
    fn deck(&self) -> &verify::Pdk {
        self.pdk
    }
}
