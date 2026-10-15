//! Generate one typed PDK module per built-in deck Substrate2 does not ship
//! (`src/pdk/mod.rs` `define_pdk!`): its drawn layers and MOS models, read
//! through `verify` so the deck stays the one source of truth.

use std::collections::HashSet;
use std::fmt::Write as _;

/// `(module, type, verify::Pdk::builtin name)`.
const PDKS: [(&str, &str, &str); 3] =
    [("gf180mcu", "Gf180mcu", "gf180mcu"), ("ihp_sg13g2", "IhpSg13g2", "ihp_sg13g2"), ("asap7", "Asap7", "asap7")];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../pdks");
    let mut out = String::new();
    for (module, ty, deck) in PDKS {
        let pdk = verify::Pdk::builtin(deck).unwrap_or_else(|e| panic!("{deck}: {e}"));
        let gds = pdk.layer_gds();

        let mut taken = HashSet::new();
        let mut layers = String::new();
        let mut pairs = HashSet::new();
        for (name, id) in &pdk.layers {
            let (l, d) = gds[id.0 as usize];
            // Computed layers have no stream pair; a pair two names share maps once.
            if (l, d) == (0, 0) || !pairs.insert((l, d)) {
                continue;
            }
            let _ = write!(layers, "({}, {name:?}, {l}, {d}), ", ident(name, &mut taken));
        }

        let mut taken = HashSet::new();
        let mut models = String::new();
        let mut seen = HashSet::new();
        for &m in &pdk.deck.devices.model {
            let name = pdk.strings.resolve(m);
            let pmos = match pdk.model_kind(name) {
                Some(verify::RefKind::Nmos) => false,
                Some(verify::RefKind::Pmos) => true,
                _ => continue,
            };
            if seen.insert(name.to_string()) {
                let _ = write!(models, "({}, {name:?}, {pmos}), ", ident(name, &mut taken));
            }
        }
        let _ = writeln!(out, "define_pdk!({module}, {ty}, {deck:?}, [{layers}], [{models}]);");
    }
    let path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("pdks.rs");
    std::fs::write(path, out).unwrap();
}

/// `metal1` → `Metal1`, `sg13_lv_nmos` → `Sg13LvNmos`; a clash gets a suffix.
fn ident(name: &str, taken: &mut HashSet<String>) -> String {
    let mut s: String = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next().map_or(String::new(), |f| f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase())
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, 'L');
    }
    let base = s.clone();
    let mut k = 2;
    while !taken.insert(s.clone()) {
        s = format!("{base}{k}");
        k += 1;
    }
    s
}
