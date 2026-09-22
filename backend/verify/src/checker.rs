//! [`Checker`] — one reusable gdsverify session over one deck. It owns its own
//! string table (re-parsed from `pdk.source`), so every `StrId` the engine
//! reports resolves against `self.loaded.strings`.

use gdsverify::check::lvs::CompareOptions;
use gdsverify::check::report::{Outcome, Severity, Violations};
use gdsverify::check::topology::NetId;
use gdsverify::engine::pipeline::{extract_into, intern_report_ids, ExtractError, Extracted, Loaded};
use gdsverify::engine::run::run_checks;
use gdsverify::engine::{Checks, Outputs, RunOptions, Summary};
use gdsverify::ingest::deck::parse_deck;
use gdsverify::ingest::{StrId, StrTable};
use pnr_core::Shape;

use crate::geom::{build_store, LabeledPin};
use crate::pdk::{nm_grid, GvLayerId, Pdk};
use crate::reference::{self, RefInput};

/// Error prefix of the extraction failure `signoff` degrades around: two
/// labels bound to one extracted net (a short).
pub const LABEL_SHORT: &str = "extract: label short";

/// A reusable verification session over one deck.
pub struct Checker {
    /// The engine's shared input; public so tests can inspect `reference`.
    pub loaded: Loaded,
    extracted: Extracted,
    out: Outputs,
}

impl Checker {
    /// Parse the deck out of `pdk.source`. `strip_density` drops every
    /// `density` rule — the in-loop mode, where a density window over a
    /// half-drawn layout is noise.
    ///
    /// # Errors
    /// The deck failing gdsverify's parser (cannot happen for a loaded `Pdk`).
    pub fn new(pdk: &Pdk, strip_density: bool) -> Result<Self, String> {
        let mut strings = StrTable::default();
        let mut deck = parse_deck(&pdk.source, nm_grid(), &mut strings)
            .map_err(|e| format!("deck rejected: {e}"))?;
        if strip_density {
            if let Some(density) = strings.get("density") {
                deck.rules.spec.retain(|s| s.kind != density);
            }
        }
        // A hand-built `Loaded` skips the loader, so the LVS report ids must be
        // interned here or the first discrepancy panics.
        intern_report_ids(&mut strings);
        let loaded = Loaded { strings, grid: Some(nm_grid()), deck, ..Loaded::default() };
        Ok(Self { loaded, extracted: Extracted::default(), out: Outputs::default() })
    }

    /// Install the schematic reference LVS compares against. Returns how many
    /// schematic devices were skipped for want of a deck recogniser.
    ///
    /// # Errors
    /// A device stating fewer terminals than its recogniser's arity.
    pub fn set_reference(&mut self, input: &RefInput) -> Result<usize, String> {
        let (netlist, skipped) =
            reference::build(input, &self.loaded.deck, &mut self.loaded.strings)?;
        self.loaded.reference = Some(netlist);
        Ok(skipped)
    }

    fn load_geometry(&mut self, shapes: &[Shape], pins: &[LabeledPin]) -> Result<(), String> {
        let (store, provenance) =
            build_store(shapes, pins, &self.loaded.deck, &mut self.loaded.strings)?;
        self.loaded.store = store;
        self.loaded.provenance = provenance;
        Ok(())
    }

    /// Run the selected checks. Findings land in [`Checker::outputs`].
    ///
    /// `unconnected_pin` findings on a **labelled** net are dropped: a port
    /// leaves the cell, so reaching no device inside it is not floating (the
    /// engine's own LVS floating-net check applies the same exemption). This
    /// is what keeps a bulk-only rail — VSS tied through taps, invisible to a
    /// 3-terminal MOS recogniser — from reading as floating metal.
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
        if let Err(e) = extract_into(&self.loaded, &mut self.extracted) {
            return Err(self.extract_error(e));
        }
        let options = RunOptions {
            checks,
            lvs: CompareOptions::default(),
            quasistatic_nets: Vec::new(),
            quasistatic_inductance: false,
            threads: None,
        };
        let mut summary = run_checks(&self.loaded, &self.extracted, &options, &mut self.out)
            .map_err(|e| format!("engine: {e}"))?;
        self.drop_port_floating(&mut summary);
        Ok(summary)
    }

    fn drop_port_floating(&mut self, summary: &mut Summary) {
        let Some(kind) = self.loaded.strings.get("unconnected_pin") else { return };
        let rules: Vec<StrId> =
            self.loaded.deck.rules.spec.iter().filter(|s| s.kind == kind).map(|s| s.id).collect();
        let v = &self.out.violations;
        let exempt = |i: usize| {
            rules.contains(&v.rule[i])
                && self.extracted.ports.name_of(self.extracted.nets.net_of(v.shape_a[i])).is_some()
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

    /// Render an extraction failure; a label conflict names the colliding
    /// labels and carries the [`LABEL_SHORT`] prefix.
    fn extract_error(&self, e: ExtractError) -> String {
        use gdsverify::check::topology::port::PortError;
        if let ExtractError::Port(PortError::ConflictingLabels(net)) = e {
            let names: Vec<&str> = self
                .loaded
                .provenance
                .labels()
                .iter()
                .filter(|&&(poly, _)| self.extracted.nets.net_of(poly) == net)
                .map(|&(_, name)| self.loaded.strings.resolve(name))
                .collect();
            return format!("{LABEL_SHORT}: labels {names:?} bind to one extracted net");
        }
        format!("extract: {e}")
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

    /// Extract only: how many devices the extractor sees in `shapes`. `None`
    /// when extraction fails — for the merge oracle, itself the ambiguity signal.
    #[must_use]
    pub fn device_count(&mut self, shapes: &[Shape]) -> Option<usize> {
        self.load_geometry(shapes, &[]).ok()?;
        extract_into(&self.loaded, &mut self.extracted).ok()?;
        Some(self.extracted.devices.len())
    }

    /// Rules the last run did not execute, as `(rule, why)`.
    #[must_use]
    pub fn skipped_rules(&self) -> Vec<(&str, String)> {
        self.out
            .runs
            .iter()
            .filter(|r| r.outcome != Outcome::Ran)
            .map(|r| (self.rule_name(r.rule), format!("{:?}", r.outcome)))
            .collect()
    }

    /// Total extracted capacitance of the last run, fF; `0.0` without PEX.
    #[must_use]
    pub fn total_cap_ff(&self) -> f32 {
        let Some(p) = self.out.parasitics.as_ref() else { return 0.0 };
        (0..self.extracted.nets.net_count())
            .map(|n| p.net_capacitance(NetId(n as u32)).raw())
            .sum::<f64>() as f32
    }

    #[must_use]
    pub fn rule_name(&self, rule: StrId) -> &str {
        self.loaded.strings.resolve(rule)
    }

    /// A violation's layer name, `"-"` for the no-layer sentinel LVS rows carry.
    #[must_use]
    pub fn layer_name(&self, layer: GvLayerId) -> &str {
        if (layer.0 as usize) < self.loaded.deck.layers.len() {
            self.loaded.strings.resolve(self.loaded.deck.layers.name(layer))
        } else {
            "-"
        }
    }

    /// `drc`/`erc` by the deck rule's kind, `lvs` for the engine's `lvs.*`
    /// ids, else `engine`.
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
