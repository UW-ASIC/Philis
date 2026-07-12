//! Deterministic, fail-closed correlation artifact and freeze-manifest tool.
//!
//! This binary deliberately has no dependency on a proprietary verifier. Vendor
//! adapters emit the neutral run artifact described in `verify/correlation/`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const RUN_SCHEMA: &str = "gdsverify.correlation.run/v1";
const CONFIG_SCHEMA: &str = "gdsverify.correlation.compare-config/v1";
const REPORT_SCHEMA: &str = "gdsverify.correlation.compare-report/v1";
const DISPOSITION_SCHEMA: &str = "gdsverify.correlation.dispositions/v1";
const FREEZE_SCHEMA: &str = "gdsverify.correlation.freeze/v1";
const FREEZE_REPORT_SCHEMA: &str = "gdsverify.correlation.freeze-report/v1";
const SUMMARY_SCHEMA: &str = "gdsverify.correlation.summary/v1";

const EXIT_SCHEMA: u8 = 2;
const EXIT_DELTA: u8 = 3;
const EXIT_FREEZE: u8 = 4;
const EXIT_USAGE: u8 = 64;

type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
struct AppError {
    code: u8,
    message: String,
}

impl AppError {
    fn schema(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_SCHEMA,
            message: message.into(),
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_USAGE,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct VersionedIdentity {
    name: String,
    version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ContentIdentity {
    id: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RunMetadata {
    engine: VersionedIdentity,
    tool: VersionedIdentity,
    deck: ContentIdentity,
    model: ContentIdentity,
    corpus: ContentIdentity,
    process: String,
    corner: String,
    command: Vec<String>,
    /// Provenance only. This is the sole field excluded from artifact identity.
    created_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Quantity {
    quantity: String,
    value: f64,
    unit: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum RingRole {
    Outer,
    Hole,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Point {
    x_nm: i64,
    y_nm: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Ring {
    role: RingRole,
    points: Vec<Point>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Bbox {
    min_x_nm: i64,
    min_y_nm: i64,
    max_x_nm: i64,
    max_y_nm: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct MarkerGeometry {
    bbox: Bbox,
    rings: Vec<Ring>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct DrcMarker {
    id: String,
    rule_id: String,
    layer: String,
    geometry: MarkerGeometry,
    measurements: Vec<Quantity>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LvsNet {
    id: String,
    pins: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct LvsDevice {
    id: String,
    kind: String,
    model: String,
    terminals: BTreeMap<String, String>,
    parameters: Vec<Quantity>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LvsMismatch {
    id: String,
    kind: String,
    schematic_objects: Vec<String>,
    layout_objects: Vec<String>,
    witness: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct LvsResults {
    nets: Vec<LvsNet>,
    devices: Vec<LvsDevice>,
    mismatches: Vec<LvsMismatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PexNode {
    id: String,
    net: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PexComponent {
    mechanism: String,
    value: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PexElement {
    id: String,
    kind: String,
    from: String,
    to: String,
    value: Quantity,
    components: Vec<PexComponent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PexResults {
    nodes: Vec<PexNode>,
    elements: Vec<PexElement>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum SignoffStatus {
    Clean,
    Violations,
    NotRun,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SignoffResult {
    check_id: String,
    status: SignoffStatus,
    result_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RunMetrics {
    runtime: Quantity,
    peak_memory: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CaseResult {
    case_id: String,
    drc_markers: Vec<DrcMarker>,
    lvs: LvsResults,
    pex: PexResults,
    signoff: Vec<SignoffResult>,
    metrics: RunMetrics,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RunArtifact {
    schema_version: String,
    metadata: RunMetadata,
    cases: Vec<CaseResult>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NumericTolerance {
    absolute: f64,
    relative: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CompareConfig {
    schema_version: String,
    geometry_tolerance_nm: i64,
    quantities: BTreeMap<String, NumericTolerance>,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA.to_string(),
            geometry_tolerance_nm: 0,
            quantities: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DeltaKind {
    Missing,
    Extra,
    Value,
    Unit,
    Geometry,
    Topology,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Delta {
    fingerprint: String,
    path: String,
    kind: DeltaKind,
    expected: Value,
    actual: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DispositionStatus {
    Approved,
    Rejected,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Disposition {
    fingerprint: String,
    owner: String,
    reason: String,
    /// Must equal the delta's complete JSON-pointer path. Wildcards are invalid.
    scope: String,
    created_at: String,
    expires_at: String,
    status: DispositionStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DispositionFile {
    schema_version: String,
    dispositions: Vec<Disposition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct AcceptedDelta {
    delta: Delta,
    disposition_owner: String,
    disposition_reason: String,
    disposition_expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct CompareReport {
    schema_version: String,
    golden_identity_sha256: String,
    actual_identity_sha256: String,
    geometry_tolerance_nm: i64,
    quantity_tolerances: BTreeMap<String, NumericTolerance>,
    accepted_deltas: Vec<AcceptedDelta>,
    unwaived_deltas: Vec<Delta>,
    unused_dispositions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FreezeEntry {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FreezeManifest {
    schema_version: String,
    inputs: Vec<String>,
    entries: Vec<FreezeEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FreezeDelta {
    path: String,
    kind: String,
    expected_sha256: Option<String>,
    actual_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FreezeReport {
    schema_version: String,
    manifest_sha256: String,
    deltas: Vec<FreezeDelta>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ArtifactSummary {
    schema_version: String,
    artifact_identity_sha256: String,
    case_count: usize,
    drc_marker_count: usize,
    lvs_net_count: usize,
    lvs_device_count: usize,
    lvs_mismatch_count: usize,
    pex_node_count: usize,
    pex_element_count: usize,
    signoff_status_counts: BTreeMap<String, usize>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("correlation: {}", err.message);
            ExitCode::from(err.code)
        }
    }
}

fn run(args: Vec<String>) -> AppResult<u8> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(AppError::usage(usage()));
    };
    match command {
        "compare" => command_compare(&args[1..]),
        "freeze" => command_freeze(&args[1..]),
        "verify-freeze" => command_verify_freeze(&args[1..]),
        "summarize" => command_summarize(&args[1..]),
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(0)
        }
        other => Err(AppError::usage(format!(
            "unknown subcommand `{other}`\n{}",
            usage()
        ))),
    }
}

fn usage() -> &'static str {
    "Usage:\n  correlation compare GOLDEN ACTUAL [--config FILE] [--dispositions FILE] [--report FILE]\n  correlation freeze --root DIR [--output FILE] INPUT...\n  correlation verify-freeze MANIFEST --root DIR [--report FILE]\n  correlation summarize ARTIFACT [--report FILE]\n\nExit codes: 0 accepted, 2 parse/schema error, 3 unwaived comparison delta, 4 freeze mismatch, 64 usage."
}

fn command_compare(args: &[String]) -> AppResult<u8> {
    if args.len() < 2 {
        return Err(AppError::usage("compare requires GOLDEN and ACTUAL"));
    }
    let golden_path = &args[0];
    let actual_path = &args[1];
    let options = parse_options(&args[2..], &["--config", "--dispositions", "--report"])?;
    let mut golden: RunArtifact = read_json(golden_path)?;
    let mut actual: RunArtifact = read_json(actual_path)?;
    validate_artifact(&golden, "golden")?;
    validate_artifact(&actual, "actual")?;
    canonicalize_artifact(&mut golden);
    canonicalize_artifact(&mut actual);

    let config = if let Some(path) = options.get("--config") {
        let config: CompareConfig = read_json(path)?;
        validate_compare_config(&config)?;
        config
    } else {
        CompareConfig::default()
    };
    let dispositions = if let Some(path) = options.get("--dispositions") {
        let file: DispositionFile = read_json(path)?;
        validate_dispositions(&file)?;
        file.dispositions
    } else {
        Vec::new()
    };

    let report = compare_artifacts(&golden, &actual, &config, &dispositions)?;
    emit_json(&report, options.get("--report").map(String::as_str))?;
    eprintln!(
        "comparison: {} accepted delta(s), {} unwaived delta(s), {} unused disposition(s)",
        report.accepted_deltas.len(),
        report.unwaived_deltas.len(),
        report.unused_dispositions.len()
    );
    Ok(if report.unwaived_deltas.is_empty() {
        0
    } else {
        EXIT_DELTA
    })
}

fn command_freeze(args: &[String]) -> AppResult<u8> {
    let (root, output, inputs) = parse_freeze_args(args)?;
    let manifest = build_freeze_manifest(Path::new(&root), &inputs)?;
    emit_json(&manifest, output.as_deref())?;
    eprintln!("freeze: {} file(s) recorded", manifest.entries.len());
    Ok(0)
}

fn command_verify_freeze(args: &[String]) -> AppResult<u8> {
    if args.is_empty() {
        return Err(AppError::usage(
            "verify-freeze requires MANIFEST and --root DIR",
        ));
    }
    let manifest_path = &args[0];
    let options = parse_options(&args[1..], &["--root", "--report"])?;
    let root = options
        .get("--root")
        .ok_or_else(|| AppError::usage("verify-freeze requires --root DIR"))?;
    let manifest: FreezeManifest = read_json(manifest_path)?;
    validate_freeze_manifest(&manifest)?;
    let report = verify_freeze_manifest(Path::new(root), &manifest)?;
    emit_json(&report, options.get("--report").map(String::as_str))?;
    eprintln!("freeze verification: {} delta(s)", report.deltas.len());
    Ok(if report.deltas.is_empty() {
        0
    } else {
        EXIT_FREEZE
    })
}

fn command_summarize(args: &[String]) -> AppResult<u8> {
    if args.is_empty() {
        return Err(AppError::usage("summarize requires ARTIFACT"));
    }
    let options = parse_options(&args[1..], &["--report"])?;
    let mut artifact: RunArtifact = read_json(&args[0])?;
    validate_artifact(&artifact, "artifact")?;
    canonicalize_artifact(&mut artifact);
    let summary = summarize_artifact(&artifact)?;
    emit_json(&summary, options.get("--report").map(String::as_str))?;
    eprintln!(
        "summary: {} case(s), {} DRC marker(s), {} LVS mismatch(es), {} PEX element(s)",
        summary.case_count,
        summary.drc_marker_count,
        summary.lvs_mismatch_count,
        summary.pex_element_count
    );
    Ok(0)
}

fn parse_options(args: &[String], allowed: &[&str]) -> AppResult<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let key = &args[index];
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::usage(format!("unknown option `{key}`")));
        }
        if result.contains_key(key) {
            return Err(AppError::usage(format!("duplicate option `{key}`")));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| AppError::usage(format!("option `{key}` requires a value")))?;
        result.insert(key.clone(), value.clone());
        index += 2;
    }
    Ok(result)
}

fn parse_freeze_args(args: &[String]) -> AppResult<(String, Option<String>, Vec<String>)> {
    let mut root = None;
    let mut output = None;
    let mut inputs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" | "--output" => {
                let key = args[index].as_str();
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| AppError::usage(format!("option `{key}` requires a value")))?;
                let slot = if key == "--root" {
                    &mut root
                } else {
                    &mut output
                };
                if slot.replace(value.clone()).is_some() {
                    return Err(AppError::usage(format!("duplicate option `{key}`")));
                }
                index += 2;
            }
            value if value.starts_with('-') => {
                return Err(AppError::usage(format!("unknown option `{value}`")));
            }
            value => {
                inputs.push(value.to_string());
                index += 1;
            }
        }
    }
    let root = root.ok_or_else(|| AppError::usage("freeze requires --root DIR"))?;
    if inputs.is_empty() {
        return Err(AppError::usage("freeze requires at least one INPUT"));
    }
    Ok((root, output, inputs))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> AppResult<T> {
    let bytes =
        fs::read(path).map_err(|err| AppError::schema(format!("cannot read `{path}`: {err}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|err| AppError::schema(format!("invalid JSON/schema in `{path}`: {err}")))
}

fn emit_json<T: Serialize>(value: &T, report_path: Option<&str>) -> AppResult<()> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|err| AppError::schema(format!("cannot serialize report: {err}")))?;
    bytes.push(b'\n');
    if let Some(path) = report_path {
        fs::write(path, bytes)
            .map_err(|err| AppError::schema(format!("cannot write report `{path}`: {err}")))?;
    } else {
        print!("{}", String::from_utf8_lossy(&bytes));
    }
    Ok(())
}

fn require_text(value: &str, path: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::schema(format!("{path} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_hash(hash: &str, path: &str) -> AppResult<()> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AppError::schema(format!(
            "{path} must be a 64-character lowercase SHA-256 hex string"
        )));
    }
    Ok(())
}

fn validate_quantity(quantity: &Quantity, path: &str) -> AppResult<()> {
    require_text(&quantity.quantity, &format!("{path}/quantity"))?;
    require_text(&quantity.unit, &format!("{path}/unit"))?;
    if !quantity.value.is_finite() {
        return Err(AppError::schema(format!("{path}/value must be finite")));
    }
    Ok(())
}

fn check_unique<'a>(values: impl IntoIterator<Item = &'a str>, path: &str) -> AppResult<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_text(value, path)?;
        if !seen.insert(value) {
            return Err(AppError::schema(format!(
                "duplicate ID `{value}` at {path}"
            )));
        }
    }
    Ok(())
}

fn validate_artifact(artifact: &RunArtifact, label: &str) -> AppResult<()> {
    if artifact.schema_version != RUN_SCHEMA {
        return Err(AppError::schema(format!(
            "{label}/schema_version must be `{RUN_SCHEMA}`"
        )));
    }
    let metadata = &artifact.metadata;
    require_text(
        &metadata.engine.name,
        &format!("{label}/metadata/engine/name"),
    )?;
    require_text(
        &metadata.engine.version,
        &format!("{label}/metadata/engine/version"),
    )?;
    require_text(&metadata.tool.name, &format!("{label}/metadata/tool/name"))?;
    require_text(
        &metadata.tool.version,
        &format!("{label}/metadata/tool/version"),
    )?;
    require_text(&metadata.deck.id, &format!("{label}/metadata/deck/id"))?;
    validate_hash(
        &metadata.deck.sha256,
        &format!("{label}/metadata/deck/sha256"),
    )?;
    require_text(&metadata.model.id, &format!("{label}/metadata/model/id"))?;
    validate_hash(
        &metadata.model.sha256,
        &format!("{label}/metadata/model/sha256"),
    )?;
    require_text(&metadata.corpus.id, &format!("{label}/metadata/corpus/id"))?;
    validate_hash(
        &metadata.corpus.sha256,
        &format!("{label}/metadata/corpus/sha256"),
    )?;
    require_text(&metadata.process, &format!("{label}/metadata/process"))?;
    require_text(&metadata.corner, &format!("{label}/metadata/corner"))?;
    if metadata.command.is_empty() || metadata.command.iter().any(|item| item.is_empty()) {
        return Err(AppError::schema(format!(
            "{label}/metadata/command must contain non-empty arguments"
        )));
    }
    parse_utc_timestamp(&metadata.created_at).map_err(|reason| {
        AppError::schema(format!("{label}/metadata/created_at is invalid: {reason}"))
    })?;
    check_unique(
        artifact.cases.iter().map(|case| case.case_id.as_str()),
        &format!("{label}/cases"),
    )?;

    for (case_index, case) in artifact.cases.iter().enumerate() {
        let base = format!("{label}/cases/{case_index}");
        check_unique(
            case.drc_markers.iter().map(|item| item.id.as_str()),
            &format!("{base}/drc_markers"),
        )?;
        check_unique(
            case.lvs.nets.iter().map(|item| item.id.as_str()),
            &format!("{base}/lvs/nets"),
        )?;
        check_unique(
            case.lvs.devices.iter().map(|item| item.id.as_str()),
            &format!("{base}/lvs/devices"),
        )?;
        check_unique(
            case.lvs.mismatches.iter().map(|item| item.id.as_str()),
            &format!("{base}/lvs/mismatches"),
        )?;
        check_unique(
            case.pex.nodes.iter().map(|item| item.id.as_str()),
            &format!("{base}/pex/nodes"),
        )?;
        check_unique(
            case.pex.elements.iter().map(|item| item.id.as_str()),
            &format!("{base}/pex/elements"),
        )?;
        check_unique(
            case.signoff.iter().map(|item| item.check_id.as_str()),
            &format!("{base}/signoff"),
        )?;

        for (index, marker) in case.drc_markers.iter().enumerate() {
            require_text(
                &marker.rule_id,
                &format!("{base}/drc_markers/{index}/rule_id"),
            )?;
            require_text(&marker.layer, &format!("{base}/drc_markers/{index}/layer"))?;
            validate_geometry(
                &marker.geometry,
                &format!("{base}/drc_markers/{index}/geometry"),
            )?;
            for (measurement_index, measurement) in marker.measurements.iter().enumerate() {
                validate_quantity(
                    measurement,
                    &format!("{base}/drc_markers/{index}/measurements/{measurement_index}"),
                )?;
            }
            check_unique(
                marker
                    .measurements
                    .iter()
                    .map(|measurement| measurement.quantity.as_str()),
                &format!("{base}/drc_markers/{index}/measurements/quantity"),
            )?;
        }
        for net in &case.lvs.nets {
            check_unique(
                net.pins.iter().map(String::as_str),
                &format!("{base}/lvs/nets/{}/pins", net.id),
            )?;
        }
        for (index, device) in case.lvs.devices.iter().enumerate() {
            require_text(&device.kind, &format!("{base}/lvs/devices/{index}/kind"))?;
            require_text(&device.model, &format!("{base}/lvs/devices/{index}/model"))?;
            if device.terminals.is_empty() {
                return Err(AppError::schema(format!(
                    "{base}/lvs/devices/{index}/terminals must not be empty"
                )));
            }
            for (terminal, net) in &device.terminals {
                require_text(
                    terminal,
                    &format!("{base}/lvs/devices/{index}/terminals/key"),
                )?;
                require_text(
                    net,
                    &format!("{base}/lvs/devices/{index}/terminals/{terminal}"),
                )?;
            }
            for (parameter_index, parameter) in device.parameters.iter().enumerate() {
                validate_quantity(
                    parameter,
                    &format!("{base}/lvs/devices/{index}/parameters/{parameter_index}"),
                )?;
            }
            check_unique(
                device
                    .parameters
                    .iter()
                    .map(|parameter| parameter.quantity.as_str()),
                &format!("{base}/lvs/devices/{index}/parameters/quantity"),
            )?;
        }
        for mismatch in &case.lvs.mismatches {
            require_text(
                &mismatch.kind,
                &format!("{base}/lvs/mismatches/{}/kind", mismatch.id),
            )?;
            require_text(
                &mismatch.witness,
                &format!("{base}/lvs/mismatches/{}/witness", mismatch.id),
            )?;
            check_unique(
                mismatch.schematic_objects.iter().map(String::as_str),
                &format!("{base}/lvs/mismatches/{}/schematic_objects", mismatch.id),
            )?;
            check_unique(
                mismatch.layout_objects.iter().map(String::as_str),
                &format!("{base}/lvs/mismatches/{}/layout_objects", mismatch.id),
            )?;
        }
        let node_ids: BTreeSet<_> = case.pex.nodes.iter().map(|node| node.id.as_str()).collect();
        for (index, node) in case.pex.nodes.iter().enumerate() {
            require_text(&node.net, &format!("{base}/pex/nodes/{index}/net"))?;
        }
        for (index, element) in case.pex.elements.iter().enumerate() {
            require_text(&element.kind, &format!("{base}/pex/elements/{index}/kind"))?;
            if !node_ids.contains(element.from.as_str()) || !node_ids.contains(element.to.as_str())
            {
                return Err(AppError::schema(format!(
                    "{base}/pex/elements/{index} references an unknown node"
                )));
            }
            validate_quantity(
                &element.value,
                &format!("{base}/pex/elements/{index}/value"),
            )?;
            check_unique(
                element
                    .components
                    .iter()
                    .map(|component| component.mechanism.as_str()),
                &format!("{base}/pex/elements/{index}/components"),
            )?;
            for (component_index, component) in element.components.iter().enumerate() {
                validate_quantity(
                    &component.value,
                    &format!("{base}/pex/elements/{index}/components/{component_index}/value"),
                )?;
                if component.value.quantity != element.value.quantity
                    || component.value.unit != element.value.unit
                {
                    return Err(AppError::schema(format!(
                        "{base}/pex/elements/{index}/components/{component_index} quantity and unit must match the total"
                    )));
                }
            }
        }
        for result in &case.signoff {
            check_unique(
                result.result_ids.iter().map(String::as_str),
                &format!("{base}/signoff/{}/result_ids", result.check_id),
            )?;
        }
        validate_quantity(&case.metrics.runtime, &format!("{base}/metrics/runtime"))?;
        validate_quantity(
            &case.metrics.peak_memory,
            &format!("{base}/metrics/peak_memory"),
        )?;
        if case.metrics.runtime.quantity != "runtime" || case.metrics.runtime.unit != "s" {
            return Err(AppError::schema(format!(
                "{base}/metrics/runtime must use quantity `runtime` and unit `s`"
            )));
        }
        if case.metrics.peak_memory.quantity != "memory" || case.metrics.peak_memory.unit != "byte"
        {
            return Err(AppError::schema(format!(
                "{base}/metrics/peak_memory must use quantity `memory` and unit `byte`"
            )));
        }
    }
    Ok(())
}

fn validate_geometry(geometry: &MarkerGeometry, path: &str) -> AppResult<()> {
    if geometry.bbox.min_x_nm > geometry.bbox.max_x_nm
        || geometry.bbox.min_y_nm > geometry.bbox.max_y_nm
    {
        return Err(AppError::schema(format!("{path}/bbox is inverted")));
    }
    if geometry.rings.is_empty() {
        return Err(AppError::schema(format!("{path}/rings must not be empty")));
    }
    if geometry
        .rings
        .iter()
        .filter(|ring| ring.role == RingRole::Outer)
        .count()
        != 1
    {
        return Err(AppError::schema(format!(
            "{path} must have exactly one outer ring"
        )));
    }
    let mut computed = Bbox {
        min_x_nm: i64::MAX,
        min_y_nm: i64::MAX,
        max_x_nm: i64::MIN,
        max_y_nm: i64::MIN,
    };
    for (ring_index, ring) in geometry.rings.iter().enumerate() {
        if ring.points.len() < 3 {
            return Err(AppError::schema(format!(
                "{path}/rings/{ring_index} needs at least three points"
            )));
        }
        let distinct: BTreeSet<_> = ring.points.iter().collect();
        if distinct.len() < 3 {
            return Err(AppError::schema(format!(
                "{path}/rings/{ring_index} needs three distinct points"
            )));
        }
        for point in &ring.points {
            computed.min_x_nm = computed.min_x_nm.min(point.x_nm);
            computed.min_y_nm = computed.min_y_nm.min(point.y_nm);
            computed.max_x_nm = computed.max_x_nm.max(point.x_nm);
            computed.max_y_nm = computed.max_y_nm.max(point.y_nm);
        }
    }
    if computed != geometry.bbox {
        return Err(AppError::schema(format!(
            "{path}/bbox does not bound its rings exactly"
        )));
    }
    Ok(())
}

fn validate_compare_config(config: &CompareConfig) -> AppResult<()> {
    if config.schema_version != CONFIG_SCHEMA {
        return Err(AppError::schema(format!(
            "config/schema_version must be `{CONFIG_SCHEMA}`"
        )));
    }
    if config.geometry_tolerance_nm < 0 {
        return Err(AppError::schema(
            "geometry_tolerance_nm must be nonnegative",
        ));
    }
    for (quantity, tolerance) in &config.quantities {
        require_text(quantity, "config/quantities/key")?;
        if !tolerance.absolute.is_finite()
            || !tolerance.relative.is_finite()
            || tolerance.absolute < 0.0
            || tolerance.relative < 0.0
        {
            return Err(AppError::schema(format!(
                "tolerance for `{quantity}` must contain finite nonnegative values"
            )));
        }
    }
    Ok(())
}

fn canonicalize_artifact(artifact: &mut RunArtifact) {
    artifact
        .cases
        .sort_by(|left, right| left.case_id.cmp(&right.case_id));
    for case in &mut artifact.cases {
        case.drc_markers
            .sort_by(|left, right| left.id.cmp(&right.id));
        for marker in &mut case.drc_markers {
            marker.measurements.sort_by(|left, right| {
                left.quantity
                    .cmp(&right.quantity)
                    .then(left.unit.cmp(&right.unit))
            });
            canonicalize_geometry(&mut marker.geometry);
        }
        case.lvs.nets.sort_by(|left, right| left.id.cmp(&right.id));
        for net in &mut case.lvs.nets {
            net.pins.sort();
        }
        case.lvs
            .devices
            .sort_by(|left, right| left.id.cmp(&right.id));
        for device in &mut case.lvs.devices {
            device.parameters.sort_by(|left, right| {
                left.quantity
                    .cmp(&right.quantity)
                    .then(left.unit.cmp(&right.unit))
            });
        }
        case.lvs
            .mismatches
            .sort_by(|left, right| left.id.cmp(&right.id));
        for mismatch in &mut case.lvs.mismatches {
            mismatch.schematic_objects.sort();
            mismatch.layout_objects.sort();
        }
        case.pex.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        case.pex
            .elements
            .sort_by(|left, right| left.id.cmp(&right.id));
        for element in &mut case.pex.elements {
            element
                .components
                .sort_by(|left, right| left.mechanism.cmp(&right.mechanism));
        }
        case.signoff
            .sort_by(|left, right| left.check_id.cmp(&right.check_id));
        for result in &mut case.signoff {
            result.result_ids.sort();
        }
    }
}

fn canonicalize_geometry(geometry: &mut MarkerGeometry) {
    for ring in &mut geometry.rings {
        if ring.points.first() == ring.points.last() {
            ring.points.pop();
        }
        let forward = canonical_ring_direction(&ring.points);
        let mut reverse_points = ring.points.clone();
        reverse_points.reverse();
        let reverse = canonical_ring_direction(&reverse_points);
        ring.points = if forward <= reverse { forward } else { reverse };
    }
    geometry.rings.sort();
}

fn canonical_ring_direction(points: &[Point]) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut best = points.to_vec();
    for offset in 1..points.len() {
        let rotated: Vec<_> = points[offset..]
            .iter()
            .chain(points[..offset].iter())
            .copied()
            .collect();
        if rotated < best {
            best = rotated;
        }
    }
    best
}

fn compare_artifacts(
    golden: &RunArtifact,
    actual: &RunArtifact,
    config: &CompareConfig,
    dispositions: &[Disposition],
) -> AppResult<CompareReport> {
    let golden_value = artifact_identity_value(golden)?;
    let actual_value = artifact_identity_value(actual)?;
    let mut deltas = Vec::new();
    compare_values("", &golden_value, &actual_value, config, &mut deltas)?;
    deltas.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.fingerprint.cmp(&right.fingerprint))
    });

    let disposition_by_fingerprint: BTreeMap<_, _> = dispositions
        .iter()
        .map(|entry| (entry.fingerprint.as_str(), entry))
        .collect();
    let mut used = BTreeSet::new();
    let mut accepted_deltas = Vec::new();
    let mut unwaived_deltas = Vec::new();
    for delta in deltas {
        let accepted = disposition_by_fingerprint
            .get(delta.fingerprint.as_str())
            .copied()
            .filter(|entry| {
                entry.status == DispositionStatus::Approved && entry.scope == delta.path
            });
        if let Some(entry) = accepted {
            used.insert(entry.fingerprint.clone());
            accepted_deltas.push(AcceptedDelta {
                delta,
                disposition_owner: entry.owner.clone(),
                disposition_reason: entry.reason.clone(),
                disposition_expires_at: entry.expires_at.clone(),
            });
        } else {
            unwaived_deltas.push(delta);
        }
    }
    let unused_dispositions = dispositions
        .iter()
        .filter(|entry| !used.contains(&entry.fingerprint))
        .map(|entry| entry.fingerprint.clone())
        .collect();
    Ok(CompareReport {
        schema_version: REPORT_SCHEMA.to_string(),
        golden_identity_sha256: sha256_hex(&canonical_json_bytes(&golden_value)?),
        actual_identity_sha256: sha256_hex(&canonical_json_bytes(&actual_value)?),
        geometry_tolerance_nm: config.geometry_tolerance_nm,
        quantity_tolerances: config.quantities.clone(),
        accepted_deltas,
        unwaived_deltas,
        unused_dispositions,
    })
}

fn artifact_identity_value(artifact: &RunArtifact) -> AppResult<Value> {
    let mut value = serde_json::to_value(artifact)
        .map_err(|err| AppError::schema(format!("cannot serialize run artifact: {err}")))?;
    let metadata = value
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::schema("serialized artifact has no metadata object"))?;
    metadata.remove("created_at");
    Ok(value)
}

fn compare_values(
    path: &str,
    expected: &Value,
    actual: &Value,
    config: &CompareConfig,
    deltas: &mut Vec<Delta>,
) -> AppResult<()> {
    if let (Some(expected_quantity), Some(actual_quantity)) =
        (value_as_quantity(expected), value_as_quantity(actual))
    {
        return compare_quantities(path, expected_quantity, actual_quantity, config, deltas);
    }
    if path.ends_with("/geometry") {
        let expected_geometry: MarkerGeometry =
            serde_json::from_value(expected.clone()).map_err(|err| {
                AppError::schema(format!("invalid expected marker geometry at {path}: {err}"))
            })?;
        let actual_geometry: MarkerGeometry =
            serde_json::from_value(actual.clone()).map_err(|err| {
                AppError::schema(format!("invalid actual marker geometry at {path}: {err}"))
            })?;
        if !geometry_matches(
            &expected_geometry,
            &actual_geometry,
            config.geometry_tolerance_nm,
        ) {
            push_delta(
                deltas,
                path,
                DeltaKind::Geometry,
                expected.clone(),
                actual.clone(),
            )?;
        }
        return Ok(());
    }
    match (expected, actual) {
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            let keys: BTreeSet<_> = expected_map.keys().chain(actual_map.keys()).collect();
            for key in keys {
                let child_path = format!("{path}/{}", pointer_escape(key));
                match (expected_map.get(key), actual_map.get(key)) {
                    (Some(left), Some(right)) => {
                        compare_values(&child_path, left, right, config, deltas)?
                    }
                    (Some(left), None) => push_delta(
                        deltas,
                        &child_path,
                        DeltaKind::Missing,
                        left.clone(),
                        Value::Null,
                    )?,
                    (None, Some(right)) => push_delta(
                        deltas,
                        &child_path,
                        DeltaKind::Extra,
                        Value::Null,
                        right.clone(),
                    )?,
                    (None, None) => unreachable!(),
                }
            }
        }
        (Value::Array(expected_items), Value::Array(actual_items)) => {
            if let Some(key_field) = keyed_array_field(expected_items, actual_items) {
                let expected_map = index_array(expected_items, key_field)?;
                let actual_map = index_array(actual_items, key_field)?;
                let keys: BTreeSet<_> = expected_map.keys().chain(actual_map.keys()).collect();
                for key in keys {
                    let child_path = format!("{path}/{}", pointer_escape(key));
                    match (expected_map.get(key), actual_map.get(key)) {
                        (Some(left), Some(right)) => {
                            compare_values(&child_path, left, right, config, deltas)?
                        }
                        (Some(left), None) => push_delta(
                            deltas,
                            &child_path,
                            DeltaKind::Missing,
                            (*left).clone(),
                            Value::Null,
                        )?,
                        (None, Some(right)) => push_delta(
                            deltas,
                            &child_path,
                            DeltaKind::Extra,
                            Value::Null,
                            (*right).clone(),
                        )?,
                        (None, None) => unreachable!(),
                    }
                }
            } else {
                let common = expected_items.len().min(actual_items.len());
                for index in 0..common {
                    compare_values(
                        &format!("{path}/{index}"),
                        &expected_items[index],
                        &actual_items[index],
                        config,
                        deltas,
                    )?;
                }
                for (index, item) in expected_items.iter().enumerate().skip(common) {
                    push_delta(
                        deltas,
                        &format!("{path}/{index}"),
                        DeltaKind::Missing,
                        item.clone(),
                        Value::Null,
                    )?;
                }
                for (index, item) in actual_items.iter().enumerate().skip(common) {
                    push_delta(
                        deltas,
                        &format!("{path}/{index}"),
                        DeltaKind::Extra,
                        Value::Null,
                        item.clone(),
                    )?;
                }
            }
        }
        _ if expected == actual => {}
        _ => push_delta(
            deltas,
            path,
            if is_topology_path(path) {
                DeltaKind::Topology
            } else {
                DeltaKind::Value
            },
            expected.clone(),
            actual.clone(),
        )?,
    }
    Ok(())
}

fn value_as_quantity(value: &Value) -> Option<Quantity> {
    let object = value.as_object()?;
    if !(object.contains_key("quantity")
        && object.contains_key("value")
        && object.contains_key("unit"))
    {
        return None;
    }
    serde_json::from_value(value.clone()).ok()
}

fn compare_quantities(
    path: &str,
    expected: Quantity,
    actual: Quantity,
    config: &CompareConfig,
    deltas: &mut Vec<Delta>,
) -> AppResult<()> {
    if expected.quantity != actual.quantity {
        push_delta(
            deltas,
            &format!("{path}/quantity"),
            DeltaKind::Value,
            Value::String(expected.quantity),
            Value::String(actual.quantity),
        )?;
        return Ok(());
    }
    if expected.unit != actual.unit {
        push_delta(
            deltas,
            &format!("{path}/unit"),
            DeltaKind::Unit,
            Value::String(expected.unit),
            Value::String(actual.unit),
        )?;
        return Ok(());
    }
    let tolerance = config
        .quantities
        .get(&expected.quantity)
        .copied()
        .unwrap_or(NumericTolerance {
            absolute: 0.0,
            relative: 0.0,
        });
    let allowed =
        tolerance.absolute + tolerance.relative * expected.value.abs().max(actual.value.abs());
    if (expected.value - actual.value).abs() > allowed {
        push_delta(
            deltas,
            &format!("{path}/value"),
            DeltaKind::Value,
            serde_json::to_value(expected.value).unwrap_or(Value::Null),
            serde_json::to_value(actual.value).unwrap_or(Value::Null),
        )?;
    }
    Ok(())
}

fn keyed_array_field(expected: &[Value], actual: &[Value]) -> Option<&'static str> {
    for field in ["case_id", "id", "check_id", "mechanism", "quantity"] {
        if expected
            .iter()
            .chain(actual.iter())
            .all(|value| value.get(field).and_then(Value::as_str).is_some())
            && !(expected.is_empty() && actual.is_empty())
        {
            return Some(field);
        }
    }
    None
}

fn index_array<'a>(items: &'a [Value], field: &str) -> AppResult<BTreeMap<&'a str, &'a Value>> {
    let mut result = BTreeMap::new();
    for item in items {
        let key = item
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::schema(format!("array item lacks string key `{field}`")))?;
        if result.insert(key, item).is_some() {
            return Err(AppError::schema(format!(
                "duplicate comparison key `{key}`"
            )));
        }
    }
    Ok(result)
}

fn geometry_matches(expected: &MarkerGeometry, actual: &MarkerGeometry, tolerance_nm: i64) -> bool {
    let close = |left: i64, right: i64| {
        (i128::from(left) - i128::from(right)).abs() <= i128::from(tolerance_nm)
    };
    if !close(expected.bbox.min_x_nm, actual.bbox.min_x_nm)
        || !close(expected.bbox.min_y_nm, actual.bbox.min_y_nm)
        || !close(expected.bbox.max_x_nm, actual.bbox.max_x_nm)
        || !close(expected.bbox.max_y_nm, actual.bbox.max_y_nm)
        || expected.rings.len() != actual.rings.len()
    {
        return false;
    }
    expected
        .rings
        .iter()
        .zip(&actual.rings)
        .all(|(left, right)| {
            left.role == right.role
                && left.points.len() == right.points.len()
                && left
                    .points
                    .iter()
                    .zip(&right.points)
                    .all(|(a, b)| close(a.x_nm, b.x_nm) && close(a.y_nm, b.y_nm))
        })
}

fn is_topology_path(path: &str) -> bool {
    path.contains("/pex/")
        && (path.ends_with("/from")
            || path.ends_with("/to")
            || path.ends_with("/net")
            || path.ends_with("/kind"))
        || path.contains("/lvs/") && (path.contains("/terminals/") || path.contains("/pins/"))
}

fn push_delta(
    deltas: &mut Vec<Delta>,
    path: &str,
    kind: DeltaKind,
    expected: Value,
    actual: Value,
) -> AppResult<()> {
    let payload = serde_json::json!({
        "actual": actual,
        "expected": expected,
        "kind": kind,
        "path": if path.is_empty() { "/" } else { path },
        "schema_version": "gdsverify.correlation.delta-fingerprint/v1"
    });
    let fingerprint = sha256_hex(&canonical_json_bytes(&payload)?);
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::schema("internal delta payload is not an object"))?;
    deltas.push(Delta {
        fingerprint,
        path: object["path"].as_str().unwrap_or("/").to_string(),
        kind: serde_json::from_value(object["kind"].clone())
            .map_err(|err| AppError::schema(format!("internal delta kind error: {err}")))?,
        expected: object["expected"].clone(),
        actual: object["actual"].clone(),
    });
    Ok(())
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn validate_dispositions(file: &DispositionFile) -> AppResult<()> {
    if file.schema_version != DISPOSITION_SCHEMA {
        return Err(AppError::schema(format!(
            "dispositions/schema_version must be `{DISPOSITION_SCHEMA}`"
        )));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::schema("system clock is before the Unix epoch"))?
        .as_secs() as i64;
    let mut fingerprints = BTreeSet::new();
    for (index, entry) in file.dispositions.iter().enumerate() {
        let path = format!("dispositions/{index}");
        validate_hash(&entry.fingerprint, &format!("{path}/fingerprint"))?;
        if !fingerprints.insert(entry.fingerprint.as_str()) {
            return Err(AppError::schema(format!(
                "duplicate disposition fingerprint `{}`",
                entry.fingerprint
            )));
        }
        require_text(&entry.owner, &format!("{path}/owner"))?;
        require_text(&entry.reason, &format!("{path}/reason"))?;
        if !entry.scope.starts_with('/')
            || entry.scope == "/"
            || entry.scope.contains('*')
            || entry.scope.contains("..")
        {
            return Err(AppError::schema(format!(
                "{path}/scope must be one exact non-root JSON-pointer path without wildcards"
            )));
        }
        let created = parse_utc_timestamp(&entry.created_at).map_err(|reason| {
            AppError::schema(format!("{path}/created_at is invalid: {reason}"))
        })?;
        let expires = parse_utc_timestamp(&entry.expires_at).map_err(|reason| {
            AppError::schema(format!("{path}/expires_at is invalid: {reason}"))
        })?;
        if created >= expires {
            return Err(AppError::schema(format!(
                "{path} expires_at must be after created_at"
            )));
        }
        if expires <= now {
            return Err(AppError::schema(format!("{path} is expired")));
        }
    }
    Ok(())
}

fn parse_utc_timestamp(value: &str) -> Result<i64, &'static str> {
    // Deliberately accept one canonical representation so lexical aliases cannot
    // change provenance without changing a delta/freeze artifact.
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err("expected YYYY-MM-DDTHH:MM:SSZ");
    }
    let parse = |start: usize, end: usize| -> Result<i64, &'static str> {
        std::str::from_utf8(&bytes[start..end])
            .map_err(|_| "timestamp is not ASCII")?
            .parse::<i64>()
            .map_err(|_| "timestamp contains non-digits")
    };
    let year = parse(0, 4)?;
    let month = parse(5, 7)?;
    let day = parse(8, 10)?;
    let hour = parse(11, 13)?;
    let minute = parse(14, 16)?;
    let second = parse(17, 19)?;
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err("timestamp component is out of range");
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > month_days[(month - 1) as usize] {
        return Err("day is out of range for month");
    }
    let mut days = 0i64;
    for current_year in 1970..year {
        days += if current_year % 4 == 0 && (current_year % 100 != 0 || current_year % 400 == 0) {
            366
        } else {
            365
        };
    }
    for current_month in 1..month {
        days += month_days[(current_month - 1) as usize];
    }
    days += day - 1;
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn build_freeze_manifest(root: &Path, inputs: &[String]) -> AppResult<FreezeManifest> {
    let root = canonical_root(root)?;
    let mut normalized_inputs: Vec<String> = Vec::new();
    let mut entries = BTreeMap::new();
    for input in inputs {
        let relative = normalize_existing_input(&root, input)?;
        if let Some(overlap) = normalized_inputs.iter().find(|existing| {
            path_is_within(&relative, existing) || path_is_within(existing, &relative)
        }) {
            return Err(AppError::schema(format!(
                "overlapping freeze inputs `{overlap}` and `{relative}`"
            )));
        }
        normalized_inputs.push(relative.clone());
        collect_tree(&root, &relative, &mut entries, true)?;
    }
    normalized_inputs.sort();
    Ok(FreezeManifest {
        schema_version: FREEZE_SCHEMA.to_string(),
        inputs: normalized_inputs,
        entries: entries.into_values().collect(),
    })
}

fn canonical_root(root: &Path) -> AppResult<PathBuf> {
    let canonical = fs::canonicalize(root).map_err(|err| {
        AppError::schema(format!(
            "cannot resolve freeze root `{}`: {err}",
            root.display()
        ))
    })?;
    if !canonical.is_dir() {
        return Err(AppError::schema(format!(
            "freeze root `{}` is not a directory",
            root.display()
        )));
    }
    Ok(canonical)
}

fn normalize_existing_input(root: &Path, input: &str) -> AppResult<String> {
    require_text(input, "freeze input")?;
    let supplied = Path::new(input);
    let joined = if supplied.is_absolute() {
        supplied.to_path_buf()
    } else {
        root.join(supplied)
    };
    reject_symlink(&joined)?;
    let canonical = fs::canonicalize(&joined)
        .map_err(|err| AppError::schema(format!("cannot resolve freeze input `{input}`: {err}")))?;
    let relative = canonical.strip_prefix(root).map_err(|_| {
        AppError::schema(format!("freeze input `{input}` escapes the declared root"))
    })?;
    relative_path_string(relative)
}

fn validate_relative_path(path: &str, label: &str) -> AppResult<()> {
    require_text(path, label)?;
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.contains('\\')
        || (path != "." && path.starts_with("./"))
        || path.ends_with('/')
        || path.contains("//")
    {
        return Err(AppError::schema(format!(
            "{label} is not a canonical relative path: `{path}`"
        )));
    }
    if relative_path_string(candidate)? != path {
        return Err(AppError::schema(format!(
            "{label} is not normalized: `{path}`"
        )));
    }
    Ok(())
}

fn relative_path_string(path: &Path) -> AppResult<String> {
    if path.as_os_str().is_empty() {
        return Ok(".".to_string());
    }
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(
                value
                    .to_str()
                    .ok_or_else(|| AppError::schema("freeze paths must be valid UTF-8"))?,
            ),
            Component::CurDir => {}
            _ => {
                return Err(AppError::schema(
                    "freeze path is not relative and normalized",
                ))
            }
        }
    }
    Ok(if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    })
}

fn reject_symlink(path: &Path) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| AppError::schema(format!("cannot inspect `{}`: {err}", path.display())))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::schema(format!(
            "symlinks are not permitted in freeze inputs: `{}`",
            path.display()
        )));
    }
    Ok(())
}

fn collect_tree(
    root: &Path,
    relative: &str,
    entries: &mut BTreeMap<String, FreezeEntry>,
    reject_duplicate: bool,
) -> AppResult<()> {
    validate_relative_path(relative, "freeze path")?;
    let path = if relative == "." {
        root.to_path_buf()
    } else {
        root.join(relative)
    };
    reject_symlink(&path)?;
    let metadata = fs::metadata(&path)
        .map_err(|err| AppError::schema(format!("cannot inspect `{}`: {err}", path.display())))?;
    if metadata.is_dir() {
        let mut children = fs::read_dir(&path)
            .map_err(|err| {
                AppError::schema(format!("cannot read directory `{}`: {err}", path.display()))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                AppError::schema(format!("cannot enumerate `{}`: {err}", path.display()))
            })?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let child_relative = child
                .path()
                .strip_prefix(root)
                .map_err(|_| AppError::schema("enumerated path escaped freeze root"))?
                .to_path_buf();
            collect_tree(
                root,
                &relative_path_string(&child_relative)?,
                entries,
                reject_duplicate,
            )?;
        }
    } else if metadata.is_file() {
        let bytes = fs::read(&path).map_err(|err| {
            AppError::schema(format!(
                "cannot read frozen file `{}`: {err}",
                path.display()
            ))
        })?;
        let entry = FreezeEntry {
            path: relative.to_string(),
            size_bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        };
        if entries.insert(relative.to_string(), entry).is_some() && reject_duplicate {
            return Err(AppError::schema(format!(
                "overlapping freeze inputs include `{relative}` more than once"
            )));
        }
    } else {
        return Err(AppError::schema(format!(
            "freeze input `{}` is neither a regular file nor a directory",
            path.display()
        )));
    }
    Ok(())
}

fn validate_freeze_manifest(manifest: &FreezeManifest) -> AppResult<()> {
    if manifest.schema_version != FREEZE_SCHEMA {
        return Err(AppError::schema(format!(
            "manifest/schema_version must be `{FREEZE_SCHEMA}`"
        )));
    }
    if manifest.inputs.is_empty() {
        return Err(AppError::schema("manifest/inputs must not be empty"));
    }
    let mut previous_input: Option<&str> = None;
    for input in &manifest.inputs {
        validate_relative_path(input, "manifest/input")?;
        if previous_input.is_some_and(|previous| previous >= input.as_str()) {
            return Err(AppError::schema(
                "manifest inputs must be unique and strictly sorted",
            ));
        }
        if manifest
            .inputs
            .iter()
            .any(|other| other != input && path_is_within(input, other))
        {
            return Err(AppError::schema(format!(
                "manifest input `{input}` overlaps another input"
            )));
        }
        previous_input = Some(input);
    }
    let mut previous_entry: Option<&str> = None;
    for entry in &manifest.entries {
        validate_relative_path(&entry.path, "manifest/entry/path")?;
        validate_hash(
            &entry.sha256,
            &format!("manifest/entries/{}/sha256", entry.path),
        )?;
        if previous_entry.is_some_and(|previous| previous >= entry.path.as_str()) {
            return Err(AppError::schema(
                "manifest entries must be unique and strictly sorted",
            ));
        }
        if !manifest
            .inputs
            .iter()
            .any(|input| path_is_within(&entry.path, input))
        {
            return Err(AppError::schema(format!(
                "manifest entry `{}` is outside every declared input",
                entry.path
            )));
        }
        previous_entry = Some(&entry.path);
    }
    Ok(())
}

fn path_is_within(path: &str, input: &str) -> bool {
    input == "."
        || path == input
        || path
            .strip_prefix(input)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn verify_freeze_manifest(root: &Path, manifest: &FreezeManifest) -> AppResult<FreezeReport> {
    let root = canonical_root(root)?;
    let mut current = BTreeMap::new();
    let mut missing_inputs = Vec::new();
    for input in &manifest.inputs {
        let path = if input == "." {
            root.clone()
        } else {
            root.join(input)
        };
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::schema(format!(
                    "symlink replaced frozen input `{input}`"
                )));
            }
            Ok(_) => collect_tree(&root, input, &mut current, false)?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                missing_inputs.push(input.clone());
            }
            Err(err) => {
                return Err(AppError::schema(format!(
                    "cannot inspect frozen input `{input}`: {err}"
                )))
            }
        }
    }
    let expected: BTreeMap<_, _> = manifest
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let paths: BTreeSet<_> = expected
        .keys()
        .copied()
        .chain(current.keys().map(String::as_str))
        .collect();
    let mut deltas = Vec::new();
    for input in missing_inputs {
        deltas.push(FreezeDelta {
            path: input,
            kind: "missing_input".to_string(),
            expected_sha256: None,
            actual_sha256: None,
        });
    }
    for path in paths {
        match (expected.get(path), current.get(path)) {
            (Some(left), Some(right))
                if left.sha256 == right.sha256 && left.size_bytes == right.size_bytes => {}
            (Some(left), Some(right)) => deltas.push(FreezeDelta {
                path: path.to_string(),
                kind: "changed".to_string(),
                expected_sha256: Some(left.sha256.clone()),
                actual_sha256: Some(right.sha256.clone()),
            }),
            (Some(left), None) => deltas.push(FreezeDelta {
                path: path.to_string(),
                kind: "missing".to_string(),
                expected_sha256: Some(left.sha256.clone()),
                actual_sha256: None,
            }),
            (None, Some(right)) => deltas.push(FreezeDelta {
                path: path.to_string(),
                kind: "added".to_string(),
                expected_sha256: None,
                actual_sha256: Some(right.sha256.clone()),
            }),
            (None, None) => unreachable!(),
        }
    }
    deltas.sort_by(|left, right| left.path.cmp(&right.path).then(left.kind.cmp(&right.kind)));
    let manifest_value = serde_json::to_value(manifest)
        .map_err(|err| AppError::schema(format!("cannot serialize freeze manifest: {err}")))?;
    Ok(FreezeReport {
        schema_version: FREEZE_REPORT_SCHEMA.to_string(),
        manifest_sha256: sha256_hex(&canonical_json_bytes(&manifest_value)?),
        deltas,
    })
}

