//! [`Checker`] — one reusable gdsverify session over one deck. It owns its own
//! string table (re-parsed from `pdk.source`), so every `StrId` the engine
//! reports resolves against `self.loaded.strings`.

use gdsverify::check::lvs::CompareOptions;
use gdsverify::check::report::{Outcome, Severity, Violations};
use gdsverify::engine::pipeline::{extract, intern_report_ids, ExtractError, Extracted, Loaded};
use gdsverify::engine::run::run_checks;
use gdsverify::engine::{Checks, Outputs, RunOptions, Summary};
use gdsverify::ingest::deck::parse_deck;
use gdsverify::ingest::{StrId, StrTable};
use pnr_core::Shape;

use crate::geom::{build_store, LabeledPin};
use crate::pdk::{nm_grid, GvLayerId, Pdk};
use crate::reference::{self, RefInput, RefKind};

/// Error prefix of the extraction failure `signoff` degrades around: two
/// labels bound to one extracted net (a short).
pub const LABEL_SHORT: &str = "extract: label short";

/// A reusable verification session over one deck: configure it with the
/// `set_*` calls, then [`Checker::run`] any number of geometries against it.
/// Each run replaces the previous run's geometry, extraction and outputs.
pub struct Checker {
    /// The engine's shared input; public so tests can inspect `reference`.
    pub loaded: Loaded,
    /// The last run's extraction (empty before the first run).
    extracted: Extracted,
    /// The last run's findings, rule outcomes and parasitics.
    out: Outputs,
    /// Rules taken out of the deck as chip-level, `(rule, why)`.
    deferred: Vec<(String, String)>,
    /// [`RefInput::external_ports`], interned in `loaded.strings` (the labels'
    /// table, so ids compare equal): the only nets `drop_port_floating` exempts.
    external: Option<Vec<StrId>>,
    /// Layers `build_store` loads as their union (routing metals, li, nwell);
    /// emptied by [`crate::ExtractOptions::unmerged`].
    merge: Vec<u16>,
    /// Layers this deck merges, for [`Checker::set_extract`] to restore.
    merge_default: Vec<u16>,
    /// [`crate::ExtractOptions::field_solve`]: PEX field-solves these nets.
    field_nets: Vec<String>,
}

impl Checker {
    /// Parses the deck out of `pdk.source`. `strip_density` drops every
    /// `density` and `density_cmp` rule — the in-loop mode, where a density
    /// window over a half-drawn layout is noise. Waived rules
    /// (`cell.waivers`: rule id → reason) and `global_density` rules are
    /// taken out and listed by [`Checker::skipped_rules`]; a waiver naming no
    /// deck rule is ignored.
    ///
    /// # Errors
    /// The deck failing gdsverify's parser (cannot happen for a loaded `Pdk`).
    pub fn new(pdk: &Pdk, strip_density: bool) -> Result<Self, String> {
        let mut strings = StrTable::default();
        let mut deck = parse_deck(&pdk.source, nm_grid(), &mut strings)
            .map_err(|e| format!("deck rejected: {e}"))?;
        if strip_density {
            let kinds = [strings.get("density"), strings.get("density_cmp")];
            deck.rules.spec.retain(|s| !kinds.contains(&Some(s.kind)));
        }
        // Whole-die fill coverage says nothing about a block: deferred to
        // chip signoff, reported as such.
        let mut deferred = Vec::new();
        // The sidecar's waivers (`cell.waivers`: rule id → reason): not run,
        // and reported with the reason.
        if let Some(w) = pdk.cell.get("waivers").and_then(|w| w.as_object()) {
            let ids: Vec<StrId> = w.keys().filter_map(|k| strings.get(k)).collect();
            for (k, why) in w {
                if strings.get(k).is_some_and(|id| deck.rules.spec.iter().any(|s| s.id == id)) {
                    deferred.push((k.clone(), format!("Waived({})", why.as_str().unwrap_or(""))));
                }
            }
            deck.rules.spec.retain(|s| !ids.contains(&s.id));
        }
        if let Some(global) = strings.get("global_density") {
            for s in deck.rules.spec.iter().filter(|s| s.kind == global) {
                deferred.push((strings.resolve(s.id).to_string(), "ChipLevel(global density: whole die)".to_string()));
            }
            deck.rules.spec.retain(|s| s.kind != global);
        }
        // A hand-built `Loaded` skips the loader, so the LVS report ids must be
        // interned here or the first discrepancy panics.
        intern_report_ids(&mut strings);
        let loaded = Loaded {
            strings,
            grid: nm_grid(),
            deck,
            store: Default::default(),
            provenance: Default::default(),
            reference: None,
            intent: None,
        };
        let mut merge: Vec<u16> = pdk
            .routing_metals
            .iter()
            .copied()
            .chain(["li", "nwell"].iter().filter_map(|r| pnr_core::Process::layer(pdk, r)))
            .map(|l| l.0)
            .collect();
        merge.sort_unstable();
        merge.dedup();
        Ok(Self {
            loaded,
            extracted: Extracted::default(),
            out: Outputs::default(),
            deferred,
            external: None,
            merge_default: merge.clone(),
            merge,
            field_nets: Vec::new(),
        })
    }

