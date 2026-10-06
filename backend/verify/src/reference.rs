//! The LVS reference netlist, compiled from plain [`RefInput`] (verify cannot
//! depend on `frontend/library`) into gdsverify's SoA [`Netlist`].
//!
//! - **Terminals are in SPICE card order** — gdsverify assigns roles by
//!   position: MOS `d g s [b]` (bulk only if the recogniser declares 4),
//!   BJT `c b e`, R/C/D `a b`.
//! - **Params are SI** (metres). Interning a param name is what makes the
//!   layout side emit its measured value, so a name on one side only is an
//!   `lvs.undeclared_param`. A device with params never parallel-merges, so the
//!   caller must expand m/nf to one card per drawn finger.
//! - **Kinds the deck cannot recognise are skipped** (returned by index):
//!   the layout extracts none, so keeping them would mismatch unconditionally.
//!   The caller counts them as LVS-unverified ([`crate::Coverage`]), never as
//!   matched. A MOM capacitor (no deck recognises one: a comb draws each
//!   electrode as many polygons and `DeviceRecognition` binds one polygon per
//!   terminal), a BJT on a deck without one, and every inductor are the cases.

use gdsverify::ingest::deck::{Deck, DeviceKind};
use gdsverify::ingest::netlist::{Netlist, RefNetId, SubcktId};
use gdsverify::ingest::{StrId, StrTable};

/// A schematic device kind, polarity included (recognisers are per-polarity).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    /// N-channel MOSFET, card order `d g s [b]`.
    Nmos,
    /// P-channel MOSFET, card order `d g s [b]`.
    Pmos,
    /// NPN bipolar, card order `c b e`.
    Npn,
    /// PNP bipolar, card order `c b e`.
    Pnp,
    /// Two-terminal resistor.
    Resistor,
    /// Two-terminal capacitor; skipped without a model (see
    /// [`RefDeviceIn::model`]).
    Capacitor,
    /// Two-terminal diode, card order anode, cathode.
    Diode,
    /// No deck recognises one: always skipped.
    Inductor,
}

/// One schematic device.
#[derive(Clone, Debug)]
pub struct RefDeviceIn {
    /// Kind and polarity; picks the deck recogniser family.
    pub kind: RefKind,
    /// Deck model name; selects the matching recogniser row. `None` takes the
    /// first recogniser of the kind/polarity (a capacitor is skipped); a name
    /// no row matches takes it too for MOS/BJT, and is skipped for R/C/D (a
    /// MOM is not a MIM).
    pub model: Option<String>,
    /// Terminal net names in card order; at least the recogniser's arity,
    /// extras (e.g. a bulk the recogniser does not extract) ignored.
    pub terminals: Vec<String>,
    /// `(spice name, value)` in SI units, one card per drawn finger.
    pub params: Vec<(String, f64)>,
}

/// The whole reference: devices plus the cell's port net names (which must
/// match the [`crate::geom::LabeledPin`] names on the geometry).
#[derive(Clone, Debug, Default)]
pub struct RefInput {
    /// Schematic devices, one card per drawn finger.
    pub devices: Vec<RefDeviceIn>,
    /// Every labelled net name, in the order they become reference ports.
    pub ports: Vec<String>,
    /// The nets that leave the block (the `.subckt` port list), apart from
    /// `ports` (every labelled net, for LVS naming): only these are exempt
    /// from `floating_gate`/`unconnected_pin`. `None`: no port list, every
    /// labelled net is exempt and reported so in the coverage (AV-04).
    pub external_ports: Option<Vec<String>>,
}

