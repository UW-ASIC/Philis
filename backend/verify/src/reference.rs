//! The LVS reference netlist, built from verify-owned input structs (D7).
//!
//! `verify` cannot depend on `frontend/library`, so the schematic arrives as
//! the plain [`RefInput`]/[`RefDeviceIn`] surface below — a mirror of the old
//! `RefNetlist` input shape — and is compiled here into gdsverify's SoA
//! [`Netlist`] against the checker's own deck and string table.
//!
//! **Terminal order is SPICE card order**, because gdsverify assigns reference
//! terminal roles by card position (`lvs::graph::card_role`):
//!
//! | kind | terminals, in order |
//! |---|---|
//! | MOS | drain, gate, source, (bulk — only if the deck's recogniser declares 4) |
//! | BJT | collector, base, emitter |
//! | R/C/D | pin a, pin b (symmetric) |
//!
//! **Device params** are stated per card in SI base units (metres), the units
//! gdsverify's `parse_spice` uses. The engine's layout side
//! (`lvs::graph::from_layout_into`) measures a MOS channel's `w`/`l` and emits
//! a layout param only for names the shared string table already carries — so
//! interning a name here is what switches that name's comparison on, and a
//! name stated on one side only is reported as `lvs.undeclared_param`
//! (fail-closed). gdsverify's `reduce_into` refuses to merge any device with a
//! declared param, so a sized card must arrive **one card per drawn finger**
//! (the caller expands m/nf); sized fingers then pair one to one.
//!
//! **Kinds the deck cannot recognise are skipped**, exactly like inductors
//! were before: a deck with no marker for the kind extracts none from the
//! layout, so keeping them in the reference would make LVS mismatch
//! unconditionally. The skip count comes back so callers can log the ceiling.
//! Both Philis decks now carry a `diom` diode recogniser (the generator draws
//! the marker over the junction, terminals are the two `li` pads).
//! **Capacitors remain unrecognisable**: the MOM comb draws both electrodes as
//! many interdigitated polygons on one metal, and `DeviceRecognition` binds
//! exactly one polygon per terminal position (surplus polygons refuse the
//! marker), so comb recognition needs a schema extension — per-terminal
//! *merged-region* binding (all same-net polygons of a layer under the marker
//! count as one electrode). Recognising only the plate kinds would be worse
//! than the skip, because the placer is free to pick the comb variant and the
//! verdict would then depend on variant choice.

use gdsverify::ingest::deck::{Deck, DeviceKind};
use gdsverify::ingest::netlist::{Netlist, RefNetId, SubcktId};
use gdsverify::ingest::{StrId, StrTable};

/// A schematic device kind, polarity included (the deck's recognisers are
/// per-polarity: `ngate` vs `pgate`, `npn` vs `pnp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Nmos,
    Pmos,
    Npn,
    Pnp,
    Resistor,
    Capacitor,
    Diode,
}

/// One schematic device, as `frontend/library` states it.
#[derive(Clone, Debug)]
pub struct RefDeviceIn {
    pub kind: RefKind,
    /// Optional deck model name (e.g. `"sky130_fd_pr__nfet_01v8"`). When given
    /// it selects the recogniser row whose model matches; when absent the
    /// first recogniser of the matching kind/polarity is used.
    pub model: Option<String>,
    /// Terminal **net names**, in SPICE card order (see module doc). Must be at
    /// least as many as the selected recogniser's terminal arity; extras are
    /// ignored (an NMOS card may carry a bulk net the 3-terminal recogniser
    /// does not extract).
    pub terminals: Vec<String>,
    /// Declared parameters, `(spice name, value)` in SI base units — `"w"`
    /// and `"l"` in metres for a MOS. Interned into the checker's table, which
    /// is what arms the layout side's measured-param emission for that name
    /// (see the module doc). One card = one drawn finger: the caller expands
    /// m/nf, because a device with params never parallel-merges.
    pub params: Vec<(String, f64)>,
}

/// The whole reference: devices plus the top cell's port (pin) net names.
#[derive(Clone, Debug, Default)]
pub struct RefInput {
    pub devices: Vec<RefDeviceIn>,
    /// Net names that are pins of the cell — must match the names the
    /// [`crate::geom::LabeledPin`]s put on the drawn geometry.
    pub ports: Vec<String>,
}

/// Compile `input` into a one-subckt (`"top"`) gdsverify [`Netlist`], interning
/// into `strings` (which must be the **checker's** table, so reference net
/// names and layout label names share one id space).
///
/// Returns the netlist and how many devices were skipped for want of a deck
/// recogniser (the documented LVS ceiling).
///
/// # Errors
/// A device whose terminals are fewer than its recogniser's arity.
pub fn build(
    input: &RefInput,
    deck: &Deck,
    strings: &mut StrTable,
) -> Result<(Netlist, usize), String> {
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

    let mut skipped = 0usize;
    n.subckt_device_start.push(0);
    n.device_terminal_start.push(0);
    n.device_param_start.push(0);
    for (index, dev) in input.devices.iter().enumerate() {
        let Some(row) = recogniser_for(dev, deck, strings) else {
            // No marker layer in this deck extracts this device from the
            // layout, so the reference must not expect it either.
            skipped += 1;
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

        n.device_name.push(strings.intern(&format!("d{index}")));
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
    n.subckt_device_start.push(n.device_name.len() as u32);

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

/// The deck recogniser row for one schematic device, or `None` when the deck
/// has no marker for its kind/polarity.
///
/// Polarity is read off the **marker layer name**: both Philis decks name
/// their MOS markers `ngate`/`pgate` and their BJT markers `npn`/`pnp`, so a
/// leading `p` is P-type. A model hint, when given, overrides and must match
/// exactly.
fn recogniser_for(dev: &RefDeviceIn, deck: &Deck, strings: &StrTable) -> Option<usize> {
    let (kind, polarity) = match dev.kind {
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
        if hinted.is_some() {
            if Some(deck.devices.model[row]) == hinted {
                return Some(row);
            }
            // Keep scanning for the hinted model; remember the first
            // kind/polarity match in case the hint names no deck model.
        } else {
            return Some(row);
        }
        fallback.get_or_insert(row);
    }
    fallback
}
