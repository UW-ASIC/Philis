//! # `cells` — automatic device geometry
//!
//! Turns one [`DeviceGroup`] into drawn [`Macro`] variants: a MOSFET stack, a
//! resistor, a capacitor array, a BJT, a diode, an inductor. One generator per
//! family, one file each. Every generator reads the group's sizing from its
//! covering [`analog::cell::Unitization`] and every layer/rule from the
//! [`Process`]; `draw` is pure and byte-deterministic.
//!
//! Pins are named `d{i}:{T}`: `i` is the member's index in `group.devices`,
//! `T` its schematic terminal (`G/D/S/B`, `P/N`, `C/B/E`). Pin nets are
//! synthetic placeholders that the caller rebinds by name.

pub mod bjt;
pub mod builder;
pub mod cap_array;
pub mod capacitor;
pub mod diode;
pub mod finfet;
pub mod inductor;
pub mod mosfet;
pub mod post_cell;
pub mod resistor;

pub use builder::Builder;

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process};

/// How the fingers/segments of a matched group interleave.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// Each device's fingers contiguous.
    Single,
    /// One-dimensional common centroid (ABBA).
    Cc1d,
    /// Simple interdigitation (ABAB).
    Interdig,
    /// A series stack (MOS): members in order, each drawn drain-left, a
    /// member's source region shared with the next one's drain (Razavi Fig.
    /// 19.12). Every member needs an odd finger count.
    Chain,
}

/// A device-family generator over its variant space.
pub trait Cell: Clone {
    /// Every feasible variant for `group`, in a deterministic order (the order
    /// `Layout::variant` indexes). Never ranked: the placer picks.
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self>;

    /// Draw this variant.
    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro;
}

/// Shared helpers for each generator's in-file DRC/ERC self-check: one device
/// group, drawn alone, so a finding is unambiguously the generator's.
#[cfg(test)]
pub(crate) mod testkit {
    use analog::cell::{SeriesParallel, Unitization};
    use pnr_core::{DeviceGroup, DeviceId, DeviceKind};

    /// The sky130 deck; `None` (skip) when absent, panic when present but broken.
    pub fn pdk() -> Option<verify::Pdk> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let json = std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?;
        Some(verify::Pdk::from_json(&json).expect("pdks/sky130.json loads"))
    }

    /// `n` matched devices of `kind` at `nf` units of `w`×`l` nm each.
    pub fn group_of(kind: DeviceKind, n: usize, nf: u16, w: i32, l: i32) -> (DeviceGroup, analog::Constraints) {
        let group = DeviceGroup { devices: (0..n).map(|i| DeviceId(i as u16)).collect() };
        let mut c = analog::Constraints::default();
        c.unitization.push(Unitization {
            devices: group.devices.clone(),
            device_type: kind,
            dev_nf: vec![nf; n],
            target_ratio: vec![1; n],
            unit_w: w,
            unit_l: l,
            series_parallel: SeriesParallel::Parallel,
            same_variant_required: true,
            // `false` keeps the zero-dummy variants in the sweep.
            dummy_required: false,
            route_matching_required: false,
        });
        (group, c)
    }

    /// DRC + ERC findings over every variant `G` offers for the group, one line
    /// per dirty variant. Density rules are chip-level and skipped.
    /// Swept with and without dummies: the count follows `dummy_required`.
    pub fn dirty<G: crate::Cell>(kind: DeviceKind, n: usize, nf: u16, w: i32, l: i32, pdk: &verify::Pdk) -> Vec<String> {
        let (group, mut c) = group_of(kind, n, nf, w, l);
        let mut out = Vec::new();
        for dummies in [false, true] {
            c.unitization[0].dummy_required = dummies;
            out.extend(
                dirty_group::<G>(&group, &c, pdk)
                    .into_iter()
                    .map(|d| format!("{kind:?} n={n} nf={nf} dummies={dummies} {d}")),
            );
        }
        out
    }

    /// A lone cell's DRC + ERC findings as `rule:layer`. Density is
    /// chip-level; its gates have no driver (`floating_gate` is vacuous); its
    /// taps meet only through the substrate until routing ties them
    /// (`soft_connection`), and a two-ended gate's straps only through poly.
    pub fn findings(shapes: &[pnr_core::Shape], labels: &[verify::LabeledPin], pdk: &verify::Pdk) -> Vec<String> {
        verify::drc(shapes, labels, pdk)
            .into_iter()
            .chain(verify::erc(shapes, labels, pdk))
            .filter(|f| !f.rule.ends_with("_density") && !(f.rule == "floating_gate" || f.rule.starts_with("soft_connection")))
            .map(|f| format!("{}:{}", f.rule, f.layer))
            .collect()
    }

    /// [`dirty`] for a caller-built group, pins read as a current mirror's
    /// ([`dirty_group_with`] over G, S, B).
    pub fn dirty_group<G: crate::Cell>(group: &DeviceGroup, c: &analog::Constraints, pdk: &verify::Pdk) -> Vec<String> {
        dirty_group_with::<G>(group, c, pdk, &["G", "S", "B"])
    }

    /// [`dirty`] for a caller-built group whose `common` terminals are one net
    /// across members and every other terminal is private to its member, so a
    /// short between two members' private terminals is an ERC finding.
    pub fn dirty_group_with<G: crate::Cell>(group: &DeviceGroup, c: &analog::Constraints, pdk: &verify::Pdk, common: &[&str]) -> Vec<String> {
        let variants = G::enumerate(group, c, pdk);
        assert!(!variants.is_empty(), "no variants to check");
        // A terminal in the substrate (a vertical PNP's collector, a substrate
        // diode's anode) is one net for every member, like the bulk.
        use pnr_core::Process;
        let mut common = common.to_vec();
        common.extend(match c.unitization.first().map(|u| u.device_type) {
            Some(DeviceKind::Pnp) => Some("C"),
            Some(DeviceKind::Diode) if pdk.layer("diode_mk").is_none() => Some("P"),
            _ => None,
        });
        variants
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let m = v.draw(group, c, pdk);
                let rules = findings(&m.shapes, &ports_with(&m, &common), pdk);
                (!rules.is_empty()).then(|| format!("#{i}: {rules:?}"))
            })
            .collect()
    }

    /// Each pin as a port label at its centre, so ERC reads a device's
    /// terminals as its interface rather than floating metal. One label per
    /// pad: shared-diffusion pads carry several members' pins. A pin whose
    /// terminal is in `common` is named by the terminal alone (one net for
    /// every member); any other is `d{i}_{T}`, private to member `i`.
    pub fn ports_with(m: &pnr_core::Macro, common: &[&str]) -> Vec<verify::LabeledPin> {
        let mut out: Vec<verify::LabeledPin> = Vec::new();
        for p in &m.pins {
            let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
            if !out.iter().any(|l| (l.x, l.y, l.layer) == (x, y, p.layer.0)) {
                let name = match p.name.split_once(':') {
                    Some((_, t)) if common.contains(&t) => t.to_string(),
                    _ => p.name.replace(':', "_"),
                };
                out.push(verify::LabeledPin { name, layer: p.layer.0, x, y });
            }
        }
        out
    }
}
