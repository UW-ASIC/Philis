//! What a terminal is, by its name and its device's kind (AA-16): never by position.
use pnr_core::DeviceKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TermRole { FetGate, Channel, Body, BjtBase, Plate, Passive }

/// FET `G`/`D`,`S`/`B`; bipolar `B` = base, `C`/`E` = channel; capacitor pins are plates;
/// anything else (resistor, diode, inductor, unknown pin names) is a passive DC path.
#[must_use]
pub fn term_role(kind: DeviceKind, term: &str) -> TermRole {
    use DeviceKind::{Capacitor, Nmos, Npn, Pmos, Pnp};
    match (kind, term) {
        (Nmos | Pmos, "G") => TermRole::FetGate,
        (Nmos | Pmos, "D" | "S") => TermRole::Channel,
        (Nmos | Pmos, "B") => TermRole::Body,
        (Npn | Pnp, "B") => TermRole::BjtBase,
        (Npn | Pnp, _) => TermRole::Channel,
        (Capacitor, _) => TermRole::Plate,
        _ => TermRole::Passive,
    }
}
