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
//! 1. **DRC-clean in isolation.** One device alone cannot violate a spacing rule
//!    against anything but itself.
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
    verify::Pdk::from_json(&json).ok()
}

/// The group + constraints for `n` matched devices of `kind`, sized so the
/// enumeration has room to offer several `nf` refolds.
///
/// `dummy_required: false` is deliberate: it is what lets the zero-dummy point
/// of the space exist at all, and a variant nobody ever draws is a variant
/// nobody ever checks.
fn group_of(kind: DeviceKind, n: usize, nf: u16) -> (DeviceGroup, analog::Constraints) {
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
        dummy_required: false,
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
    let (group, c) = group_of(kind, n, nf);
    Mosfet::enumerate(&group, &c, pdk)
        .into_iter()
        .map(|v| {
            let label = format!(
                "{kind:?} n={n} nf={} dummies={} {:?}",
                v.nf, v.dummies_per_edge, v.style
            );
            (label, v.draw(&group, &c, pdk))
        })
        .collect()
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

#[test]
fn every_variant_is_drc_clean() {
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    for (label, m) in all_variants(&pdk) {
        let findings = verify::drc(&m.shapes, &[], &pdk);
        let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
        for f in &findings {
            *by_rule.entry(format!("{}:{}", f.rule, f.layer)).or_default() += 1;
        }
        assert!(
            findings.is_empty(),
            "{label}: a device must be DRC-clean by construction; got {} violations: {:?}",
            findings.len(),
            by_rule
        );
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
    for needle in ["dummies=0", "dummies=1", "dummies=2", "Single", "n=2 nf=2 dummies=1 Cc1d", "n=4 nf=4 dummies=1 Cc1d"] {
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