/// Compiles `input` into a one-subckt (`"top"`) [`Netlist`], interning into the
/// **checker's** `strings` so reference and layout names share one id space.
/// Returns the netlist and the indices into `input.devices` of the devices
/// skipped for want of a recogniser, ascending. Nets are numbered in
/// first-seen order over the ports, then each kept device's terminals; a
/// name repeated in `ports` is one net and one port.
///
/// # Errors
/// A device whose terminals are fewer than its recogniser's arity.
pub fn build(
    input: &RefInput,
    deck: &Deck,
    strings: &mut StrTable,
) -> Result<(Netlist, Vec<usize>), String> {
    let mut n = Netlist::default();
    n.subckt_name.push(strings.intern("top"));

    // Deterministic net numbering: first-seen order over ports then devices.
    let mut net_names: Vec<StrId> = Vec::new();
    let net_of = |name: &str, strings: &mut StrTable, nets: &mut Vec<StrId>| -> RefNetId {
        let id = strings.intern(name);
        let at = nets.iter().position(|&existing| existing == id).unwrap_or_else(|| {
            nets.push(id);
            nets.len() - 1
        });
        RefNetId(at as u32)
    };

    n.subckt_port_start.push(0);
    for port in &input.ports {
        let net = net_of(port, strings, &mut net_names);
        n.port_net.push(net);
    }
    n.subckt_port_start.push(n.port_net.len() as u32);

    let mut skipped = Vec::new();
    n.subckt_device_start.push(0);
    n.device_terminal_start.push(0);
    n.device_param_start.push(0);
    for (index, dev) in input.devices.iter().enumerate() {
        let Some(row) = recogniser_for(dev, deck, strings) else {
            // No marker layer in this deck extracts this device from the
            // layout, so the reference must not expect it either.
            skipped.push(index);
            continue;
        };
        let arity = (deck.devices.terminal_start[row + 1] - deck.devices.terminal_start[row])
            as usize;
        if dev.terminals.len() < arity {
            return Err(format!(
                "reference device {index} ({:?}) states {} terminals, its deck recogniser \
                 needs {arity}",
                dev.kind,
                dev.terminals.len()
            ));
        }

        n.device_model.push(deck.devices.model[row]);
        n.device_kind.push(deck.devices.kind[row]);
        for name in &dev.terminals[..arity] {
            let net = net_of(name, strings, &mut net_names);
            n.terminal_net.push(net);
        }
        n.device_terminal_start.push(n.terminal_net.len() as u32);
        for (name, value) in &dev.params {
            n.param.push((strings.intern(name), *value));
        }
        n.device_param_start.push(n.param.len() as u32);
    }
    n.subckt_device_start.push(n.device_model.len() as u32);

    n.net_name = net_names;
    n.net_subckt = vec![SubcktId(0); n.net_name.len()];

    debug_assert_eq!(n.subckt_count(), 1);
    debug_assert_eq!(
        n.device_param_start.last().copied().unwrap_or(0) as usize,
        n.param.len(),
        "the param CSR's terminator and its rows disagree"
    );
    Ok((n, skipped))
}