fn summarize_artifact(artifact: &RunArtifact) -> AppResult<ArtifactSummary> {
    let mut status_counts = BTreeMap::new();
    let mut summary = ArtifactSummary {
        schema_version: SUMMARY_SCHEMA.to_string(),
        artifact_identity_sha256: sha256_hex(&canonical_json_bytes(&artifact_identity_value(
            artifact,
        )?)?),
        case_count: artifact.cases.len(),
        drc_marker_count: 0,
        lvs_net_count: 0,
        lvs_device_count: 0,
        lvs_mismatch_count: 0,
        pex_node_count: 0,
        pex_element_count: 0,
        signoff_status_counts: BTreeMap::new(),
    };
    for case in &artifact.cases {
        summary.drc_marker_count += case.drc_markers.len();
        summary.lvs_net_count += case.lvs.nets.len();
        summary.lvs_device_count += case.lvs.devices.len();
        summary.lvs_mismatch_count += case.lvs.mismatches.len();
        summary.pex_node_count += case.pex.nodes.len();
        summary.pex_element_count += case.pex.elements.len();
        for result in &case.signoff {
            let key = match result.status {
                SignoffStatus::Clean => "clean",
                SignoffStatus::Violations => "violations",
                SignoffStatus::NotRun => "not_run",
                SignoffStatus::Error => "error",
            };
            *status_counts.entry(key.to_string()).or_insert(0) += 1;
        }
    }
    summary.signoff_status_counts = status_counts;
    Ok(summary)
}

