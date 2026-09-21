//! [`Checker`] — one reusable gdsverify session: a hand-built `Loaded` plus the
//! `Extracted`/`Outputs` buffers the engine refills on every run (D4).
//!
//! `Checker::new` parses the PDK's deck source into a **fresh** string table
//! (the `Loaded` owns its own; the `Pdk` keeps a separate copy for queries), so
//! every `StrId` the engine reports resolves against `self.loaded.strings`.

use gdsverify::engine::pipeline::{extract_into, intern_report_ids, Extracted, ExtractError, Loaded};
use gdsverify::engine::run::run_checks;
use gdsverify::engine::{Checks, Outputs, RunOptions, Summary};
use gdsverify::ingest::deck::parse_deck;
use gdsverify::ingest::{StrId, StrTable};
use gdsverify::check::lvs::CompareOptions;
use gdsverify::check::topology::NetId;
use pnr_core::Shape;

use crate::geom::{build_store, LabeledPin};
use crate::pdk::{nm_grid, GvLayerId, Pdk};
use crate::reference::{self, RefInput};

/// Error prefix of the one extraction failure `signoff` degrades around: two
/// labels binding to one extracted net (the engine's short detection, which
/// also fires on the extractor's known limit — MOS diffusion is never split at
/// the gate, so a device's source and drain labels share a component).
pub const LABEL_SHORT: &str = "extract: label short";

/// A reusable verification session over one deck.
pub struct Checker {
    /// The engine's shared input. Public so callers (and tests) can inspect
    /// what a run was checked against — e.g. `loaded.reference`.
    pub loaded: Loaded,
    extracted: Extracted,
    out: Outputs,
    lvs_options: CompareOptions,
}

impl Checker {
    /// Parse the deck out of `pdk.source` into a fresh session.
    ///
    /// `strip_density` removes every `density` rule row from the rule table —
    /// the in-loop mode, where a density window over a half-drawn iteration is
    /// meaningless noise. The strip retains rows of `RuleTable::spec`; each
    /// spec carries its own `layer_start`/`param_start` offsets into the shared
    /// side tables, so removing spec rows leaves the CSR views of every
    /// surviving rule intact (the orphaned side-table runs are simply never
    /// referenced again).
    ///
    /// # Errors
    /// The deck source failing gdsverify's parser — which cannot happen for a
    /// `Pdk` that loaded, but the seam stays honest.
    pub fn new(pdk: &Pdk, strip_density: bool) -> Result<Self, String> {
        let mut strings = StrTable::default();
        let mut deck = parse_deck(&pdk.source, nm_grid(), &mut strings)
            .map_err(|e| format!("deck rejected: {e}"))?;
        if strip_density {
            if let Some(density) = strings.get("density") {
                deck.rules.spec.retain(|s| s.kind != density);
            }
        }
        // Hand-built Loaded never runs the loader, so the LVS report ids must
        // be interned here or the first discrepancy panics in the report.
        intern_report_ids(&mut strings);
        let loaded = Loaded {
            strings,
            grid: Some(nm_grid()),
            deck,
            ..Loaded::default()
        };
        Ok(Self {
            loaded,
            extracted: Extracted::default(),
            out: Outputs::default(),
            lvs_options: CompareOptions::default(),
        })
    }

    /// Install the schematic reference LVS compares against. Returns how many
    /// schematic devices were skipped for want of a deck recogniser (see
    /// [`crate::reference`]).
    ///
    /// # Errors
    /// A device stating fewer terminals than its recogniser's arity.
    pub fn set_reference(&mut self, input: &RefInput) -> Result<usize, String> {
        let (netlist, skipped) =
            reference::build(input, &self.loaded.deck, &mut self.loaded.strings)?;
        self.loaded.reference = Some(netlist);
        Ok(skipped)
    }

    /// Swap fresh geometry into the session: store build → derive → labels.
    fn load_geometry(&mut self, shapes: &[Shape], pins: &[LabeledPin]) -> Result<(), String> {
        let (store, provenance) =
            build_store(shapes, pins, &self.loaded.deck, &mut self.loaded.strings)?;
        self.loaded.store = store;
        self.loaded.provenance = provenance;
        Ok(())
    }