/// Returns the deck recogniser row for one schematic device, `None` when the deck has
/// no marker for its kind/polarity. Polarity is read off the marker layer name:
/// its first `_`-separated segment led by `n` or `p` (`ngate`/`pgate`,
/// `npn`/`pnp`, sky130's `esd_pfet_…`). A MOS/BJT row with no such segment
/// (IHP's `esd_vdd` bjt, an ESD diode) is taken only by an exact or vendor
/// model match, never by a model-less card or the fallback. For R/C/D a model
/// hint no row names (exactly or as a `__` vendor suffix) is `None`: sky130's
/// `cap_generic_m1m2` (MOM) must not be compared as its `capm` (MIM). So is a
/// capacitor with no model (an elaborated composition's card): every deck's
/// capacitor row is a MIM or MOS cap, never the MOM the generators draw. A
/// model-less R/D still takes the first row (antenna diodes carry no model).
fn recogniser_for(dev: &RefDeviceIn, deck: &Deck, strings: &StrTable) -> Option<usize> {
    let (kind, polarity) = match dev.kind {
        RefKind::Inductor => return None,
        RefKind::Capacitor if dev.model.is_none() => return None,
        RefKind::Nmos => (DeviceKind::Mos, Some(false)),
        RefKind::Pmos => (DeviceKind::Mos, Some(true)),
        RefKind::Npn => (DeviceKind::Bjt, Some(false)),
        RefKind::Pnp => (DeviceKind::Bjt, Some(true)),
        RefKind::Resistor => (DeviceKind::Resistor, None),
        RefKind::Capacitor => (DeviceKind::Capacitor, None),
        RefKind::Diode => (DeviceKind::Diode, None),
    };
    let hinted = dev.model.as_deref().and_then(|m| strings.get(m));
    let mut fallback = None;
    for row in 0..deck.devices.kind.len() {
        if deck.devices.kind[row] != kind {
            continue;
        }
        if let Some(want_p) = polarity {
            let marker = strings.resolve(deck.layers.name(deck.devices.marker[row]));
            if marker.starts_with('p') != want_p {
                continue;
            }
        }
        let named = strings.resolve(deck.devices.model[row]);
        let vendor = dev.model.as_deref().is_some_and(|m| named.ends_with(&format!("__{m}")) || m.ends_with(&format!("__{named}")));
        if hinted.is_none() && dev.model.is_none() || hinted == Some(deck.devices.model[row]) || vendor {
            return Some(row);
        }
        // The hint names no deck model (yet): a MOS/BJT falls back to the
        // first match whose marker states its polarity (IHP's `esd_vdd` bjt
        // is an ESD diode, no NPN a schematic may mean).
        let marker = strings.resolve(deck.layers.name(deck.devices.marker[row]));
        if polarity.is_some() && (marker.starts_with('n') || marker.starts_with('p')) {
            fallback.get_or_insert(row);
        }
    }
    fallback
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::{Checker, Pdk};

    fn checker(pdk: &str) -> Checker {
        Checker::new(&Pdk::builtin(pdk).unwrap(), true).unwrap()
    }

    fn dev(kind: RefKind, model: Option<&str>, terminals: &[&str]) -> RefDeviceIn {
        RefDeviceIn {
            kind,
            model: model.map(Into::into),
            terminals: terminals.iter().map(|&t| t.into()).collect(),
            params: vec![],
        }
    }

    /// The model name of the row `recogniser_for` picks, `None` when skipped.
    fn picks(c: &Checker, kind: RefKind, model: Option<&str>) -> Option<String> {
        let (deck, st) = (&c.loaded.deck, &c.loaded.strings);
        recogniser_for(&dev(kind, model, &[]), deck, st).map(|r| st.resolve(deck.devices.model[r]).to_string())
    }

    #[test]
    fn an_empty_reference_is_one_empty_subckt() {
        let mut c = checker("sky130");
        let (n, skipped) = build(&RefInput::default(), &c.loaded.deck, &mut c.loaded.strings).unwrap();
        assert!(skipped.is_empty());
        assert_eq!(n.subckt_count(), 1);
        assert_eq!(c.loaded.strings.resolve(n.subckt_name[0]), "top");
        assert_eq!(n.subckt_port_start, [0, 0]);
        assert_eq!(n.subckt_device_start, [0, 0]);
        assert_eq!(n.device_terminal_start, [0]);
        assert_eq!(n.device_param_start, [0]);
        assert!(n.net_name.is_empty() && n.net_subckt.is_empty() && n.port_net.is_empty());
    }

    // Nets number first-seen over ports then terminals; a repeated port is
    // one net and one port.
    #[test]
    fn nets_number_in_first_seen_order() {
        let mut c = checker("sky130");
        let input = RefInput {
            devices: vec![dev(RefKind::Nmos, None, &["x", "a", "y", "x"])],
            ports: vec!["a".into(), "b".into(), "a".into()],
            external_ports: None,
        };
        let (n, _) = build(&input, &c.loaded.deck, &mut c.loaded.strings).unwrap();
        let names: Vec<&str> = n.net_name.iter().map(|&id| c.loaded.strings.resolve(id)).collect();
        assert_eq!(names, ["a", "b", "x", "y"]);
        assert_eq!(n.port_net, [RefNetId(0), RefNetId(1)], "a repeated port is one port");
        assert_eq!(n.subckt_port_start, [0, 2]);
        assert_eq!(n.terminal_net, [RefNetId(2), RefNetId(0), RefNetId(3), RefNetId(2)]);
        assert_eq!(n.net_subckt, vec![SubcktId(0); 4]);
    }

    // Terminals past the recogniser's arity are ignored.
    #[test]
    fn extra_terminals_are_truncated_to_the_arity() {
        let mut c = checker("ihp_sg13g2");
        let input = RefInput { devices: vec![dev(RefKind::Nmos, None, &["d", "g", "s", "b"])], ..Default::default() };
        let (n, _) = build(&input, &c.loaded.deck, &mut c.loaded.strings).unwrap();
        assert_eq!(n.device_terminal_start, [0, 3], "ihp MOS recognisers take d g s");
        assert!(n.net_name.iter().all(|&id| c.loaded.strings.resolve(id) != "b"), "the bulk names no net");
    }

    #[test]
    fn too_few_terminals_is_an_error_naming_the_device() {
        let mut c = checker("sky130");
        let input = RefInput {
            devices: vec![dev(RefKind::Inductor, None, &[]), dev(RefKind::Resistor, None, &["a"])],
            ..Default::default()
        };
        let err = build(&input, &c.loaded.deck, &mut c.loaded.strings).unwrap_err();
        assert!(err.contains("reference device 1") && err.contains("needs 2"), "{err}");
    }

    // Skipped devices leave no row and no CSR entry; kept ones keep their params.
    #[test]
    fn skipped_devices_leave_no_rows() {
        let mut c = checker("sky130");
        let mut r = dev(RefKind::Resistor, None, &["a", "b"]);
        r.params = vec![("r".into(), 1e3)];
        let input = RefInput {
            devices: vec![dev(RefKind::Inductor, None, &["p", "q"]), r, dev(RefKind::Capacitor, None, &["c", "d"])],
            ..Default::default()
        };
        let (n, skipped) = build(&input, &c.loaded.deck, &mut c.loaded.strings).unwrap();
        assert_eq!(skipped, [0, 2]);
        assert_eq!(n.device_model.len(), 1);
        assert_eq!(n.device_kind, [DeviceKind::Resistor]);
        assert_eq!(n.device_terminal_start, [0, 2]);
        assert_eq!(n.device_param_start, [0, 1]);
        assert_eq!(n.subckt_device_start, [0, 1]);
        let names: Vec<&str> = n.net_name.iter().map(|&id| c.loaded.strings.resolve(id)).collect();
        assert_eq!(names, ["a", "b"], "a skipped device's terminals name no net");
    }

    #[test]
    fn recogniser_rows_on_sky130() {
        let c = checker("sky130");
        let p = |kind, model| picks(&c, kind, model);
        assert_eq!(p(RefKind::Inductor, Some("anything")), None);
        assert_eq!(p(RefKind::Nmos, None).as_deref(), Some("sky130_fd_pr__nfet_01v8"));
        assert_eq!(p(RefKind::Pmos, None).as_deref(), Some("sky130_fd_pr__pfet_01v8"));
        assert_eq!(p(RefKind::Npn, None).as_deref(), Some("sky130_fd_pr__npn_05v5_W1p00L1p00"));
        assert_eq!(p(RefKind::Pnp, None).as_deref(), Some("sky130_fd_pr__pnp_05v5_W3p40L3p40"));
        // Exact name, vendor suffix either way round, and an unknown MOS name
        // falling back to the polarity's first row.
        assert_eq!(p(RefKind::Nmos, Some("sky130_fd_pr__nfet_01v8_lvt")).as_deref(), Some("sky130_fd_pr__nfet_01v8_lvt"));
        assert_eq!(p(RefKind::Nmos, Some("nfet_01v8_lvt")).as_deref(), Some("sky130_fd_pr__nfet_01v8_lvt"));
        assert_eq!(p(RefKind::Nmos, Some("no_such_fet")).as_deref(), Some("sky130_fd_pr__nfet_01v8"));
        assert_eq!(p(RefKind::Pmos, Some("no_such_fet")).as_deref(), Some("sky130_fd_pr__pfet_01v8"));
        // A name without the `__` separator is not a vendor suffix.
        assert_eq!(p(RefKind::Resistor, Some("generic_po")), None);
        // R/C/D: an unknown name is skipped, no name takes the first row
        // (a capacitor excepted).
        assert_eq!(p(RefKind::Resistor, None).as_deref(), Some("sky130_fd_pr__res_generic_po"));
        assert_eq!(p(RefKind::Resistor, Some("no_such_res")), None);
        assert_eq!(p(RefKind::Diode, None).as_deref(), Some("sky130_fd_pr__diode_pw2nd_05v5"));
        assert_eq!(p(RefKind::Capacitor, None), None);
        assert_eq!(p(RefKind::Capacitor, Some("cap_generic_m1m2")), None);
        assert_eq!(p(RefKind::Capacitor, Some("cap_mim_m3_1")).as_deref(), Some("sky130_fd_pr__cap_mim_m3_1"));
        // A hint of another kind's model does not cross kinds.
        assert_eq!(p(RefKind::Resistor, Some("sky130_fd_pr__cap_mim_m3_1")), None);
    }

    // The polarity of an ESD fet is in its name past the `esd_` prefix: an
    // esd pfet is P-type, and never an NMOS's row.
    #[test]
    fn an_esd_fet_keeps_its_polarity() {
        let c = checker("sky130");
        let p = |kind, model| picks(&c, kind, model);
        assert_eq!(p(RefKind::Pmos, Some("sky130_fd_pr__esd_pfet_g5v0d10v5")).as_deref(), Some("sky130_fd_pr__esd_pfet_g5v0d10v5"));
        assert_eq!(p(RefKind::Nmos, Some("sky130_fd_pr__esd_nfet_g5v0d10v5")).as_deref(), Some("sky130_fd_pr__esd_nfet_g5v0d10v5"));
        assert_eq!(p(RefKind::Nmos, Some("sky130_fd_pr__esd_pfet_g5v0d10v5")).as_deref(), Some("sky130_fd_pr__nfet_01v8"));
    }

    // IHP has no NPN: its `esd_vdd`/`esd_vss` bjt rows are ESD diodes, so a
    // model-less NPN/PNP is skipped rather than compared against one.
    #[test]
    fn ihp_has_no_bipolar_for_a_modelless_card() {
        let c = checker("ihp_sg13g2");
        assert_eq!(picks(&c, RefKind::Npn, None), None);
        assert_eq!(picks(&c, RefKind::Pnp, None), None);
        assert_eq!(picks(&c, RefKind::Npn, Some("no_such_npn")), None);
        assert_eq!(picks(&c, RefKind::Pmos, None).as_deref(), Some("sg13_lv_pmos"));
        assert_eq!(picks(&c, RefKind::Nmos, Some("sg13_hv_nmos")).as_deref(), Some("sg13_hv_nmos"));
        // A vendor-prefixed schematic name matches the deck's bare one.
        assert_eq!(picks(&c, RefKind::Nmos, Some("lib__sg13_hv_nmos")).as_deref(), Some("sg13_hv_nmos"));
    }
}
