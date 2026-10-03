//! What a device draws, as the matcher and the unitization classes compare it:
//! size, model flavour and bulk net. The one reader of the size convention
//! ([`pnr_core::MosSize`], FLOW-01) inside the annotator's matching.

use pnr_core::ids::NetId;
use pnr_core::netlist::{Device, DeviceKind};

use crate::netrole::NetRole;
use crate::param;

/// A device's drawn identity. An unknown size is `None`, never `0`: two
/// devices missing W/L are not "identical" (AA-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub w_finger_nm: Option<i64>,
    pub l_nm: Option<i64>,
    pub fingers: u32,
    /// Index into the `models` table [`drawn`] interns into (case-insensitive).
    pub model: u16,
    /// The FET `B` net; `None` for anything else (a bipolar's `B` is its base).
    pub bulk: Option<NetId>,
}

/// MOS: from `dev.mos_size()` (FLOW-01: w_finger = W_total/nf, fingers = nf·m); `None` size → both `None`, fingers 1.
/// Other kinds: `w`, `l` params as written (`None` when absent), fingers = `m` (default 1).
/// `model` interns `dev.model` (case-insensitive) into `models`; `bulk` = the FET `B` net, else `None`.
pub fn drawn(dev: &Device, models: &mut Vec<String>) -> Drawn {
    let model = dev.model.to_ascii_lowercase();
    let model = models.iter().position(|m| *m == model).unwrap_or_else(|| {
        models.push(model);
        models.len() - 1
    }) as u16;
    let fet = matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos);
    let bulk = fet.then(|| dev.terminals.iter().find(|(t, _)| t == "B").map(|&(_, n)| n)).flatten();
    let p = |k: &str| dev.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let (w_finger_nm, l_nm, fingers) = if fet {
        dev.mos_size().map_or((None, None, 1), |s| (Some(s.w_finger_nm()), Some(s.l_nm), s.fingers()))
    } else {
        (p("w"), p("l"), param(dev, "m", 1).clamp(1, i64::from(u32::MAX)) as u32)
    };
    Drawn { w_finger_nm, l_nm, fingers, model, bulk }
}

/// A bipolar or diode written without W/L: a fixed-geometry PDK device whose
/// size is its model (`bgr_core`'s `pnp_05v5_W3p40L3p40`).
fn fixed_geometry(kind: DeviceKind, d: &Drawn) -> bool {
    matches!(kind, DeviceKind::Npn | DeviceKind::Pnp | DeviceKind::Diode) && d.w_finger_nm.is_none() && d.l_nm.is_none()
}

/// The size is unknown: never `ExactAs`, and reported as missing `device W/L`.
pub fn unknown_size(kind: DeviceKind, d: &Drawn) -> bool {
    !fixed_geometry(kind, d) && (d.w_finger_nm.is_none() || d.l_nm.is_none())
}

/// `SizeMatch::ExactAs`: same known W/L (or both fixed-geometry), same model,
/// and the same bulk net or both bulks on rails.
pub(crate) fn exact_as(kind: DeviceKind, a: &Drawn, b: &Drawn, roles: &[NetRole]) -> bool {
    let rail = |n: NetId| matches!(roles[n.0 as usize], NetRole::Supply | NetRole::Ground);
    let size = if fixed_geometry(kind, a) {
        b.w_finger_nm.is_none() && b.l_nm.is_none()
    } else {
        !unknown_size(kind, a) && (a.w_finger_nm, a.l_nm) == (b.w_finger_nm, b.l_nm)
    };
    let bulk = a.bulk == b.bulk || a.bulk.zip(b.bulk).is_some_and(|(x, y)| rail(x) && rail(y));
    size && a.model == b.model && bulk
}

/// `SizeMatch::SameLAs`: same known L, same model.
pub(crate) fn same_l_as(a: &Drawn, b: &Drawn) -> bool {
    a.l_nm.is_some() && a.l_nm == b.l_nm && a.model == b.model
}

#[cfg(test)]
mod tests {
    use crate::tests::{fet, nets};
    use crate::{annotate, block, constraints, AnnotationConfig, Block, BlockKind};
    use pnr_core::ids::DeviceId;
    use pnr_core::netlist::{DeviceKind, Netlist};

    /// Two NMOS on a shared source (`tail`), distinct gates and drains, W/L
    /// 10 µm/1 µm (`w = None` drops W). Nets: 0=inp 1=inm 2=outp 3=outm 4=tail
    /// 5=VSS 6=VDD 7=nb1 8=nb2; each device is `(model, bulk net)`.
    fn pair(devs: [(&str, u16); 2], w: Option<i64>) -> Netlist {
        let mut nl = Netlist {
            devices: vec![
                fet("XM1", DeviceKind::Nmos, 0, 2, 4, devs[0].1, 10_000, 1_000),
                fet("XM2", DeviceKind::Nmos, 1, 3, 4, devs[1].1, 10_000, 1_000),
            ],
            nets: nets(&["inp", "inm", "outp", "outm", "tail", "VSS", "VDD", "nb1", "nb2"]),
        };
        for (d, (model, _)) in nl.devices.iter_mut().zip(devs) {
            d.model = model.into();
            if w.is_none() {
                d.params.retain(|(k, _)| k != "w");
            }
        }
        nl
    }

    fn diff_pairs(nl: &Netlist) -> usize {
        let p = annotate(nl, &AnnotationConfig::default());
        block::leaves(&p.blocks).iter().filter(|b| b.kind == BlockKind::DiffPair).count()
    }

    #[test]
    fn flavours_do_not_match() {
        let nl = pair([("nfet_01v8", 5), ("nfet_01v8_lvt", 5)], Some(10_000));
        assert_eq!(diff_pairs(&nl), 0);
        // Grouped by hand (no pattern groups them now): one drawn unit per flavour.
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| super::drawn(d, &mut models)).collect();
        let both = Block { kind: BlockKind::Group, template: "test", devices: vec![DeviceId(0), DeviceId(1)], injected: false, sub_blocks: Vec::new(), selfs: Vec::new() };
        assert_eq!(constraints::assemble(&nl, &drawn, &[both]).unitization.len(), 2, "two unitization classes");
        // Control: one flavour, written in either case, is a pair.
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 5), ("NFET_01V8", 5)], Some(10_000))), 1);
    }

    #[test]
    fn unknown_size_never_matches() {
        let nl = pair([("nfet_01v8", 5); 2], None);
        assert_eq!(diff_pairs(&nl), 0);
        let p = annotate(&nl, &AnnotationConfig::default());
        assert!(p.coverage.iter().all(|&(_, c)| c == crate::Coverage::Unconstrained("unknown size")), "{:?}", p.coverage);
        let missing = |nl: &Netlist| annotate(nl, &AnnotationConfig::default()).missing.contains(&("MatchedSet", "device W/L"));
        assert!(missing(&nl));
        assert!(!missing(&pair([("nfet_01v8", 5); 2], Some(10_000))));
    }

    #[test]
    fn split_bulk_does_not_match() {
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 7), ("nfet_01v8", 8)], Some(10_000))), 0);
        // Control: bulks on two rails (VSS, VDD) still match.
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 5), ("nfet_01v8", 6)], Some(10_000))), 1);
    }
}