    /// Run the selected checks over `shapes` + `pins`. The findings land in
    /// [`Checker::outputs`]; the returned [`Summary`] says what ran.
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
            lvs: self.lvs_options,
            quasistatic_nets: Vec::new(),
            // Off keeps the run byte-identical to a build without the
            // inductance bridge (see gpurify `RunOptions`).
            quasistatic_inductance: false,
            threads: None,
        };
        run_checks(&self.loaded, &self.extracted, &options, &mut self.out)
            .map_err(|e| format!("engine: {e}"))
    }

    /// Render an extraction failure. A label conflict — the engine's short
    /// detection: two different labels bound to one connected component — is
    /// spelled out with the colliding label names (the net table is already
    /// filled when binding fails, so they are recoverable here) and prefixed
    /// with [`LABEL_SHORT`] so `signoff` can degrade around it instead of
    /// zeroing the whole report.
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
    ///
    /// Paired with [`Checker::outputs`]'s `parasitics`, this is everything a
    /// post-layout netlist needs; see [`crate::netlist`]. Empty (not stale)
    /// before the first [`Checker::run`], because `Extracted::default` is what
    /// the session starts with.
    #[must_use]
    pub fn extracted(&self) -> &Extracted {
        &self.extracted
    }

    /// **Extract only** — how many distinct devices the extractor sees in
    /// `shapes`. `None` means extraction itself failed, which for the merge
    /// oracle *is* the ambiguity signal (a placement move fused two devices'
    /// implants into geometry the recogniser refuses).
    #[must_use]
    pub fn device_count(&mut self, shapes: &[Shape]) -> Option<usize> {
        self.load_geometry(shapes, &[]).ok()?;
        extract_into(&self.loaded, &mut self.extracted).ok()?;
        Some(self.extracted.devices.len())
    }

    /// TEMP DEBUG (doc-hidden): extract `shapes` and describe every recognised
    /// device — model + terminal `(role, net)` pairs — plus each label's net.
    /// For chasing device-pairing failures; not a stable surface.
    #[doc(hidden)]
    pub fn debug_devices(
        &mut self,
        shapes: &[Shape],
        pins: &[LabeledPin],
    ) -> Result<Vec<String>, String> {
        self.load_geometry(shapes, pins)?;
        extract_into(&self.loaded, &mut self.extracted).map_err(|e| format!("extract: {e}"))?;
        let mut out = Vec::new();
        let d = &self.extracted.devices;
        for i in 0..d.len() {
            let dev = gdsverify::check::topology::DeviceId(i as u32);
            let (nets, roles) = d.terminals_of(dev);
            let model = self.loaded.strings.resolve(d.model[i]);
            let terms: Vec<String> = nets
                .iter()
                .zip(roles)
                .map(|(n, r)| format!("{r:?}={}", n.0))
                .collect();
            let bb = self.loaded.store.poly_bbox(d.marker[i]);
            out.push(format!(
                "device {i} {model} [{}] marker@({},{})..({},{})",
                terms.join(", "),
                bb.xlo.raw(),
                bb.ylo.raw(),
                bb.xhi.raw(),
                bb.yhi.raw()
            ));
        }
        for &(poly, name) in self.loaded.provenance.labels() {
            out.push(format!(
                "label {} -> net {}",
                self.loaded.strings.resolve(name),
                self.extracted.nets.net_of(poly).0
            ));
        }
        out.push(format!("nets extracted: {}", self.extracted.nets.net_count()));
        Ok(out)
    }

    /// Total extracted capacitance of the last run, fF. `0.0` when the run did
    /// not include PEX.
    #[must_use]
    pub fn total_cap_ff(&self) -> f32 {
        let Some(p) = self.out.parasitics.as_ref() else { return 0.0 };
        (0..self.extracted.nets.net_count())
            .map(|n| p.net_capacitance(NetId(n as u32)).raw())
            .sum::<f64>() as f32
    }

    /// Resolve a report `StrId` (rule id) back to text.
    #[must_use]
    pub fn rule_name(&self, rule: StrId) -> &str {
        self.loaded.strings.resolve(rule)
    }

    /// The name of a violation's layer, or `"-"` for the no-layer sentinel LVS
    /// findings carry.
    #[must_use]
    pub fn layer_name(&self, layer: GvLayerId) -> &str {
        if (layer.0 as usize) < self.loaded.deck.layers.len() {
            self.loaded.strings.resolve(self.loaded.deck.layers.name(layer))
        } else {
            "-"
        }
    }

    /// Which domain a violation's rule id belongs to, for report prefixes.
    ///
    /// Deck rules classify by their **kind** against the two crates' public
    /// `KINDS` vocabularies (the robust route: rule *ids* are free-form deck
    /// spellings, kinds are the engine's own dispatch keys). Rule ids that are
    /// not deck rows are the engine's own: `lvs.*` by prefix, anything else
    /// (unreachable today) files under `engine`.
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
        if self.loaded.strings.resolve(rule).starts_with("lvs.") {
            return "lvs";
        }
        "engine"
    }
}
