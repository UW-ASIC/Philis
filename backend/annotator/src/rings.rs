//! REL-07: guard rings by role. Rings go on injectors and aggressors, and on
//! victims only when they have a quiet return of their own (Hastings §14.2,
//! §12.2.9; Charbon §8.6.1 / App. D.3: a ring on a shared return can be
//! worse than none). Anything else gets no ring: the cells carry their own
//! taps (CELL-13).

use analog::cell::{GuardRingRequirement, GuardRingType, RingRole};
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::{DeviceKind, Netlist, SubstrateKind};

/// Which minority carrier a device injects into the substrate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Carrier {
    /// An n-type diffusion forward-biasing into p-substrate (collected by an ECGR).
    Electrons,
    /// A p-type diffusion forward-biasing into an n-well (collected by an HCGR).
    Holes,
}

/// Plain per-device data, so the policy is table-testable without ports or a PDK.
/// Every per-device slice is indexed by device id; a shorter slice reads as
/// `false`/`None` past its end.
pub struct RingInputs<'a> {
    /// The devices to ring; a device without a `B` terminal gets none.
    pub netlist: &'a Netlist,
    /// EXT-23 Switching/Capacitive/ImpactIonization; fallback: touches a Clock-class net.
    pub aggressor: &'a [bool],
    /// EXT-23 victims (Moderate+ set members, Reference-gated devices).
    pub victim: &'a [bool],
    /// EXT-23 MinorityElectron/MinorityHole; all `None` without ports.
    pub injector: &'a [Option<Carrier>],
    /// The wafer's substrate; `EpiOnLowRes` turns off rows 5–6 of [`plan`].
    pub substrate: SubstrateKind,
    /// The victims' own ring return.
    pub quiet_ring_net: Option<NetId>,
    /// The supply an electron-collecting ring ties to.
    pub highest_supply: Option<NetId>,
    /// The Ground-class net of lowest `NetId`.
    pub ground: Option<NetId>,
    /// Deck `min_guard_ring_width`, nm.
    pub min_ring_width_nm: i32,
    /// Sidecar `ecgr_min_width_nm` (CELL-17); `None` on every shipped deck.
    pub ecgr_min_width_nm: Option<i32>,
    /// `cells::post_cell::drawable(Ecgr, ..)`.
    pub ecgr_drawable: bool,
    /// `cells::post_cell::drawable(Hcgr, ..)`.
    pub hcgr_drawable: bool,
    /// Sidecar `IsolatedTub` (GAP-14): members and tie net.
    pub tubs: &'a [(Vec<DeviceId>, NetId)],
    /// `cells::post_cell::drawable(Tub, ..)`.
    pub tub_drawable: bool,
}

