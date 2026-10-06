//! What a terminal is, by its name and its device's kind (AA-16): never by position.
use pnr_core::DeviceKind;

/// The electrical role of one device terminal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TermRole {
    /// A MOS gate: capacitive load, no DC path.
    FetGate,
    /// A MOS drain/source or a bipolar collector/emitter: carries the device current.
    Channel,
    /// A MOS bulk: a junction to the well or substrate.
    Body,
    /// A bipolar base: a DC path carrying base current.
    BjtBase,
    /// A capacitor terminal: no DC path.
    Plate,
    /// Any other pin (resistor, diode, inductor, unknown names): a passive DC path.
    Passive,
}

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

/// Step-2 coverage: the full role table.
#[cfg(test)]
mod cleanup_tests {
    use super::*;

    #[test]
    fn roles_by_kind_and_name() {
        use DeviceKind::*;
        use TermRole::*;
        let cases = [
            (Nmos, "G", FetGate),
            (Pmos, "D", Channel),
            (Nmos, "S", Channel),
            (Pmos, "B", Body),
            (Nmos, "X", Passive),
            (Nmos, "g", Passive),
            (Npn, "B", BjtBase),
            (Pnp, "C", Channel),
            (Npn, "E", Channel),
            (Capacitor, "P", Plate),
            (Capacitor, "B", Plate),
            (Resistor, "P", Passive),
            (Diode, "N", Passive),
            (Resistor, "G", Passive),
        ];
        for (k, t, want) in cases {
            assert_eq!(term_role(k, t), want, "{k:?} {t}");
        }
    }
}