    /// Installs the schematic reference LVS compares against, replacing any
    /// earlier one, and its external port list. Returns the `(kind, model
    /// hint)` of each schematic device skipped for want of a deck recogniser,
    /// in input order: LVS compares nothing for those.
    ///
    /// # Errors
    /// A device stating fewer terminals than its recogniser's arity; the
    /// checker then holds no reference.
    pub fn set_reference(&mut self, input: &RefInput) -> Result<Vec<(RefKind, Option<String>)>, String> {
        let (netlist, skipped) =
            reference::build(input, &self.loaded.deck, &mut self.loaded.strings)?;
        self.loaded.reference = Some(netlist);
        let strings = &mut self.loaded.strings;
        self.external = input.external_ports.as_ref().map(|p| p.iter().map(|n| strings.intern(n)).collect());
        Ok(skipped.into_iter().map(|i| (input.devices[i].kind, input.devices[i].model.clone())).collect())
    }

    /// Installs design intent (supplies, their voltage, the current each is
    /// budgeted to carry, the drop each may take): what the deck's EM and IR
    /// rules need to run rather than skip. Replaces the previous intent; an
    /// [`crate::Intent`] with no supplies installs none (currents and drops
    /// without a supply are dropped). Each power supply is a domain named
    /// for its voltage rounded to the mV; every ground joins the highest
    /// power domain (0 mV when there is none). Currents and drops that are
    /// not positive and finite are omitted.
    ///
    /// # Errors
    /// gdsverify refusing the intent (a net in two domains, a bad limit); the
    /// checker then holds no intent.
    pub fn set_intent(&mut self, intent: &crate::Intent) -> Result<(), String> {
        self.loaded.intent = None;
        if intent.supplies.is_empty() {
            return Ok(());
        }
        let domain = |mv: f64| format!("d{}", mv.round() as i64);
        let power_mv = intent.supplies.iter().filter(|s| !s.2).map(|s| s.1).fold(0.0, f64::max);
        let mut domains = serde_json::Map::new();
        let mut supplies = Vec::with_capacity(intent.supplies.len());
        for (net, mv, ground) in &intent.supplies {
            // A ground joins the (highest) power domain: gdsverify's domains are
            // voltage levels with a supply pair.
            let mv = if *ground { power_mv } else { *mv };
            domains.insert(domain(mv), serde_json::json!({ "voltage_mv": mv }));
            let role = if *ground { "ground" } else { "power" };
            supplies.push(serde_json::json!({ "net": net, "domain": domain(mv), "role": role }));
        }
        // One object per net: gdsverify refuses a net listed twice.
        let mut by_net: std::collections::BTreeMap<&str, serde_json::Map<String, serde_json::Value>> = std::collections::BTreeMap::new();
        for (net, ua) in intent.currents.iter().filter(|(_, ua)| *ua > 0.0) {
            by_net.entry(net).or_default().insert("budget_current_ua".into(), serde_json::json!(ua));
        }
        for (net, mv) in intent.max_drop_mv.iter().filter(|(_, mv)| *mv > 0.0 && mv.is_finite()) {
            by_net.entry(net).or_default().insert("max_drop_mv".into(), serde_json::json!(mv));
        }
        let limits: Vec<_> = by_net
            .into_iter()
            .map(|(net, mut m)| {
                m.insert("net".into(), serde_json::json!(net));
                serde_json::Value::Object(m)
            })
            .collect();
        let json = serde_json::json!({ "domains": domains, "supplies": supplies, "limits": limits }).to_string();
        let parsed = gdsverify::ingest::intent::parse_intent(&json, &mut self.loaded.strings).map_err(|e| e.to_string())?;
        self.loaded.intent = Some(parsed);
        Ok(())
    }