/// At most one ring per device, the first matching row winning (plan-06
/// REL-07): 0 a member of user tub `k` (GAP-14) → a shareable `Tub { id: k }`
/// victim ring on its tie, when the deck draws a deep n-well (else
/// `("IsolatedTub", ..)` and the rows below); 1 electron injector → supply-tied `Ecgr`; 2 hole injector →
/// ground-tied `Hcgr`; 3/4 an injector whose collector cannot be drawn → a
/// majority tap on its bulk; 5 an aggressor (not a victim) → a shareable tap
/// on its bulk; 6 a victim, once some device got a row 1–5 ring, → a
/// shareable tap on `quiet_ring_net`; 7 anything else → none. Rows 5–6 are
/// skipped on `EpiOnLowRes` (rings give only 7–10 dB there). A device
/// without a `B` terminal gets no ring. Returns the rings and the
/// `("GuardRing", why)` notes for inputs that limited them.
#[must_use]
pub fn plan(i: &RingInputs) -> (Vec<GuardRingRequirement>, Vec<(&'static str, &'static str)>) {
    let (mut rings, mut missing) = (Vec::new(), Vec::new());
    let note = |m: &mut Vec<(&'static str, &'static str)>, why: &'static str| note_once(m, ("GuardRing", why));
    let ring = |d: usize, ring_type, role, connection_net, min_width_nm, shareable| GuardRingRequirement {
        device: DeviceId(d as u16),
        ring_type,
        shareable,
        min_width_nm,
        max_ring_resistance_mohm: 100_000,
        connection_net,
        role,
    };
    let epi = i.substrate == SubstrateKind::EpiOnLowRes;
    let w = i.min_ring_width_nm;
    let mut victims = Vec::new();
    for (d, dev) in i.netlist.devices.iter().enumerate() {
        let Some(&(_, bulk)) = dev.terminals.iter().find(|(t, _)| t == "B") else { continue };
        let pmos = dev.kind == DeviceKind::Pmos;
        if let Some((k, (_, tie))) = i.tubs.iter().enumerate().find(|(_, (m, _))| m.contains(&DeviceId(d as u16))) {
            if i.tub_drawable {
                rings.push(ring(d, GuardRingType::Tub { id: k as u16 }, RingRole::Victim, *tie, w, true));
                continue;
            }
            note_once(&mut missing, ("IsolatedTub", "deck has no deep n-well: tub drawn as an ordinary ring"));
        }
        let r = match i.injector.get(d).copied().flatten() {
            Some(Carrier::Electrons) => match i.highest_supply.filter(|_| i.ecgr_drawable) {
                Some(vdd) => {
                    if i.ecgr_min_width_nm.is_none() {
                        note(&mut missing, "ECGR width rule not given: collection efficiency unknown");
                    }
                    Some(ring(d, GuardRingType::Ecgr, RingRole::Injector, vdd, w.max(i.ecgr_min_width_nm.unwrap_or(0)), false))
                }
                None => {
                    if i.ecgr_drawable {
                        note(&mut missing, "injector ring: no supply/ground net");
                    }
                    note(&mut missing, "ECGR not drawable: electron injector has a majority ring only");
                    Some(ring(d, GuardRingType::Tap { in_well: false }, RingRole::Injector, bulk, w, false))
                }
            },
            Some(Carrier::Holes) => match i.ground.filter(|_| i.hcgr_drawable) {
                Some(gnd) => Some(ring(d, GuardRingType::Hcgr, RingRole::Injector, gnd, w, false)),
                None => {
                    if i.hcgr_drawable {
                        note(&mut missing, "injector ring: no supply/ground net");
                    }
                    Some(ring(d, GuardRingType::Tap { in_well: true }, RingRole::Injector, bulk, w, false))
                }
            },
            None if epi => None,
            None if i.victim.get(d).copied().unwrap_or(false) => {
                victims.push((d, pmos));
                None
            }
            None if i.aggressor.get(d).copied().unwrap_or(false) => {
                Some(ring(d, GuardRingType::Tap { in_well: pmos }, RingRole::Aggressor, bulk, w, true))
            }
            None => None,
        };
        rings.extend(r);
    }
    // Row 6: victims ring only against a device that got a row 1–5 ring.
    if !rings.is_empty() && !victims.is_empty() {
        match i.quiet_ring_net {
            Some(q) => rings.extend(victims.into_iter().map(|(d, pmos)| ring(d, GuardRingType::Tap { in_well: pmos }, RingRole::Victim, q, w, true))),
            None => note(&mut missing, "victim rings: no quiet ring return (SUB-30)"),
        }
    }
    (rings, missing)
}

/// Appends `why` unless it is already listed (the notes are a handful, so a
/// linear scan).
fn note_once(missing: &mut Vec<(&'static str, &'static str)>, why: (&'static str, &'static str)) {
    if !missing.contains(&why) {
        missing.push(why);
    }
}

