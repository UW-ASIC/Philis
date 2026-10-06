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
    Nmos,
    Pmos,
    Npn,
    Pnp,
    Resistor,
    Capacitor,
    Diode,
    /// No deck recognises one: always skipped.
    Inductor,
}

/// One schematic device.
#[derive(Clone, Debug)]
pub struct RefDeviceIn {
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
    pub devices: Vec<RefDeviceIn>,
    pub ports: Vec<String>,
    /// The nets that leave the block (the `.subckt` port list), apart from
    /// `ports` (every labelled net, for LVS naming): only these are exempt
    /// from `floating_gate`/`unconnected_pin`. `None`: no port list, every
    /// labelled net is exempt and reported so in the coverage (AV-04).
    pub external_ports: Option<Vec<String>>,
}

/// Compile `input` into a one-subckt (`"top"`) [`Netlist`], interning into the
/// **checker's** `strings` so reference and layout names share one id space.
/// Returns the netlist and the indices into `input.devices` of the devices
/// skipped for want of a recogniser.
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

/// The deck recogniser row for one schematic device, `None` when the deck has
/// no marker for its kind/polarity. Polarity is read off the marker layer name
/// (`ngate`/`pgate`, `npn`/`pnp`: a leading `p` is P-type). For R/C/D a model
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