    /// Sets the extraction options of the following [`Checker::run`]s:
    /// `unmerged` loads every shape as its own polygon, `field_solve` names
    /// the nets PEX field-solves. Replaces the previous options.
    pub fn set_extract(&mut self, o: &crate::ExtractOptions) {
        self.field_nets.clone_from(&o.field_solve);
        self.merge = if o.unmerged { Vec::new() } else { self.merge_default.clone() };
    }

    /// Builds the store and label provenance of `shapes`/`pins` into
    /// `self.loaded`, replacing the previous geometry.
    fn load_geometry(&mut self, shapes: &[Shape], pins: &[LabeledPin]) -> Result<(), String> {
        let (store, provenance) =
            build_store(shapes, pins, &self.loaded.deck, &mut self.loaded.strings, &self.merge)?;
        self.loaded.store = store;
        self.loaded.provenance = provenance;
        Ok(())
    }

    /// Runs the selected checks over `shapes` labelled by `pins`. Findings
    /// land in [`Checker::outputs`], the extraction in [`Checker::extracted`].
    ///
    /// `unconnected_pin` and `floating_gate` findings on a **port** net are
    /// dropped: a port leaves the cell, so reaching no device inside it, or
    /// only gates (an input driven from outside), is not floating (the
    /// engine's own LVS floating-net check applies the same exemption). This
    /// is what keeps a bulk-only rail — VSS tied through taps, invisible to a
    /// 3-terminal MOS recogniser — from reading as floating metal. A port is
    /// a labelled net named in [`RefInput::external_ports`]; with no port
    /// list every labelled net is one, and [`Checker::skipped_rules`] says so.
    ///
    /// # Errors
    /// Geometry that cannot be loaded (a mislanded pin label, a derived-layer
    /// failure), an extraction fault, or an engine refusal of the deck.
    pub fn run(
        &mut self,
        shapes: &[Shape],
        pins: &[LabeledPin],
        checks: Checks,
    ) -> Result<Summary, String> {
        self.load_geometry(shapes, pins)?;
        self.extracted = extract(&self.loaded).map_err(|e| self.extract_error(e))?;
        let options = RunOptions {
            checks,
            lvs: CompareOptions::default(),
            quasistatic_nets: self.field_nets.clone(),
            quasistatic_inductance: false,
        };
        let (out, mut summary) =
            run_checks(&self.loaded, &self.extracted, &options).map_err(|e| format!("engine: {e}"))?;
        self.out = out;
        self.drop_port_floating(&mut summary);
        Ok(summary)
    }

    /// Removes the `unconnected_pin`/`floating_gate` rows on port nets (see
    /// [`Checker::run`]) from the outputs and their counts from `summary`.
    fn drop_port_floating(&mut self, summary: &mut Summary) {
        let kinds: Vec<StrId> = ["unconnected_pin", "floating_gate"].iter().filter_map(|k| self.loaded.strings.get(k)).collect();
        let rules: Vec<StrId> =
            self.loaded.deck.rules.spec.iter().filter(|s| kinds.contains(&s.kind)).map(|s| s.id).collect();
        let v = &self.out.violations;
        let exempt = |i: usize| {
            rules.contains(&v.rule[i])
                && self.extracted.ports.name_of(self.extracted.nets.net_of(v.shape_a[i])).is_some_and(|name| {
                    self.external.as_ref().is_none_or(|e| e.contains(&name))
                })
        };
        if !(0..v.len()).any(exempt) {
            return;
        }
        let mut kept = Violations::default();
        for i in (0..v.len()).filter(|&i| !exempt(i)) {
            kept.push(v.get(i));
        }
        for i in (0..v.len()).filter(|&i| exempt(i)) {
            summary.violations -= 1;
            match v.severity[i] {
                Severity::Error => summary.errors -= 1,
                _ => summary.warnings -= 1,
            }
        }
        self.out.violations = kept;
    }

    /// Renders an extraction failure; a label conflict names the colliding
    /// labels and carries the [`LABEL_SHORT`] prefix.
    fn extract_error(&self, e: ExtractError) -> String {
        use gdsverify::check::topology::port::PortError;
        if let ExtractError::Port(PortError::ConflictingLabels(net)) = e {
            // Extraction stopped at the ports: the nets are rebuilt to name
            // the labels that share one.
            let names = self.label_nets().into_iter().find(|g| g.0 == net).map(|g| g.1).unwrap_or_default();
            return format!("{LABEL_SHORT}: labels {names:?} bind to one extracted net");
        }
        format!("extract: {e}")
    }