/// Step-2 coverage: every row of [`plan`], its fallbacks and notes.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::{fet, nets};

    const NO_SUPPLY: (&str, &str) = ("GuardRing", "injector ring: no supply/ground net");
    const NO_ECGR: (&str, &str) = ("GuardRing", "ECGR not drawable: electron injector has a majority ring only");
    const NO_WIDTH: (&str, &str) = ("GuardRing", "ECGR width rule not given: collection efficiency unknown");
    const SUB30: (&str, &str) = ("GuardRing", "victim rings: no quiet ring return (SUB-30)");

    /// N0, N1 NMOS on bulk vss; P2 PMOS on bulk vdd; R3 a resistor (no `B`).
    /// Nets: 0=x 1=vss 2=vdd 3=quiet.
    fn nl() -> Netlist {
        let mut r = fet("R3", DeviceKind::Resistor, 0, 0, 0, 0, 1, 1);
        r.terminals = vec![("P".into(), NetId(0)), ("N".into(), NetId(1))];
        Netlist {
            devices: vec![
                fet("N0", DeviceKind::Nmos, 0, 0, 1, 1, 1_000, 150),
                fet("N1", DeviceKind::Nmos, 0, 0, 1, 1, 1_000, 150),
                fet("P2", DeviceKind::Pmos, 0, 0, 2, 2, 1_000, 150),
                r,
            ],
            nets: nets(&["x", "vss", "vdd", "quiet"]),
            ..Default::default()
        }
    }

    fn base<'a>(nl: &'a Netlist, aggressor: &'a [bool], victim: &'a [bool], injector: &'a [Option<Carrier>]) -> RingInputs<'a> {
        RingInputs {
            netlist: nl,
            aggressor,
            victim,
            injector,
            substrate: SubstrateKind::Bulk,
            quiet_ring_net: Some(NetId(3)),
            highest_supply: Some(NetId(2)),
            ground: Some(NetId(1)),
            min_ring_width_nm: 300,
            ecgr_min_width_nm: Some(500),
            ecgr_drawable: true,
            hcgr_drawable: true,
            tubs: &[],
            tub_drawable: true,
        }
    }

    type Row = (u16, GuardRingType, RingRole, u16, i32, bool);

    fn rows(i: &RingInputs) -> (Vec<Row>, Vec<(&'static str, &'static str)>) {
        let (r, m) = plan(i);
        (r.iter().map(|r| (r.device.0, r.ring_type, r.role, r.connection_net.0, r.min_width_nm, r.shareable)).collect(), m)
    }

    #[test]
    fn nothing_flagged_no_rings() {
        let nl = nl();
        assert_eq!(rows(&base(&nl, &[], &[], &[])), (vec![], vec![]), "empty slices read as false/None");
    }

    #[test]
    fn a_device_without_a_bulk_gets_no_ring() {
        let nl = nl();
        let (r, _) = rows(&base(&nl, &[false, false, false, true], &[], &[None, None, None, Some(Carrier::Electrons)]));
        assert!(r.is_empty(), "{r:?}");
    }

    /// Row 1 with a width rule: the wider of the two, no note; every ring
    /// carries the 100 Ω resistance cap.
    #[test]
    fn an_ecgr_takes_the_wider_width_rule() {
        let nl = nl();
        let (r, m) = rows(&base(&nl, &[], &[], &[Some(Carrier::Electrons)]));
        assert_eq!((r, m), (vec![(0, GuardRingType::Ecgr, RingRole::Injector, 2, 500, false)], vec![]));
        let narrow = RingInputs { ecgr_min_width_nm: Some(100), ..base(&nl, &[], &[], &[Some(Carrier::Electrons)]) };
        assert_eq!(rows(&narrow).0[0].4, 300);
        assert!(plan(&narrow).0.iter().all(|g| g.max_ring_resistance_mohm == 100_000));
    }

    /// Row 3: no supply → a substrate tap on the bulk, with both notes, once
    /// however many injectors.
    #[test]
    fn an_electron_injector_without_a_supply_gets_a_tap() {
        let nl = nl();
        let inj = [Some(Carrier::Electrons), Some(Carrier::Electrons)];
        let (r, m) = rows(&RingInputs { highest_supply: None, ..base(&nl, &[], &[], &inj) });
        assert_eq!(r, [(0, GuardRingType::Tap { in_well: false }, RingRole::Injector, 1, 300, false), (1, GuardRingType::Tap { in_well: false }, RingRole::Injector, 1, 300, false)]);
        assert_eq!(m, [NO_SUPPLY, NO_ECGR]);
    }

    /// Row 3, deck cannot draw an ECGR: only the drawability note.
    #[test]
    fn an_undrawable_ecgr_notes_only_that() {
        let nl = nl();
        let (r, m) = rows(&RingInputs { ecgr_drawable: false, ecgr_min_width_nm: None, ..base(&nl, &[], &[], &[Some(Carrier::Electrons)]) });
        assert_eq!((r[0].1, r[0].3), (GuardRingType::Tap { in_well: false }, 1));
        assert_eq!(m, [NO_ECGR]);
    }

    #[test]
    fn a_hole_injector_gets_a_ground_tied_hcgr() {
        let nl = nl();
        let inj = [None, None, Some(Carrier::Holes)];
        assert_eq!(rows(&base(&nl, &[], &[], &inj)), (vec![(2, GuardRingType::Hcgr, RingRole::Injector, 1, 300, false)], vec![]));
        // Drawable but no ground: a well tap on the bulk, noted.
        let (r, m) = rows(&RingInputs { ground: None, ..base(&nl, &[], &[], &inj) });
        assert_eq!((r, m), (vec![(2, GuardRingType::Tap { in_well: true }, RingRole::Injector, 2, 300, false)], vec![NO_SUPPLY]));
        // Not drawable: the same tap, nothing to note.
        assert!(rows(&RingInputs { hcgr_drawable: false, ..base(&nl, &[], &[], &inj) }).1.is_empty());
    }

    /// An aggressor tap is in the well for a PMOS.
    #[test]
    fn aggressor_taps_follow_polarity() {
        let nl = nl();
        let (r, m) = rows(&base(&nl, &[true, false, true], &[], &[]));
        assert_eq!(r, [(0, GuardRingType::Tap { in_well: false }, RingRole::Aggressor, 1, 300, true), (2, GuardRingType::Tap { in_well: true }, RingRole::Aggressor, 2, 300, true)]);
        assert!(m.is_empty());
    }

    /// A device both victim and aggressor is treated as a victim.
    #[test]
    fn victim_beats_aggressor() {
        let nl = nl();
        let (r, _) = rows(&base(&nl, &[true, true], &[false, true], &[]));
        assert_eq!(r, [(0, GuardRingType::Tap { in_well: false }, RingRole::Aggressor, 1, 300, true), (1, GuardRingType::Tap { in_well: false }, RingRole::Victim, 3, 300, true)]);
        // Alone, it has nothing to be guarded against.
        assert_eq!(rows(&base(&nl, &[false, true], &[false, true], &[])), (vec![], vec![]));
    }

    /// An injector victim is ringed as an injector, not as a victim.
    #[test]
    fn injector_beats_victim() {
        let nl = nl();
        let (r, _) = rows(&base(&nl, &[], &[true], &[Some(Carrier::Electrons)]));
        assert_eq!(r.iter().map(|r| r.2).collect::<Vec<_>>(), [RingRole::Injector]);
    }

    /// Victims need a quiet return; without one they are noted, not ringed.
    #[test]
    fn victims_without_a_quiet_return_are_noted() {
        let nl = nl();
        let (r, m) = rows(&RingInputs { quiet_ring_net: None, ..base(&nl, &[true], &[false, true], &[]) });
        assert_eq!(r.len(), 1);
        assert_eq!(m, [SUB30]);
    }

    /// Row 0: each tub gets its own id; a drawn tub ring is the member's only ring.
    #[test]
    fn tubs_are_numbered_and_exclusive() {
        let nl = nl();
        let tubs = [(vec![DeviceId(0)], NetId(2)), (vec![DeviceId(1)], NetId(3))];
        let (r, m) = rows(&RingInputs { tubs: &tubs, ..base(&nl, &[], &[], &[Some(Carrier::Electrons), Some(Carrier::Electrons)]) });
        assert_eq!(r, [(0, GuardRingType::Tub { id: 0 }, RingRole::Victim, 2, 300, true), (1, GuardRingType::Tub { id: 1 }, RingRole::Victim, 3, 300, true)]);
        assert!(m.is_empty());
    }

    /// A tub ring is not a row 1–5 ring: it does not turn victim rings on.
    #[test]
    fn a_tub_ring_alone_does_not_ring_victims() {
        let nl = nl();
        let tubs = [(vec![DeviceId(0)], NetId(2))];
        let (r, m) = rows(&RingInputs { tubs: &tubs, ..base(&nl, &[], &[false, true], &[]) });
        assert_eq!((r, m), (vec![(0, GuardRingType::Tub { id: 0 }, RingRole::Victim, 2, 300, true)], vec![]));
    }

    /// An undrawable tub falls through to the ordinary rows, noted once.
    #[test]
    fn undrawable_tubs_fall_through() {
        let nl = nl();
        let tubs = [(vec![DeviceId(0), DeviceId(1)], NetId(2))];
        let (r, m) = rows(&RingInputs { tubs: &tubs, tub_drawable: false, ..base(&nl, &[true, true], &[], &[]) });
        assert_eq!(r.iter().map(|r| r.2).collect::<Vec<_>>(), [RingRole::Aggressor, RingRole::Aggressor]);
        assert_eq!(m, [("IsolatedTub", "deck has no deep n-well: tub drawn as an ordinary ring")]);
    }

    /// On epi, injectors still ring; aggressors and victims do not.
    #[test]
    fn epi_keeps_injector_rings() {
        let nl = nl();
        let i = RingInputs { substrate: SubstrateKind::EpiOnLowRes, ..base(&nl, &[false, true], &[false, false, true], &[Some(Carrier::Electrons)]) };
        assert_eq!(rows(&i).0.iter().map(|r| (r.0, r.2)).collect::<Vec<_>>(), [(0, RingRole::Injector)]);
    }
}