fn canonical_json_bytes(value: &Value) -> AppResult<Vec<u8>> {
    let mut output = String::new();
    write_canonical_json(value, &mut output)?;
    Ok(output.into_bytes())
}

fn write_canonical_json(value: &Value, output: &mut String) -> AppResult<()> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => output.push_str(
            &serde_json::to_string(value)
                .map_err(|err| AppError::schema(format!("cannot encode JSON string: {err}")))?,
        ),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).map_err(|err| {
                        AppError::schema(format!("cannot encode JSON key: {err}"))
                    })?,
                );
                output.push(':');
                write_canonical_json(&values[*key], output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut state = INITIAL;
    for block in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sigma0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut output = String::with_capacity(64);
    for word in state {
        let _ = write!(output, "{word:08x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "gdsverify-correlation-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> RunArtifact {
        serde_json::from_str(include_str!("../../correlation/fixtures/pex_386af.json"))
            .expect("fixture must parse")
    }

    fn canonical_pair() -> (RunArtifact, RunArtifact) {
        let mut golden = fixture();
        let mut actual = fixture();
        canonicalize_artifact(&mut golden);
        canonicalize_artifact(&mut actual);
        (golden, actual)
    }

    fn element_mut<'a>(artifact: &'a mut RunArtifact, id: &str) -> &'a mut PexElement {
        artifact.cases[0]
            .pex
            .elements
            .iter_mut()
            .find(|element| element.id == id)
            .expect("element exists")
    }

    fn report(golden: &RunArtifact, actual: &RunArtifact, config: &CompareConfig) -> CompareReport {
        compare_artifacts(golden, actual, config, &[]).expect("comparison")
    }

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn exact_match_ignores_only_timestamp() {
        let (golden, mut actual) = canonical_pair();
        actual.metadata.created_at = "2026-07-12T13:00:00Z".to_string();
        let result = report(&golden, &actual, &CompareConfig::default());
        assert!(result.accepted_deltas.is_empty());
        assert!(result.unwaived_deltas.is_empty());
        assert_eq!(result.golden_identity_sha256, result.actual_identity_sha256);
    }

    #[test]
    fn numeric_tolerance_boundary_is_inclusive() {
        let (golden, mut actual) = canonical_pair();
        element_mut(&mut actual, "C1").value.value = 386.5;
        let config = CompareConfig {
            schema_version: CONFIG_SCHEMA.to_string(),
            geometry_tolerance_nm: 0,
            quantities: BTreeMap::from([(
                "capacitance".to_string(),
                NumericTolerance {
                    absolute: 0.5,
                    relative: 0.0,
                },
            )]),
        };
        assert!(report(&golden, &actual, &config).unwaived_deltas.is_empty());
        element_mut(&mut actual, "C1").value.value = 386.500_001;
        assert!(!report(&golden, &actual, &config).unwaived_deltas.is_empty());
    }

    #[test]
    fn unit_mismatch_is_never_hidden_by_numeric_tolerance() {
        let (golden, mut actual) = canonical_pair();
        let element = element_mut(&mut actual, "C1");
        element.value.unit = "fF".to_string();
        for component in &mut element.components {
            component.value.unit = "fF".to_string();
        }
        let config = CompareConfig {
            schema_version: CONFIG_SCHEMA.to_string(),
            geometry_tolerance_nm: 0,
            quantities: BTreeMap::from([(
                "capacitance".to_string(),
                NumericTolerance {
                    absolute: 1.0e9,
                    relative: 1.0,
                },
            )]),
        };
        let result = report(&golden, &actual, &config);
        assert!(result
            .unwaived_deltas
            .iter()
            .any(|delta| delta.kind == DeltaKind::Unit));
    }

    #[test]
    fn topology_mismatch_with_equal_counts_is_detected() {
        let (golden, mut actual) = canonical_pair();
        element_mut(&mut actual, "R1").from = "0".to_string();
        let result = report(&golden, &actual, &CompareConfig::default());
        assert_eq!(
            golden.cases[0].pex.elements.len(),
            actual.cases[0].pex.elements.len()
        );
        assert!(result
            .unwaived_deltas
            .iter()
            .any(|delta| delta.kind == DeltaKind::Topology && delta.path.ends_with("/from")));
    }

    #[test]
    fn marker_shape_mismatch_with_equal_bbox_and_count_is_detected() {
        let (golden, mut actual) = canonical_pair();
        actual.cases[0].drc_markers[0].geometry.rings[0].points = vec![
            Point { x_nm: 50, y_nm: 0 },
            Point {
                x_nm: 100,
                y_nm: 50,
            },
            Point {
                x_nm: 50,
                y_nm: 100,
            },
            Point { x_nm: 0, y_nm: 50 },
        ];
        canonicalize_geometry(&mut actual.cases[0].drc_markers[0].geometry);
        let result = report(&golden, &actual, &CompareConfig::default());
        assert_eq!(
            golden.cases[0].drc_markers.len(),
            actual.cases[0].drc_markers.len()
        );
        assert_eq!(
            golden.cases[0].drc_markers[0].geometry.bbox,
            actual.cases[0].drc_markers[0].geometry.bbox
        );
        assert!(result
            .unwaived_deltas
            .iter()
            .any(|delta| delta.kind == DeltaKind::Geometry));
    }

    #[test]
    fn missing_case_is_detected_by_case_id() {
        let (golden, mut actual) = canonical_pair();
        actual.cases.clear();
        let result = report(&golden, &actual, &CompareConfig::default());
        assert!(result.unwaived_deltas.iter().any(|delta| {
            delta.kind == DeltaKind::Missing && delta.path == "/cases/PEX_386_AF"
        }));
    }

    #[test]
    fn stable_order_and_ring_direction_are_equivalent() {
        let mut golden = fixture();
        let mut actual = fixture();
        let case = &mut actual.cases[0];
        case.lvs.nets.reverse();
        case.pex.nodes.reverse();
        case.pex.elements.reverse();
        case.signoff.reverse();
        case.pex.elements[0].components.reverse();
        case.drc_markers[0].geometry.rings[0].points.rotate_left(2);
        case.drc_markers[0].geometry.rings[0].points.reverse();
        canonicalize_artifact(&mut golden);
        canonicalize_artifact(&mut actual);
        assert!(report(&golden, &actual, &CompareConfig::default())
            .unwaived_deltas
            .is_empty());
    }

    #[test]
    fn exact_future_disposition_accepts_one_delta() {
        let (golden, mut actual) = canonical_pair();
        element_mut(&mut actual, "R1").value.value = 12.5;
        let initial = report(&golden, &actual, &CompareConfig::default());
        assert_eq!(initial.unwaived_deltas.len(), 1);
        let delta = &initial.unwaived_deltas[0];
        let disposition = Disposition {
            fingerprint: delta.fingerprint.clone(),
            owner: "correlation-owner".to_string(),
            reason: "accepted golden-tool rounding difference".to_string(),
            scope: delta.path.clone(),
            created_at: "2026-07-12T00:00:00Z".to_string(),
            expires_at: "2999-01-01T00:00:00Z".to_string(),
            status: DispositionStatus::Approved,
        };
        validate_dispositions(&DispositionFile {
            schema_version: DISPOSITION_SCHEMA.to_string(),
            dispositions: vec![disposition.clone()],
        })
        .expect("valid disposition");
        let accepted =
            compare_artifacts(&golden, &actual, &CompareConfig::default(), &[disposition])
                .expect("comparison");
        assert_eq!(accepted.accepted_deltas.len(), 1);
        assert!(accepted.unwaived_deltas.is_empty());
        assert!(accepted.unused_dispositions.is_empty());
    }

    #[test]
    fn expired_disposition_is_schema_error() {
        let file = DispositionFile {
            schema_version: DISPOSITION_SCHEMA.to_string(),
            dispositions: vec![Disposition {
                fingerprint: "a".repeat(64),
                owner: "owner".to_string(),
                reason: "historical exception".to_string(),
                scope: "/cases/C/pex/elements/R/value/value".to_string(),
                created_at: "2019-01-01T00:00:00Z".to_string(),
                expires_at: "2020-01-01T00:00:00Z".to_string(),
                status: DispositionStatus::Approved,
            }],
        };
        assert!(validate_dispositions(&file).is_err());
    }

    #[test]
    fn mismatched_disposition_does_not_waive_delta() {
        let (golden, mut actual) = canonical_pair();
        element_mut(&mut actual, "R1").value.value = 13.0;
        let disposition = Disposition {
            fingerprint: "a".repeat(64),
            owner: "owner".to_string(),
            reason: "different delta".to_string(),
            scope: "/cases/PEX_386_AF/pex/elements/R1/value/value".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2999-01-01T00:00:00Z".to_string(),
            status: DispositionStatus::Approved,
        };
        let result = compare_artifacts(&golden, &actual, &CompareConfig::default(), &[disposition])
            .expect("comparison");
        assert_eq!(result.unwaived_deltas.len(), 1);
        assert_eq!(result.unused_dispositions, vec!["a".repeat(64)]);
    }

    #[test]
    fn malformed_or_extra_schema_fields_fail_closed() {
        let mut value: Value =
            serde_json::from_str(include_str!("../../correlation/fixtures/pex_386af.json"))
                .expect("fixture JSON");
        value
            .as_object_mut()
            .expect("artifact object")
            .insert("unknown".to_string(), Value::Bool(true));
        assert!(serde_json::from_value::<RunArtifact>(value).is_err());

        let mut duplicate = fixture();
        duplicate.cases.push(duplicate.cases[0].clone());
        assert!(validate_artifact(&duplicate, "duplicate").is_err());
    }

    #[test]
    fn pex_fixture_preserves_386_af_component_breakdown() {
        let artifact = fixture();
        validate_artifact(&artifact, "fixture").expect("fixture validates");
        let element = artifact.cases[0]
            .pex
            .elements
            .iter()
            .find(|element| element.id == "C1")
            .expect("C1");
        assert_eq!(element.value.value, 386.0);
        assert_eq!(element.value.unit, "aF");
        assert_eq!(element.components.len(), 3);
        let components: BTreeMap<_, _> = element
            .components
            .iter()
            .map(|item| (item.mechanism.as_str(), item.value.value))
            .collect();
        assert_eq!(components.get("area_ground"), Some(&(25.0 * 0.4)));
        assert_eq!(components.get("fringe_ground"), Some(&(40.0 * 4.4)));
        assert_eq!(
            components.get("mutual_coupling"),
            Some(&(100.0 * 2.0 * (200.0 / 200.0)))
        );
        assert_eq!(
            element
                .components
                .iter()
                .map(|item| item.value.value)
                .sum::<f64>(),
            386.0
        );
    }

    #[test]
    fn freeze_detects_tamper_add_remove_and_duplicate_manifest_entry() {
        let temp = TestDir::new("freeze");
        let corpus = temp.0.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus");
        fs::write(corpus.join("a.json"), b"alpha").expect("write a");
        fs::write(corpus.join("b.deck"), b"beta").expect("write b");
        let manifest = build_freeze_manifest(&temp.0, &["corpus".to_string()]).expect("freeze");
        validate_freeze_manifest(&manifest).expect("manifest validates");
        assert!(verify_freeze_manifest(&temp.0, &manifest)
            .expect("verify exact")
            .deltas
            .is_empty());

        fs::write(corpus.join("a.json"), b"tampered").expect("tamper");
        let changed = verify_freeze_manifest(&temp.0, &manifest).expect("verify changed");
        assert!(changed.deltas.iter().any(|delta| delta.kind == "changed"));
        fs::write(corpus.join("a.json"), b"alpha").expect("restore");

        fs::write(corpus.join("extra.model"), b"gamma").expect("add");
        let added = verify_freeze_manifest(&temp.0, &manifest).expect("verify added");
        assert!(added.deltas.iter().any(|delta| delta.kind == "added"));
        fs::remove_file(corpus.join("extra.model")).expect("remove extra");

        fs::remove_file(corpus.join("b.deck")).expect("remove b");
        let removed = verify_freeze_manifest(&temp.0, &manifest).expect("verify removed");
        assert!(removed.deltas.iter().any(|delta| delta.kind == "missing"));

        let mut duplicate = manifest.clone();
        duplicate.entries.push(duplicate.entries[0].clone());
        duplicate
            .entries
            .sort_by(|left, right| left.path.cmp(&right.path));
        assert!(validate_freeze_manifest(&duplicate).is_err());

        fs::write(corpus.join("a.json"), b"alpha").expect("restore a");
        assert!(build_freeze_manifest(
            &temp.0,
            &["corpus".to_string(), "corpus/a.json".to_string()]
        )
        .is_err());

        let empty = temp.0.join("empty-corpus");
        fs::create_dir_all(&empty).expect("create empty corpus");
        let empty_manifest =
            build_freeze_manifest(&temp.0, &["empty-corpus".to_string()]).expect("freeze empty");
        fs::remove_dir(&empty).expect("remove empty corpus");
        let missing_input =
            verify_freeze_manifest(&temp.0, &empty_manifest).expect("verify missing empty input");
        assert!(missing_input
            .deltas
            .iter()
            .any(|delta| delta.kind == "missing_input"));
    }

    #[test]
    fn deterministic_report_serialization_is_stable() {
        let (golden, actual) = canonical_pair();
        let first = report(&golden, &actual, &CompareConfig::default());
        let second = report(&golden, &actual, &CompareConfig::default());
        let first_json = serde_json::to_string_pretty(&first).expect("serialize");
        let second_json = serde_json::to_string_pretty(&second).expect("serialize");
        assert_eq!(first_json, second_json);
        assert_eq!(
            sha256_hex(first_json.as_bytes()),
            "5148204ba33076de931ece10f6157f5e429cd722f97d992ec710f72d5eb3f92e"
        );
    }

    #[test]
    fn command_exit_codes_are_distinct() {
        let temp = TestDir::new("exit-codes");
        let golden_path = temp.0.join("golden.json");
        let actual_path = temp.0.join("actual.json");
        let report_path = temp.0.join("report.json");
        let golden = fixture();
        let mut actual = fixture();
        element_mut(&mut actual, "R1").value.value = 14.0;
        fs::write(
            &golden_path,
            serde_json::to_vec(&golden).expect("serialize golden"),
        )
        .expect("write golden");
        fs::write(
            &actual_path,
            serde_json::to_vec(&actual).expect("serialize actual"),
        )
        .expect("write actual");
        assert_eq!(
            run(vec![
                "compare".to_string(),
                golden_path.to_string_lossy().into_owned(),
                actual_path.to_string_lossy().into_owned(),
                "--report".to_string(),
                report_path.to_string_lossy().into_owned(),
            ])
            .expect("comparison command"),
            EXIT_DELTA
        );

        fs::write(&actual_path, b"{").expect("write malformed JSON");
        assert_eq!(
            run(vec![
                "summarize".to_string(),
                actual_path.to_string_lossy().into_owned(),
            ])
            .expect_err("malformed artifact must error")
            .code,
            EXIT_SCHEMA
        );

        let corpus = temp.0.join("corpus");
        fs::create_dir_all(&corpus).expect("create corpus");
        fs::write(corpus.join("deck.json"), b"deck-v1").expect("write deck");
        let manifest = build_freeze_manifest(&temp.0, &["corpus".to_string()]).expect("freeze");
        let manifest_path = temp.0.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(corpus.join("deck.json"), b"deck-v2").expect("tamper deck");
        assert_eq!(
            run(vec![
                "verify-freeze".to_string(),
                manifest_path.to_string_lossy().into_owned(),
                "--root".to_string(),
                temp.0.to_string_lossy().into_owned(),
                "--report".to_string(),
                report_path.to_string_lossy().into_owned(),
            ])
            .expect("freeze verification command"),
            EXIT_FREEZE
        );
        assert_eq!(
            run(vec!["unknown".to_string()])
                .expect_err("unknown command")
                .code,
            EXIT_USAGE
        );
    }
}