    /// Returns the label names of each extracted net that carries more than one —
    /// what a [`LABEL_SHORT`] is — over the last loaded geometry, first-seen
    /// order, each name once.
    #[must_use]
    pub fn shorted_labels(&self) -> Vec<Vec<String>> {
        self.label_nets().into_iter().map(|g| g.1).filter(|g| g.len() > 1).collect()
    }

    /// Returns every labelled net of the last loaded geometry with its label names,
    /// first-seen order, each name once. Rebuilds the net table: extraction
    /// may have stopped before producing one.
    fn label_nets(&self) -> Vec<(gdsverify::check::topology::NetId, Vec<String>)> {
        let mut nets = gdsverify::check::topology::NetTable::default();
        gdsverify::check::topology::net::extract_nets_into(&self.loaded.store, &self.loaded.deck.connectivity, &mut nets);
        let mut groups: Vec<(_, Vec<String>)> = Vec::new();
        for &(poly, name) in self.loaded.provenance.labels() {
            let (net, name) = (nets.net_of(poly), self.loaded.strings.resolve(name).to_string());
            match groups.iter_mut().find(|g| g.0 == net) {
                Some(g) if !g.1.contains(&name) => g.1.push(name),
                Some(_) => {}
                None => groups.push((net, vec![name])),
            }
        }
        groups
    }

    /// What the last [`Checker::run`] produced.
    #[must_use]
    pub fn outputs(&self) -> &Outputs {
        &self.out
    }

    /// The last run's extraction — nets, recognised devices, ports.
    #[must_use]
    pub fn extracted(&self) -> &Extracted {
        &self.extracted
    }

    /// Extracts only: returns how many devices the extractor sees in `shapes`
    /// (unlabelled). `None` when loading or extraction fails — for the merge
    /// oracle, itself the ambiguity signal. Replaces the loaded geometry and
    /// extraction but not [`Checker::outputs`].
    #[must_use]
    pub fn device_count(&mut self, shapes: &[Shape]) -> Option<usize> {
        self.load_geometry(shapes, &[]).ok()?;
        self.extracted = extract(&self.loaded).ok()?;
        Some(self.extracted.devices.len())
    }

    /// Returns the rules the last run did not execute, as `(rule, why)`: the
    /// engine's non-`Ran` outcomes, then the waived and chip-level rules
    /// taken out of the deck ([`Checker::new`],
    /// [`Checker::defer_density_wider_than`]), then `floating_gate` when no
    /// port list narrowed its exemption (it ran, but checked no labelled net).
    #[must_use]
    pub fn skipped_rules(&self) -> Vec<(&str, String)> {
        let no_ports = self.external.is_none().then(|| ("floating_gate", "exempt on every labelled net: no port list".to_string()));
        self.out
            .runs
            .iter()
            .filter(|r| r.outcome != Outcome::Ran)
            .map(|r| (self.rule_name(r.rule), format!("{:?}", r.outcome)))
            .chain(self.deferred.iter().map(|(n, why)| (n.as_str(), why.clone())))
            .chain(no_ports)
            .collect()
    }

    /// Takes out every `density` (one square `window`) and `density_cmp`
    /// (`window_x` × `window_y`, sky130's `m*.density`) rule whose window side
    /// exceeds the `w × h` nm block's: a window wider than the block measures
    /// the chip around it — and with `partial_windows: false` GPurify examines
    /// zero windows and records `Ran` — so that check is chip integration's:
    /// not run here, and reported as not run (never as passed). Returns the
    /// rules taken out.
    pub fn defer_density_wider_than(&mut self, w: i64, h: i64) -> Vec<String> {
        use gdsverify::ingest::deck::{ParamValue, RuleSpec};
        let (st, rules) = (&self.loaded.strings, &self.loaded.deck.rules);
        let (density, cmp) = (st.get("density"), st.get("density_cmp"));
        let len = |s: &RuleSpec, k: &str| match st.get(k).and_then(|k| rules.param(s, k)) {
            Some(ParamValue::Length(d)) => Some(d.raw()),
            _ => None,
        };
        let window = |s: &RuleSpec| match Some(s.kind) {
            k if k == density => len(s, "window").map(|d| (d, d)),
            k if k == cmp => len(s, "window_x").zip(len(s, "window_y")),
            _ => None,
        };
        let out: Vec<(StrId, String, String)> = rules
            .spec
            .iter()
            .filter_map(|s| {
                let (wx, wy) = window(s).filter(|&(wx, wy)| wx > w || wy > h)?;
                let win = if wx == wy { format!("{wx}") } else { format!("{wx}x{wy}") };
                Some((s.id, st.resolve(s.id).to_string(), format!("ChipLevel(window {win} nm > block {w}x{h} nm)")))
            })
            .collect();
        self.loaded.deck.rules.spec.retain(|s| !out.iter().any(|o| o.0 == s.id));
        let names = out.iter().map(|o| o.1.clone()).collect();
        self.deferred.extend(out.into_iter().map(|(_, n, why)| (n, why)));
        names
    }

