//! Additive production-deck schema and fail-closed checked execution.
//!
//! The original [`crate::params::Deck`] remains the explicitly named W1 legacy
//! adapter. This module does not reinterpret legacy JSON. New decks use a versioned,
//! strict schema with typed quantities, expressions, lookup tables, symbolic base or
//! derived layer references, and context selectors. A parsed rule is never silently
//! skipped: missing evidence is `Error`, an inapplicable declared context is `NotRun`,
//! and an implementation gap is `Unsupported`.

use super::derived::{DerivedError, DerivedEvaluator, DerivedExpr, DerivedValue, LayerExprRef};
use super::{run_drc, Violation};
use crate::geometry::exact::{
    rectilinear_intersection, rectilinear_subtraction, Point, Polygon, PolygonSet,
};
use crate::geometry::{GeometryStore, LayerId};
use crate::params::{Deck, LayerTable};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Dbu,
    Nanometer,
    SquareDbu,
    SquareNanometer,
    Ratio,
    Count,
    Volt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Length,
    Area,
    Ratio,
    Count,
    Voltage,
}

impl Unit {
    pub fn dimension(self) -> Dimension {
        match self {
            Self::Dbu | Self::Nanometer => Dimension::Length,
            Self::SquareDbu | Self::SquareNanometer => Dimension::Area,
            Self::Ratio => Dimension::Ratio,
            Self::Count => Dimension::Count,
            Self::Volt => Dimension::Voltage,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Literal {
    pub value: f64,
    pub unit: Unit,
}

/// Quantity normalized to DBU, DBU², ratio, count, or volts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub dimension: Dimension,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScalarExpr {
    Literal {
        value: f64,
        unit: Unit,
    },
    Variable {
        name: String,
    },
    Add {
        lhs: Box<ScalarExpr>,
        rhs: Box<ScalarExpr>,
    },
    Subtract {
        lhs: Box<ScalarExpr>,
        rhs: Box<ScalarExpr>,
    },
    Multiply {
        lhs: Box<ScalarExpr>,
        rhs: Box<ScalarExpr>,
    },
    Divide {
        lhs: Box<ScalarExpr>,
        rhs: Box<ScalarExpr>,
    },
    Lookup {
        table: String,
        input: Box<ScalarExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TablePoint {
    pub input: f64,
    pub output: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupTable {
    pub input_unit: Unit,
    pub output_unit: Unit,
    pub points: Vec<TablePoint>,
    /// If false, use a lower-bound step table. If true, linearly interpolate.
    #[serde(default)]
    pub interpolate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum LayerSourceSchema {
    Base { name: String },
    Derived { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedLayerSource {
    Base(LayerId),
    Derived(String),
}

/// Symbolic derived-layer AST used at the JSON boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum DerivedLayerSchema {
    Layer {
        source: LayerSourceSchema,
    },
    Union {
        operands: Vec<DerivedLayerSchema>,
    },
    Intersection {
        lhs: Box<DerivedLayerSchema>,
        rhs: Box<DerivedLayerSchema>,
    },
    Subtraction {
        lhs: Box<DerivedLayerSchema>,
        rhs: Box<DerivedLayerSchema>,
    },
    Xor {
        lhs: Box<DerivedLayerSchema>,
        rhs: Box<DerivedLayerSchema>,
    },
    NotWithin {
        region: Box<DerivedLayerSchema>,
        operand: Box<DerivedLayerSchema>,
    },
    Grow {
        operand: Box<DerivedLayerSchema>,
        distance: i32,
    },
    Shrink {
        operand: Box<DerivedLayerSchema>,
        distance: i32,
    },
    Inside {
        operand: Box<DerivedLayerSchema>,
        region: Box<DerivedLayerSchema>,
    },
    Outside {
        operand: Box<DerivedLayerSchema>,
        region: Box<DerivedLayerSchema>,
    },
    Interacting {
        operand: Box<DerivedLayerSchema>,
        other: Box<DerivedLayerSchema>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetRelation {
    SameNet,
    DifferentNet,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSelector {
    pub net_relation: Option<NetRelation>,
    pub min_voltage_difference: Option<ScalarExpr>,
    pub domains: Option<Vec<String>>,
    pub region: Option<String>,
    pub cells: Option<Vec<String>>,
    pub hierarchy_prefix: Option<String>,
    pub process: Option<String>,
    pub corner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductionRuleSchema {
    MinWidth {
        id: String,
        layer: LayerSourceSchema,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    Spacing {
        id: String,
        layer: LayerSourceSchema,
        other: Option<LayerSourceSchema>,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    PrlSpacing {
        id: String,
        layer: LayerSourceSchema,
        parallel_run_length: ScalarExpr,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    EolSpacing {
        id: String,
        layer: LayerSourceSchema,
        eol_width: ScalarExpr,
        within: ScalarExpr,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    Enclosure {
        id: String,
        outer: LayerSourceSchema,
        inner: LayerSourceSchema,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    Extension {
        id: String,
        layer: LayerSourceSchema,
        reference: LayerSourceSchema,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    MinArea {
        id: String,
        layer: LayerSourceSchema,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    MinEnclosedArea {
        id: String,
        layer: LayerSourceSchema,
        limit: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    /// Large material components require an exact hole or explicit slot/keyhole
    /// evidence layer that touches or interacts with the component.
    Cheesing {
        id: String,
        layer: LayerSourceSchema,
        slots: LayerSourceSchema,
        max_area_without_slot: ScalarExpr,
        #[serde(default)]
        when: ContextSelector,
    },
    CutClass {
        id: String,
        cut: LayerSourceSchema,
        class: String,
        min_count: ScalarExpr,
        within: ScalarExpr,
        #[serde(default)]
        redundancy: Option<ScalarExpr>,
        #[serde(default)]
        when: ContextSelector,
    },
    Density {
        id: String,
        layer: LayerSourceSchema,
        region: LayerSourceSchema,
        exclusion: Option<LayerSourceSchema>,
        window: ScalarExpr,
        step: ScalarExpr,
        min: Option<ScalarExpr>,
        max: Option<ScalarExpr>,
        #[serde(default)]
        cmp_model: Option<String>,
        #[serde(default)]
        when: ContextSelector,
    },
    Antenna {
        id: String,
        conductor: LayerSourceSchema,
        gate: LayerSourceSchema,
        diode: Option<LayerSourceSchema>,
        fabrication_stage: u32,
        ratio_limit: ScalarExpr,
        #[serde(default)]
        equation: Option<String>,
        #[serde(default)]
        when: ContextSelector,
    },
    MultiPatterning {
        id: String,
        layer: LayerSourceSchema,
        colors: ScalarExpr,
        spacing: ScalarExpr,
        #[serde(default)]
        max_search_states: Option<u64>,
        #[serde(default)]
        when: ContextSelector,
    },
}

impl ProductionRuleSchema {
    pub fn id(&self) -> &str {
        match self {
            Self::MinWidth { id, .. }
            | Self::Spacing { id, .. }
            | Self::PrlSpacing { id, .. }
            | Self::EolSpacing { id, .. }
            | Self::Enclosure { id, .. }
            | Self::Extension { id, .. }
            | Self::MinArea { id, .. }
            | Self::MinEnclosedArea { id, .. }
            | Self::Cheesing { id, .. }
            | Self::CutClass { id, .. }
            | Self::Density { id, .. }
            | Self::Antenna { id, .. }
            | Self::MultiPatterning { id, .. } => id,
        }
    }

    fn selector(&self) -> &ContextSelector {
        match self {
            Self::MinWidth { when, .. }
            | Self::Spacing { when, .. }
            | Self::PrlSpacing { when, .. }
            | Self::EolSpacing { when, .. }
            | Self::Enclosure { when, .. }
            | Self::Extension { when, .. }
            | Self::MinArea { when, .. }
            | Self::MinEnclosedArea { when, .. }
            | Self::Cheesing { when, .. }
            | Self::CutClass { when, .. }
            | Self::Density { when, .. }
            | Self::Antenna { when, .. }
            | Self::MultiPatterning { when, .. } => when,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionDeckSchema {
    pub schema_version: u32,
    pub deck_id: String,
    pub model_revision: String,
    pub dbu_nm: f64,
    #[serde(default)]
    pub variables: BTreeMap<String, ScalarExpr>,
    #[serde(default)]
    pub tables: BTreeMap<String, LookupTable>,
    #[serde(default)]
    pub derived_layers: BTreeMap<String, DerivedLayerSchema>,
    pub rules: Vec<ProductionRuleSchema>,
}

#[derive(Clone, Debug)]
pub struct ProductionDeck {
    pub schema: ProductionDeckSchema,
    pub derived_layers: BTreeMap<String, DerivedExpr>,
}

impl ProductionDeck {
    /// Parse strict JSON and resolve all symbolic layers before any layout runs.
    pub fn from_json(value: &serde_json::Value, layers: &LayerTable) -> Result<Self, DeckError> {
        let schema: ProductionDeckSchema = serde_json::from_value(value.clone())
            .map_err(|error| DeckError::Schema(error.to_string()))?;
        Self::from_schema(schema, layers)
    }

    pub fn from_schema(
        schema: ProductionDeckSchema,
        layers: &LayerTable,
    ) -> Result<Self, DeckError> {
        if schema.schema_version != 1 {
            return Err(DeckError::Schema(format!(
                "unsupported production DRC schema version {}",
                schema.schema_version
            )));
        }
        if schema.deck_id.trim().is_empty() || schema.model_revision.trim().is_empty() {
            return Err(DeckError::Schema(
                "deck_id and model_revision must not be empty".into(),
            ));
        }
        if !schema.dbu_nm.is_finite() || schema.dbu_nm <= 0.0 {
            return Err(DeckError::Schema(
                "dbu_nm must be finite and positive".into(),
            ));
        }
        validate_tables(&schema.tables, schema.dbu_nm)?;
        validate_variables(&schema.variables, &schema.tables, schema.dbu_nm)?;

        let derived_layers = schema
            .derived_layers
            .iter()
            .map(|(name, expression)| {
                if name.trim().is_empty() {
                    return Err(DeckError::Schema(
                        "derived-layer name must not be empty".into(),
                    ));
                }
                Ok((
                    name.clone(),
                    resolve_derived(expression, layers, &schema.derived_layers)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        let mut ids = BTreeSet::new();
        for rule in &schema.rules {
            if rule.id().trim().is_empty() || !ids.insert(rule.id().to_owned()) {
                return Err(DeckError::Schema(format!(
                    "rule ID `{}` is empty or duplicated",
                    rule.id()
                )));
            }
            validate_rule(rule, &schema, layers)?;
        }

        // Resolve every named derived layer now. Cycles and arbitrary-angle operations
        // that depend on actual geometry remain checked at run time, but references are
        // guaranteed to exist before execution.
        for expression in derived_layers.values() {
            validate_derived_references(expression, &derived_layers)?;
        }
        Ok(Self {
            schema,
            derived_layers,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct DrcContext {
    pub poly_nets: Option<BTreeMap<u32, String>>,
    pub net_voltages: Option<BTreeMap<String, f64>>,
    pub net_domains: Option<BTreeMap<String, String>>,
    pub poly_cells: Option<BTreeMap<u32, String>>,
    pub hierarchy_paths: Option<BTreeMap<u32, String>>,
    pub regions: BTreeMap<String, PolygonSet>,
    pub exclusion_regions: BTreeMap<String, PolygonSet>,
    pub process: Option<String>,
    pub corner: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatus {
    Clean,
    Violations,
    NotRun,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticCode {
    MissingContext,
    ContextNotApplicable,
    Unsupported,
    InvalidGeometry,
    InvalidModel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct CheckedRuleResult {
    pub rule_id: String,
    pub status: RuleStatus,
    pub violations: Vec<Violation>,
    pub diagnostics: Vec<RuleDiagnostic>,
}

#[derive(Clone, Debug)]
pub struct CheckedDrcReport {
    pub deck_id: String,
    pub model_revision: String,
    pub legacy_adapter: bool,
    pub rules: Vec<CheckedRuleResult>,
}

impl CheckedDrcReport {
    pub fn is_clean(&self) -> bool {
        !self.rules.is_empty()
            && self
                .rules
                .iter()
                .all(|rule| rule.status == RuleStatus::Clean)
    }
}

/// Explicit W1 compatibility adapter. It preserves all legacy results but does not
/// advertise production contextual semantics.
pub fn run_legacy_adapter(store: &GeometryStore, deck: &Deck) -> CheckedDrcReport {
    let report = run_drc(store, deck);
    let mut by_rule = BTreeMap::<String, Vec<Violation>>::new();
    for violation in report.violations {
        by_rule
            .entry(violation.rule_id.clone())
            .or_default()
            .push(violation);
    }
    let rules = deck
        .drc_rules
        .iter()
        .map(|rule| {
            let violations = by_rule.remove(rule.id()).unwrap_or_default();
            CheckedRuleResult {
                rule_id: rule.id().to_owned(),
                status: if violations.is_empty() {
                    RuleStatus::Clean
                } else {
                    RuleStatus::Violations
                },
                violations,
                diagnostics: Vec::new(),
            }
        })
        .collect();
    CheckedDrcReport {
        deck_id: "legacy-w1".into(),
        model_revision: "legacy-w1".into(),
        legacy_adapter: true,
        rules,
    }
}

/// Run the strict production deck. Currently exact min-area and explicit-region
/// density are production-enabled. Other declared operations return `Unsupported`
/// until their private legacy bbox measurements have migrated to the exact kernel.
pub fn run_checked(
    store: &GeometryStore,
    deck: &ProductionDeck,
    layers: &LayerTable,
    context: &DrcContext,
) -> CheckedDrcReport {
    let mut evaluator =
        DerivedEvaluator::new_checked(store, &deck.derived_layers, layers.id_to_name.len());
    let rules = deck
        .schema
        .rules
        .iter()
        .map(|rule| {
            if let Some(result) = check_context(rule.id(), rule.selector(), context) {
                return result;
            }
            match run_rule(rule, deck, layers, &mut evaluator) {
                Ok(violations) => CheckedRuleResult {
                    rule_id: rule.id().to_owned(),
                    status: if violations.is_empty() {
                        RuleStatus::Clean
                    } else {
                        RuleStatus::Violations
                    },
                    violations,
                    diagnostics: Vec::new(),
                },
                Err(error) => CheckedRuleResult {
                    rule_id: rule.id().to_owned(),
                    status: RuleStatus::Error,
                    violations: Vec::new(),
                    diagnostics: vec![RuleDiagnostic {
                        code: match error {
                            RunError::Unsupported(_) => DiagnosticCode::Unsupported,
                            RunError::Derived(_) => DiagnosticCode::InvalidGeometry,
                            RunError::Expression(_) => DiagnosticCode::InvalidModel,
                        },
                        message: error.to_string(),
                    }],
                },
            }
        })
        .collect();
    CheckedDrcReport {
        deck_id: deck.schema.deck_id.clone(),
        model_revision: deck.schema.model_revision.clone(),
        legacy_adapter: false,
        rules,
    }
}

fn run_rule(
    rule: &ProductionRuleSchema,
    deck: &ProductionDeck,
    layers: &LayerTable,
    evaluator: &mut DerivedEvaluator<'_>,
) -> Result<Vec<Violation>, RunError> {
    match rule {
        ProductionRuleSchema::MinArea { id, layer, limit, .. } => {
            let set = evaluate_source(layer, layers, evaluator)?;
            let limit = evaluate_expression(limit, deck)?.value;
            let mut violations = Vec::new();
            for polygon in set.polygons() {
                let area = polygon.area2() as f64 / 2.0;
                if area < limit {
                    let point = polygon.outer().vertices()[0];
                    violations.push(Violation {
                        rule_id: id.clone(),
                        kind: "min_area".into(),
                        layer: source_name(layer),
                        measured: saturating_i64(area),
                        limit: saturating_i64(limit),
                        x: point.x,
                        y: point.y,
                    });
                }
            }
            Ok(violations)
        }
        ProductionRuleSchema::MinEnclosedArea { id, layer, limit, .. } => {
            let set = evaluate_source(layer, layers, evaluator)?;
            let limit = evaluate_expression(limit, deck)?.value;
            let mut violations = Vec::new();
            for polygon in set.polygons() {
                for hole in polygon.holes() {
                    let area = -(hole.signed_area2() as f64) / 2.0;
                    if area < limit {
                        let point = hole.vertices()[0];
                        violations.push(Violation {
                            rule_id: id.clone(), kind: "min_enclosed_area".into(),
                            layer: source_name(layer), measured: saturating_i64(area),
                            limit: saturating_i64(limit), x: point.x, y: point.y,
                        });
                    }
                }
            }
            Ok(violations)
        }
        ProductionRuleSchema::Cheesing {
            id, layer, slots, max_area_without_slot, ..
        } => {
            let set = evaluate_source(layer, layers, evaluator)?;
            let slots_set = evaluate_source(slots, layers, evaluator)?;
            let limit = evaluate_expression(max_area_without_slot, deck)?.value;
            let mut violations = Vec::new();
            for polygon in set.polygons() {
                let area = polygon.area2() as f64 / 2.0;
                if area <= limit || !polygon.holes().is_empty() { continue; }
                let has_explicit_slot = slots_set.polygons().iter().any(|slot| {
                    crate::geometry::exact::classify_polygon_contact(
                        polygon.outer(), slot.outer(),
                    ) != crate::geometry::exact::PolygonContact::Disjoint
                });
                if !has_explicit_slot {
                    let point = polygon.outer().vertices()[0];
                    violations.push(Violation {
                        rule_id: id.clone(), kind: "cheesing".into(),
                        layer: source_name(layer), measured: saturating_i64(area),
                        limit: saturating_i64(limit), x: point.x, y: point.y,
                    });
                }
            }
            Ok(violations)
        }
        ProductionRuleSchema::Density {
            id,
            layer,
            region,
            exclusion,
            window,
            step,
            min,
            max,
            cmp_model,
            ..
        } => {
            if cmp_model.is_some() {
                return Err(RunError::Unsupported(
                    "CMP model execution requires calibrated model inputs; the schema is retained but no coefficients are invented".into(),
                ));
            }
            let material = evaluate_source(layer, layers, evaluator)?;
            let mut region = evaluate_source(region, layers, evaluator)?;
            if let Some(exclusion) = exclusion {
                region = rectilinear_subtraction(&region, &evaluate_source(exclusion, layers, evaluator)?)?;
            }
            let window = positive_i32(evaluate_expression(window, deck)?.value, "window")?;
            let step = positive_i32(evaluate_expression(step, deck)?.value, "step")?;
            if step > window {
                return Err(RunError::Expression("density step cannot exceed window (unchecked gaps)".into()));
            }
            let min = min.as_ref().map(|expr| evaluate_expression(expr, deck)).transpose()?.map(|q| q.value);
            let max = max.as_ref().map(|expr| evaluate_expression(expr, deck)).transpose()?.map(|q| q.value);
            exact_density(id, layer, &material, &region, window, step, min, max)
        }
        ProductionRuleSchema::MinWidth { .. }
        | ProductionRuleSchema::Spacing { .. }
        | ProductionRuleSchema::PrlSpacing { .. }
        | ProductionRuleSchema::EolSpacing { .. }
        | ProductionRuleSchema::Enclosure { .. }
        | ProductionRuleSchema::Extension { .. }
        | ProductionRuleSchema::CutClass { .. }
        | ProductionRuleSchema::Antenna { .. }
        | ProductionRuleSchema::MultiPatterning { .. } => Err(RunError::Unsupported(
            "rule is schema-valid but its legacy private geometry kernel has not yet migrated to exact production execution".into(),
        )),
    }
}

fn exact_density(
    id: &str,
    layer: &LayerSourceSchema,
    material: &PolygonSet,
    region: &PolygonSet,
    window: i32,
    step: i32,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<Vec<Violation>, RunError> {
    let Some((xmin, ymin, xmax, ymax)) = set_bbox(region) else {
        return Err(RunError::Expression("density region is empty".into()));
    };
    let mut violations = Vec::new();
    let mut y = ymin;
    loop {
        let mut x = xmin;
        loop {
            let x1 = x
                .checked_add(window)
                .ok_or_else(|| RunError::Expression("window coordinate overflow".into()))?
                .min(xmax);
            let y1 = y
                .checked_add(window)
                .ok_or_else(|| RunError::Expression("window coordinate overflow".into()))?
                .min(ymax);
            let window_set = rectangle(x, y, x1, y1)?;
            let scoped_window = rectilinear_intersection(region, &window_set)?;
            let denominator = scoped_window.area2() as f64 / 2.0;
            if denominator > 0.0 {
                let covered =
                    rectilinear_intersection(material, &scoped_window)?.area2() as f64 / 2.0;
                let fraction = covered / denominator;
                let bad_limit = min
                    .filter(|limit| fraction < *limit)
                    .map(|limit| ("min_density", limit))
                    .or_else(|| {
                        max.filter(|limit| fraction > *limit)
                            .map(|limit| ("max_density", limit))
                    });
                if let Some((kind, limit)) = bad_limit {
                    violations.push(Violation {
                        rule_id: id.into(),
                        kind: kind.into(),
                        layer: source_name(layer),
                        measured: saturating_i64(fraction * 1_000_000.0),
                        limit: saturating_i64(limit * 1_000_000.0),
                        x,
                        y,
                    });
                }
            }
            if x1 == xmax {
                break;
            }
            x = x
                .checked_add(step)
                .ok_or_else(|| RunError::Expression("density step overflow".into()))?;
            if x >= xmax {
                break;
            }
        }
        if y.checked_add(window).unwrap_or(i32::MAX) >= ymax {
            break;
        }
        y = y
            .checked_add(step)
            .ok_or_else(|| RunError::Expression("density step overflow".into()))?;
        if y >= ymax {
            break;
        }
    }
    Ok(violations)
}

fn evaluate_source(
    source: &LayerSourceSchema,
    layers: &LayerTable,
    evaluator: &mut DerivedEvaluator<'_>,
) -> Result<PolygonSet, RunError> {
    let value = match source {
        LayerSourceSchema::Base { name: layer } => {
            let layer = layers
                .id(layer)
                .ok_or_else(|| RunError::Expression(format!("unknown layer `{layer}`")))?;
            evaluator.evaluate(&DerivedExpr::Layer {
                layer: LayerExprRef::Base { layer },
            })?
        }
        LayerSourceSchema::Derived { name } => evaluator.evaluate_named(name)?,
    };
    match value {
        DerivedValue::Area(set) => Ok(set),
        DerivedValue::Edges(_) => Err(RunError::Unsupported(
            "area rule received an edge layer".into(),
        )),
    }
}

fn check_context(
    id: &str,
    selector: &ContextSelector,
    context: &DrcContext,
) -> Option<CheckedRuleResult> {
    let missing = if selector.net_relation.is_some() && context.poly_nets.is_none() {
        Some("polygon-to-net attribution")
    } else if selector.min_voltage_difference.is_some() && context.net_voltages.is_none() {
        Some("net voltage evidence")
    } else if selector.domains.is_some() && context.net_domains.is_none() {
        Some("net domain evidence")
    } else if selector.cells.is_some() && context.poly_cells.is_none() {
        Some("cell-name attribution")
    } else if selector.hierarchy_prefix.is_some() && context.hierarchy_paths.is_none() {
        Some("hierarchy paths")
    } else if selector.process.is_some() && context.process.is_none() {
        Some("process identity")
    } else if selector.corner.is_some() && context.corner.is_none() {
        Some("corner identity")
    } else if selector
        .region
        .as_ref()
        .is_some_and(|name| !context.regions.contains_key(name))
    {
        Some("named region geometry")
    } else {
        None
    };
    if let Some(evidence) = missing {
        return Some(CheckedRuleResult {
            rule_id: id.into(),
            status: RuleStatus::Error,
            violations: Vec::new(),
            diagnostics: vec![RuleDiagnostic {
                code: DiagnosticCode::MissingContext,
                message: format!("rule requires missing {evidence}"),
            }],
        });
    }
    if selector.net_relation.is_some()
        || selector.min_voltage_difference.is_some()
        || selector.domains.is_some()
        || selector.region.is_some()
        || selector.cells.is_some()
        || selector.hierarchy_prefix.is_some()
    {
        return Some(CheckedRuleResult {
            rule_id: id.into(),
            status: RuleStatus::Error,
            violations: Vec::new(),
            diagnostics: vec![RuleDiagnostic {
                code: DiagnosticCode::Unsupported,
                message: "required per-object context is present but contextual geometry filtering is not implemented".into(),
            }],
        });
    }
    let not_applicable = selector
        .process
        .as_ref()
        .is_some_and(|expected| context.process.as_ref() != Some(expected))
        || selector
            .corner
            .as_ref()
            .is_some_and(|expected| context.corner.as_ref() != Some(expected));
    not_applicable.then(|| CheckedRuleResult {
        rule_id: id.into(),
        status: RuleStatus::NotRun,
        violations: Vec::new(),
        diagnostics: vec![RuleDiagnostic {
            code: DiagnosticCode::ContextNotApplicable,
            message: "rule selector does not apply to the active process/corner".into(),
        }],
    })
}

fn evaluate_expression(
    expression: &ScalarExpr,
    deck: &ProductionDeck,
) -> Result<Quantity, RunError> {
    let mut stack = Vec::new();
    eval(
        expression,
        &deck.schema.variables,
        &deck.schema.tables,
        deck.schema.dbu_nm,
        &mut stack,
    )
    .map_err(RunError::Expression)
}

fn eval(
    expression: &ScalarExpr,
    variables: &BTreeMap<String, ScalarExpr>,
    tables: &BTreeMap<String, LookupTable>,
    dbu_nm: f64,
    stack: &mut Vec<String>,
) -> Result<Quantity, String> {
    use ScalarExpr::*;
    match expression {
        Literal { value, unit } => normalize(*value, *unit, dbu_nm),
        Variable { name } => {
            if stack.contains(name) {
                return Err(format!("variable cycle: {} -> {name}", stack.join(" -> ")));
            }
            let expression = variables
                .get(name)
                .ok_or_else(|| format!("unknown variable `{name}`"))?;
            stack.push(name.clone());
            let result = eval(expression, variables, tables, dbu_nm, stack);
            stack.pop();
            result
        }
        Add { lhs, rhs } | Subtract { lhs, rhs } => {
            let lhs = eval(lhs, variables, tables, dbu_nm, stack)?;
            let rhs = eval(rhs, variables, tables, dbu_nm, stack)?;
            if lhs.dimension != rhs.dimension {
                return Err("add/subtract operands have different dimensions".into());
            }
            let value = if matches!(expression, Add { .. }) {
                lhs.value + rhs.value
            } else {
                lhs.value - rhs.value
            };
            finite_quantity(value, lhs.dimension)
        }
        Multiply { lhs, rhs } => {
            let lhs = eval(lhs, variables, tables, dbu_nm, stack)?;
            let rhs = eval(rhs, variables, tables, dbu_nm, stack)?;
            let dimension = match (lhs.dimension, rhs.dimension) {
                (Dimension::Ratio, dimension) | (dimension, Dimension::Ratio) => dimension,
                (Dimension::Length, Dimension::Length) => Dimension::Area,
                _ => return Err("unsupported dimensional multiplication".into()),
            };
            finite_quantity(lhs.value * rhs.value, dimension)
        }
        Divide { lhs, rhs } => {
            let lhs = eval(lhs, variables, tables, dbu_nm, stack)?;
            let rhs = eval(rhs, variables, tables, dbu_nm, stack)?;
            if rhs.value == 0.0 {
                return Err("division by zero".into());
            }
            let dimension = if lhs.dimension == rhs.dimension {
                Dimension::Ratio
            } else if rhs.dimension == Dimension::Ratio {
                lhs.dimension
            } else {
                return Err("unsupported dimensional division".into());
            };
            finite_quantity(lhs.value / rhs.value, dimension)
        }
        Lookup { table, input } => {
            let table = tables
                .get(table)
                .ok_or_else(|| format!("unknown lookup table `{table}`"))?;
            let input = eval(input, variables, tables, dbu_nm, stack)?;
            if input.dimension != table.input_unit.dimension() {
                return Err("lookup input has wrong dimension".into());
            }
            let raw_input = denormalize(input.value, table.input_unit, dbu_nm)?;
            let output = table_lookup(table, raw_input)?;
            normalize(output, table.output_unit, dbu_nm)
        }
    }
}

fn validate_tables(tables: &BTreeMap<String, LookupTable>, dbu_nm: f64) -> Result<(), DeckError> {
    for (name, table) in tables {
        if name.trim().is_empty() || table.points.is_empty() {
            return Err(DeckError::Schema(format!("lookup table `{name}` is empty")));
        }
        let mut previous = f64::NEG_INFINITY;
        for point in &table.points {
            if !point.input.is_finite() || !point.output.is_finite() || point.input <= previous {
                return Err(DeckError::Schema(format!(
                    "lookup table `{name}` points must be finite with strictly increasing input"
                )));
            }
            previous = point.input;
        }
        normalize(table.points[0].input, table.input_unit, dbu_nm).map_err(DeckError::Schema)?;
        normalize(table.points[0].output, table.output_unit, dbu_nm).map_err(DeckError::Schema)?;
    }
    Ok(())
}

fn validate_variables(
    variables: &BTreeMap<String, ScalarExpr>,
    tables: &BTreeMap<String, LookupTable>,
    dbu_nm: f64,
) -> Result<(), DeckError> {
    for (name, expression) in variables {
        if name.trim().is_empty() {
            return Err(DeckError::Schema("variable name must not be empty".into()));
        }
        eval(
            expression,
            variables,
            tables,
            dbu_nm,
            &mut vec![name.clone()],
        )
        .map_err(DeckError::Schema)?;
    }
    Ok(())
}

fn validate_rule(
    rule: &ProductionRuleSchema,
    schema: &ProductionDeckSchema,
    layers: &LayerTable,
) -> Result<(), DeckError> {
    let expected: &[(ScalarExpr, Dimension)] = match rule {
        ProductionRuleSchema::MinWidth { limit, .. }
        | ProductionRuleSchema::Spacing { limit, .. }
        | ProductionRuleSchema::Enclosure { limit, .. }
        | ProductionRuleSchema::Extension { limit, .. } => &[(limit.clone(), Dimension::Length)],
        ProductionRuleSchema::PrlSpacing {
            parallel_run_length,
            limit,
            ..
        } => &[
            (parallel_run_length.clone(), Dimension::Length),
            (limit.clone(), Dimension::Length),
        ],
        ProductionRuleSchema::EolSpacing {
            eol_width,
            within,
            limit,
            ..
        } => &[
            (eol_width.clone(), Dimension::Length),
            (within.clone(), Dimension::Length),
            (limit.clone(), Dimension::Length),
        ],
        ProductionRuleSchema::MinArea { limit, .. }
        | ProductionRuleSchema::MinEnclosedArea { limit, .. } => {
            &[(limit.clone(), Dimension::Area)]
        }
        ProductionRuleSchema::Cheesing {
            max_area_without_slot,
            ..
        } => &[(max_area_without_slot.clone(), Dimension::Area)],
        ProductionRuleSchema::CutClass {
            min_count,
            within,
            redundancy,
            ..
        } => {
            let mut values = vec![
                (min_count.clone(), Dimension::Count),
                (within.clone(), Dimension::Length),
            ];
            if let Some(redundancy) = redundancy {
                values.push((redundancy.clone(), Dimension::Count));
            }
            return validate_rule_values(rule, &values, schema, layers);
        }
        ProductionRuleSchema::Density {
            window,
            step,
            min,
            max,
            ..
        } => {
            if min.is_none() && max.is_none() {
                return Err(DeckError::Rule {
                    id: rule.id().into(),
                    message: "density needs min and/or max".into(),
                });
            }
            let mut values = vec![
                (window.clone(), Dimension::Length),
                (step.clone(), Dimension::Length),
            ];
            if let Some(min) = min {
                values.push((min.clone(), Dimension::Ratio));
            }
            if let Some(max) = max {
                values.push((max.clone(), Dimension::Ratio));
            }
            return validate_rule_values(rule, &values, schema, layers);
        }
        ProductionRuleSchema::Antenna {
            ratio_limit,
            fabrication_stage,
            ..
        } => {
            if *fabrication_stage == 0 {
                return Err(DeckError::Rule {
                    id: rule.id().into(),
                    message: "fabrication_stage must be positive".into(),
                });
            }
            &[(ratio_limit.clone(), Dimension::Ratio)]
        }
        ProductionRuleSchema::MultiPatterning {
            colors,
            spacing,
            max_search_states,
            ..
        } => {
            if max_search_states == &Some(0) {
                return Err(DeckError::Rule {
                    id: rule.id().into(),
                    message: "max_search_states must be positive".into(),
                });
            }
            &[
                (colors.clone(), Dimension::Count),
                (spacing.clone(), Dimension::Length),
            ]
        }
    };
    validate_rule_values(rule, expected, schema, layers)
}

fn validate_rule_values(
    rule: &ProductionRuleSchema,
    expected: &[(ScalarExpr, Dimension)],
    schema: &ProductionDeckSchema,
    layers: &LayerTable,
) -> Result<(), DeckError> {
    for source in rule_sources(rule) {
        resolve_source(source, layers, &schema.derived_layers)?;
    }
    for (expression, dimension) in expected {
        let quantity = eval(
            expression,
            &schema.variables,
            &schema.tables,
            schema.dbu_nm,
            &mut Vec::new(),
        )
        .map_err(|message| DeckError::Rule {
            id: rule.id().into(),
            message,
        })?;
        if quantity.dimension != *dimension || !quantity.value.is_finite() || quantity.value <= 0.0
        {
            return Err(DeckError::Rule {
                id: rule.id().into(),
                message: format!("expected positive {dimension:?}, got {quantity:?}"),
            });
        }
    }
    if let Some(expr) = &rule.selector().min_voltage_difference {
        let value = eval(
            expr,
            &schema.variables,
            &schema.tables,
            schema.dbu_nm,
            &mut Vec::new(),
        )
        .map_err(|message| DeckError::Rule {
            id: rule.id().into(),
            message,
        })?;
        if value.dimension != Dimension::Voltage || value.value < 0.0 {
            return Err(DeckError::Rule {
                id: rule.id().into(),
                message: "min_voltage_difference must be non-negative volts".into(),
            });
        }
    }
    Ok(())
}

fn rule_sources(rule: &ProductionRuleSchema) -> Vec<&LayerSourceSchema> {
    match rule {
        ProductionRuleSchema::MinWidth { layer, .. }
        | ProductionRuleSchema::PrlSpacing { layer, .. }
        | ProductionRuleSchema::EolSpacing { layer, .. }
        | ProductionRuleSchema::MinArea { layer, .. }
        | ProductionRuleSchema::MinEnclosedArea { layer, .. }
        | ProductionRuleSchema::MultiPatterning { layer, .. } => vec![layer],
        ProductionRuleSchema::Spacing { layer, other, .. } => {
            std::iter::once(layer).chain(other.iter()).collect()
        }
        ProductionRuleSchema::Enclosure { outer, inner, .. } => vec![outer, inner],
        ProductionRuleSchema::Cheesing { layer, slots, .. } => vec![layer, slots],
        ProductionRuleSchema::Extension {
            layer, reference, ..
        } => vec![layer, reference],
        ProductionRuleSchema::CutClass { cut, .. } => vec![cut],
        ProductionRuleSchema::Density {
            layer,
            region,
            exclusion,
            ..
        } => std::iter::once(layer)
            .chain(std::iter::once(region))
            .chain(exclusion.iter())
            .collect(),
        ProductionRuleSchema::Antenna {
            conductor,
            gate,
            diode,
            ..
        } => std::iter::once(conductor)
            .chain(std::iter::once(gate))
            .chain(diode.iter())
            .collect(),
    }
}

fn resolve_source(
    source: &LayerSourceSchema,
    layers: &LayerTable,
    derived: &BTreeMap<String, DerivedLayerSchema>,
) -> Result<ResolvedLayerSource, DeckError> {
    match source {
        LayerSourceSchema::Base { name } => layers
            .id(name)
            .map(ResolvedLayerSource::Base)
            .ok_or_else(|| DeckError::Schema(format!("unknown base layer `{name}`"))),
        LayerSourceSchema::Derived { name } => derived
            .contains_key(name)
            .then(|| ResolvedLayerSource::Derived(name.clone()))
            .ok_or_else(|| DeckError::Schema(format!("unknown derived layer `{name}`"))),
    }
}

fn resolve_derived(
    expression: &DerivedLayerSchema,
    layers: &LayerTable,
    definitions: &BTreeMap<String, DerivedLayerSchema>,
) -> Result<DerivedExpr, DeckError> {
    use DerivedLayerSchema::*;
    let recur =
        |expr: &DerivedLayerSchema| resolve_derived(expr, layers, definitions).map(Box::new);
    Ok(match expression {
        Layer { source } => DerivedExpr::Layer {
            layer: match resolve_source(source, layers, definitions)? {
                ResolvedLayerSource::Base(layer) => LayerExprRef::Base { layer },
                ResolvedLayerSource::Derived(name) => LayerExprRef::Derived { name },
            },
        },
        Union { operands } => DerivedExpr::Union {
            operands: operands
                .iter()
                .map(|expr| resolve_derived(expr, layers, definitions))
                .collect::<Result<_, _>>()?,
        },
        Intersection { lhs, rhs } => DerivedExpr::Intersection {
            lhs: recur(lhs)?,
            rhs: recur(rhs)?,
        },
        Subtraction { lhs, rhs } => DerivedExpr::Subtraction {
            lhs: recur(lhs)?,
            rhs: recur(rhs)?,
        },
        Xor { lhs, rhs } => DerivedExpr::Xor {
            lhs: recur(lhs)?,
            rhs: recur(rhs)?,
        },
        NotWithin { region, operand } => DerivedExpr::NotWithin {
            region: recur(region)?,
            operand: recur(operand)?,
        },
        Grow { operand, distance } => DerivedExpr::Grow {
            operand: recur(operand)?,
            distance: *distance,
        },
        Shrink { operand, distance } => DerivedExpr::Shrink {
            operand: recur(operand)?,
            distance: *distance,
        },
        Inside { operand, region } => DerivedExpr::Inside {
            operand: recur(operand)?,
            region: recur(region)?,
        },
        Outside { operand, region } => DerivedExpr::Outside {
            operand: recur(operand)?,
            region: recur(region)?,
        },
        Interacting { operand, other } => DerivedExpr::Interacting {
            operand: recur(operand)?,
            other: recur(other)?,
        },
    })
}

fn validate_derived_references(
    expression: &DerivedExpr,
    definitions: &BTreeMap<String, DerivedExpr>,
) -> Result<(), DeckError> {
    fn visit(
        expression: &DerivedExpr,
        definitions: &BTreeMap<String, DerivedExpr>,
        visiting: &mut Vec<String>,
    ) -> Result<(), DeckError> {
        use DerivedExpr::*;
        match expression {
            Layer {
                layer: LayerExprRef::Derived { name },
            } => {
                if visiting.contains(name) {
                    return Err(DeckError::Schema(format!(
                        "derived-layer cycle: {} -> {name}",
                        visiting.join(" -> ")
                    )));
                }
                let target = definitions
                    .get(name)
                    .ok_or_else(|| DeckError::Schema(format!("unknown derived layer `{name}`")))?;
                visiting.push(name.clone());
                visit(target, definitions, visiting)?;
                visiting.pop();
            }
            Layer { .. } => {}
            Union { operands } => {
                for operand in operands {
                    visit(operand, definitions, visiting)?;
                }
            }
            Intersection { lhs, rhs } | Subtraction { lhs, rhs } | Xor { lhs, rhs } => {
                visit(lhs, definitions, visiting)?;
                visit(rhs, definitions, visiting)?;
            }
            NotWithin { region, operand }
            | Inside { operand, region }
            | Outside { operand, region } => {
                visit(region, definitions, visiting)?;
                visit(operand, definitions, visiting)?;
            }
            Grow { operand, .. } | Shrink { operand, .. } | Edges { operand, .. } => {
                visit(operand, definitions, visiting)?
            }
            Interacting { operand, other } => {
                visit(operand, definitions, visiting)?;
                visit(other, definitions, visiting)?;
            }
        }
        Ok(())
    }
    visit(expression, definitions, &mut Vec::new())
}

fn normalize(value: f64, unit: Unit, dbu_nm: f64) -> Result<Quantity, String> {
    if !value.is_finite() {
        return Err("quantity must be finite".into());
    }
    let value = match unit {
        Unit::Dbu | Unit::Ratio | Unit::Count | Unit::Volt => value,
        Unit::Nanometer => value / dbu_nm,
        Unit::SquareDbu => value,
        Unit::SquareNanometer => value / (dbu_nm * dbu_nm),
    };
    finite_quantity(value, unit.dimension())
}

fn denormalize(value: f64, unit: Unit, dbu_nm: f64) -> Result<f64, String> {
    let value = match unit {
        Unit::Dbu | Unit::Ratio | Unit::Count | Unit::Volt | Unit::SquareDbu => value,
        Unit::Nanometer => value * dbu_nm,
        Unit::SquareNanometer => value * dbu_nm * dbu_nm,
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| "quantity conversion overflow".into())
}

fn finite_quantity(value: f64, dimension: Dimension) -> Result<Quantity, String> {
    value
        .is_finite()
        .then_some(Quantity { value, dimension })
        .ok_or_else(|| "expression is not finite".into())
}

fn table_lookup(table: &LookupTable, input: f64) -> Result<f64, String> {
    if input <= table.points[0].input {
        return Ok(table.points[0].output);
    }
    for pair in table.points.windows(2) {
        if input <= pair[1].input {
            if !table.interpolate {
                return Ok(pair[0].output);
            }
            let fraction = (input - pair[0].input) / (pair[1].input - pair[0].input);
            return Ok(pair[0].output + fraction * (pair[1].output - pair[0].output));
        }
    }
    Ok(table.points.last().unwrap().output)
}

fn positive_i32(value: f64, field: &str) -> Result<i32, RunError> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > f64::from(i32::MAX) {
        return Err(RunError::Expression(format!(
            "{field} must resolve to a positive integer DBU value"
        )));
    }
    Ok(value as i32)
}

fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32) -> Result<PolygonSet, RunError> {
    Ok(PolygonSet::from_polygon(Polygon::from_outer(vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ])?))
}

fn set_bbox(set: &PolygonSet) -> Option<(i32, i32, i32, i32)> {
    let mut bbox = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let mut any = false;
    for polygon in set.polygons() {
        for point in polygon.outer().vertices() {
            any = true;
            bbox.0 = bbox.0.min(point.x);
            bbox.1 = bbox.1.min(point.y);
            bbox.2 = bbox.2.max(point.x);
            bbox.3 = bbox.3.max(point.y);
        }
    }
    any.then_some(bbox)
}

fn source_name(source: &LayerSourceSchema) -> String {
    match source {
        LayerSourceSchema::Base { name } => name.clone(),
        LayerSourceSchema::Derived { name } => format!("derived:{name}"),
    }
}

fn saturating_i64(value: f64) -> i64 {
    value.round().clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeckError {
    Schema(String),
    Rule { id: String, message: String },
}

impl fmt::Display for DeckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(message) => write!(f, "production DRC deck: {message}"),
            Self::Rule { id, message } => write!(f, "production DRC rule `{id}`: {message}"),
        }
    }
}

impl std::error::Error for DeckError {}

#[derive(Clone, Debug)]
enum RunError {
    Unsupported(String),
    Derived(DerivedError),
    Expression(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
            Self::Derived(error) => write!(f, "derived geometry: {error}"),
            Self::Expression(message) => write!(f, "model expression: {message}"),
        }
    }
}

impl From<DerivedError> for RunError {
    fn from(value: DerivedError) -> Self {
        Self::Derived(value)
    }
}

impl From<crate::geometry::exact::ExactGeometryError> for RunError {
    fn from(value: crate::geometry::exact::ExactGeometryError) -> Self {
        Self::Derived(DerivedError::Geometry(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::LayerDef;
    use serde_json::json;
    use std::collections::HashMap;

    fn layers() -> LayerTable {
        LayerTable::from_defs(&HashMap::from([
            (
                "m1".into(),
                LayerDef {
                    layer: 1,
                    datatype: 0,
                },
            ),
            (
                "die".into(),
                LayerDef {
                    layer: 2,
                    datatype: 0,
                },
            ),
            (
                "ko".into(),
                LayerDef {
                    layer: 3,
                    datatype: 0,
                },
            ),
        ]))
    }

    #[test]
    fn strict_schema_rejects_unknown_fields_units_layers_and_variable_cycles() {
        let layers = layers();
        let mut value = json!({
            "schema_version": 1, "deck_id": "D", "model_revision": "R", "dbu_nm": 1.0,
            "rules": [{"kind":"min_area", "id":"M1.A", "layer":{"source":"base","name":"m1"},
                "limit":{"op":"literal","value":10.0,"unit":"square_dbu"}, "typo": 4}]
        });
        assert!(ProductionDeck::from_json(&value, &layers)
            .unwrap_err()
            .to_string()
            .contains("unknown field"));
        value["rules"][0].as_object_mut().unwrap().remove("typo");
        value["rules"][0]["layer"]["name"] = json!("missing");
        assert!(ProductionDeck::from_json(&value, &layers)
            .unwrap_err()
            .to_string()
            .contains("unknown base layer"));

        value["rules"][0]["layer"]["name"] = json!("m1");
        value["variables"] = json!({
            "a":{"op":"variable","name":"b"}, "b":{"op":"variable","name":"a"}
        });
        assert!(ProductionDeck::from_json(&value, &layers)
            .unwrap_err()
            .to_string()
            .contains("cycle"));
    }

    #[test]
    fn tables_interpolate_with_dimensions_and_derived_layer_feeds_min_area() {
        let layers = layers();
        let value = json!({
            "schema_version":1, "deck_id":"D", "model_revision":"R", "dbu_nm":2.0,
            "tables":{"area_by_count":{"input_unit":"count","output_unit":"square_nanometer",
                "points":[{"input":1.0,"output":40.0},{"input":3.0,"output":80.0}],"interpolate":true}},
            "derived_layers":{"trimmed":{"op":"subtraction",
                "lhs":{"op":"layer","source":{"source":"base","name":"m1"}},
                "rhs":{"op":"layer","source":{"source":"base","name":"ko"}}}},
            "rules":[{"kind":"min_area","id":"M1.A","layer":{"source":"derived","name":"trimmed"},
                "limit":{"op":"lookup","table":"area_by_count",
                    "input":{"op":"literal","value":2.0,"unit":"count"}}}]
        });
        let deck = ProductionDeck::from_json(&value, &layers).unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(layers.id("m1").unwrap(), 0, 0, 4, 4);
        store.add_rect(layers.id("ko").unwrap(), 2, 0, 2, 4);
        let report = run_checked(&store, &deck, &layers, &DrcContext::default());
        assert_eq!(report.rules[0].status, RuleStatus::Violations);
        assert_eq!(report.rules[0].violations[0].measured, 8);
        assert_eq!(report.rules[0].violations[0].limit, 15); // 60nm² / (2nm/DBU)²
    }

    #[test]
    fn missing_context_and_unsupported_rule_never_report_clean() {
        let layers = layers();
        let value = json!({
            "schema_version":1, "deck_id":"D", "model_revision":"R", "dbu_nm":1.0,
            "rules":[
                {"kind":"spacing","id":"M1.S","layer":{"source":"base","name":"m1"},"other":null,
                    "limit":{"op":"literal","value":2.0,"unit":"dbu"},
                    "when":{"net_relation":"different_net"}},
                {"kind":"min_width","id":"M1.W","layer":{"source":"base","name":"m1"},
                    "limit":{"op":"literal","value":2.0,"unit":"dbu"}}
            ]
        });
        let deck = ProductionDeck::from_json(&value, &layers).unwrap();
        let report = run_checked(
            &GeometryStore::new(),
            &deck,
            &layers,
            &DrcContext::default(),
        );
        assert_eq!(report.rules[0].status, RuleStatus::Error);
        assert_eq!(
            report.rules[0].diagnostics[0].code,
            DiagnosticCode::MissingContext
        );
        assert_eq!(report.rules[1].status, RuleStatus::Error);
        assert_eq!(
            report.rules[1].diagnostics[0].code,
            DiagnosticCode::Unsupported
        );
        assert!(!report.is_clean());
    }

    #[test]
    fn density_uses_explicit_region_exclusion_and_union_area() {
        let layers = layers();
        let value = json!({
            "schema_version":1, "deck_id":"D", "model_revision":"R", "dbu_nm":1.0,
            "rules":[{"kind":"density","id":"M1.D","layer":{"source":"base","name":"m1"},
                "region":{"source":"base","name":"die"},"exclusion":{"source":"base","name":"ko"},
                "window":{"op":"literal","value":10.0,"unit":"dbu"},
                "step":{"op":"literal","value":10.0,"unit":"dbu"},
                "min":{"op":"literal","value":0.75,"unit":"ratio"},"max":null}]
        });
        let deck = ProductionDeck::from_json(&value, &layers).unwrap();
        let mut store = GeometryStore::new();
        let m1 = layers.id("m1").unwrap();
        store.add_rect(m1, 0, 0, 6, 10);
        store.add_rect(m1, 4, 0, 6, 10); // overlap must not double count
        store.add_rect(layers.id("die").unwrap(), 0, 0, 10, 10);
        store.add_rect(layers.id("ko").unwrap(), 8, 0, 2, 10);
        let report = run_checked(&store, &deck, &layers, &DrcContext::default());
        assert_eq!(report.rules[0].status, RuleStatus::Clean); // 80/80 after keepout
    }

    #[test]
    fn exact_holes_and_explicit_keyhole_slots_drive_plate_rules() {
        let layers = layers();
        let value = json!({
            "schema_version":1, "deck_id":"D", "model_revision":"R", "dbu_nm":1.0,
            "derived_layers":{"ring":{"op":"subtraction",
                "lhs":{"op":"layer","source":{"source":"base","name":"die"}},
                "rhs":{"op":"layer","source":{"source":"base","name":"ko"}}}},
            "rules":[
                {"kind":"min_enclosed_area","id":"HOLE.A",
                    "layer":{"source":"derived","name":"ring"},
                    "limit":{"op":"literal","value":10.0,"unit":"square_dbu"}},
                {"kind":"cheesing","id":"PLATE.SLOT",
                    "layer":{"source":"derived","name":"ring"},
                    "slots":{"source":"base","name":"ko"},
                    "max_area_without_slot":{"op":"literal","value":50.0,"unit":"square_dbu"}}
            ]
        });
        let deck = ProductionDeck::from_json(&value, &layers).unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(layers.id("die").unwrap(), 0, 0, 10, 10);
        store.add_rect(layers.id("ko").unwrap(), 2, 2, 3, 3);
        let report = run_checked(&store, &deck, &layers, &DrcContext::default());
        assert_eq!(report.rules[0].status, RuleStatus::Violations);
        assert_eq!(report.rules[0].violations[0].measured, 9);
        assert_eq!(report.rules[1].status, RuleStatus::Clean);

        // A boundary-reaching slot reconstructs as a keyhole indentation, not a hole.
        let mut keyhole_store = GeometryStore::new();
        keyhole_store.add_rect(layers.id("die").unwrap(), 0, 0, 10, 10);
        keyhole_store.add_rect(layers.id("ko").unwrap(), 4, 4, 2, 6);
        let report = run_checked(&keyhole_store, &deck, &layers, &DrcContext::default());
        assert_eq!(report.rules[1].status, RuleStatus::Clean);
    }
}