    /// Returns the last run's extracted capacitance matrix over **labelled** nets, fF:
    /// `(net, None, C)` to ground, `(a, Some(b), C)` coupling, `a < b`, rows
    /// summed per pair, sorted by `(a, b)`. Elements touching an unlabelled
    /// net, and coupling between two nodes of one net, are dropped (no
    /// schematic node pair to hang them on); empty without PEX.
    #[must_use]
    pub fn cap_matrix(&self) -> Vec<(String, Option<String>, f64)> {
        use gdsverify::extract::network::Parasitic;
        let Some(p) = self.out.parasitics.as_ref() else { return Vec::new() };
        let name = |node: u32| {
            let net = *p.node_net.get(node as usize)?;
            self.extracted.ports.name_of(net).map(|s| self.loaded.strings.resolve(s).to_string())
        };
        let mut rows: std::collections::BTreeMap<(String, Option<String>), f64> = std::collections::BTreeMap::new();
        for i in 0..p.from.len() {
            let key = match (p.value[i], p.to[i]) {
                (Parasitic::GroundCap(q), _) => name(p.from[i].0).map(|a| ((a, None), q.raw())),
                (Parasitic::CouplingCap(q), Some(to)) => match (name(p.from[i].0), name(to.0)) {
                    (Some(x), Some(y)) if x != y => {
                        let (x, y) = if x < y { (x, y) } else { (y, x) };
                        Some(((x, Some(y)), q.raw()))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((k, c)) = key {
                *rows.entry(k).or_default() += c;
            }
        }
        rows.into_iter().map(|((a, b), c)| (a, b, c)).collect()
    }

    /// Returns the total extracted capacitance of the last run, fF, every net
    /// (labelled or not); `0.0` without PEX.
    #[must_use]
    pub fn total_cap_ff(&self) -> f32 {
        let Some(p) = self.out.parasitics.as_ref() else { return 0.0 };
        p.capacitance_per_net().iter().sum::<f64>() as f32
    }

    /// Returns the deck or engine name of an interned rule id.
    ///
    /// # Panics
    /// When `rule` was not interned in this checker's string table.
    #[must_use]
    pub fn rule_name(&self, rule: StrId) -> &str {
        self.loaded.strings.resolve(rule)
    }

    /// Returns a violation's layer name, `"-"` for any id past the deck's
    /// layer table (the no-layer sentinel LVS rows carry).
    #[must_use]
    pub fn layer_name(&self, layer: GvLayerId) -> &str {
        if (layer.0 as usize) < self.loaded.deck.layers.len() {
            self.loaded.strings.resolve(self.loaded.deck.layers.name(layer))
        } else {
            "-"
        }
    }

    /// Returns the domain a rule reports under: `drc`/`erc` by the deck
    /// rule's kind, `lvs` for the engine's `lvs.*` ids, else `engine`.
    #[must_use]
    pub fn domain_of(&self, rule: StrId) -> &'static str {
        if let Some(spec) = self.loaded.deck.rules.spec.iter().find(|s| s.id == rule) {
            let kind = self.loaded.strings.resolve(spec.kind);
            if gdsverify::check::drc::ruleset::KINDS.contains(&kind) {
                return "drc";
            }
            if gdsverify::check::erc::ruleset::KINDS.contains(&kind) {
                return "erc";
            }
        }
        if self.rule_name(rule).starts_with("lvs.") {
            return "lvs";
        }
        "engine"
    }
}
