//! Discover and preprocess benchmark circuit fixtures (ALIGN, MAGICAL, TinyTapeout).
//!
//! Repos clone on demand into `benchmarks/fixtures/` and are cleaned up after use.
//! Generic SPICE netlists are preprocessed to PDK-compatible format before parsing.
//!
//! Ported verbatim from `tools/benchmark/src/fixtures.rs` — this machinery is
//! PDK/flow-agnostic (it only produces preprocessed `.spice` text + paths), so it
//! carries over to the new `library`-based flow unchanged.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

// ---------------------------------------------------------------------------
// Repo registry
// ---------------------------------------------------------------------------

/// TinyTapeout fixtures, cloned on demand. The two *competitors* we benchmark
/// against (ALIGN, MAGICAL) are git submodules under `competition/` instead —
/// they are the reference the harness exists for, so they stay checked out and
/// survive `cleanup_fixtures`.
const REPOS: &[(&str, &str, &str)] = &[
    ("ttsky25a-2stageCMOSOpAmp", "https://github.com/anweiteck/ttsky25a-2stageCMOSOpAmp.git", "dac039b949e075582b9571b58111e49af8cdb424"),
    ("tt08-analog-vco", "https://github.com/gbsha/tt08-analog-vco.git", "a4d9c07e88c8b2cb9a38d203a1b11cfc217b1f2e"),
    ("tt08-analog-r2r-dac-3v3", "https://github.com/mattvenn/tt08-analog-r2r-dac-3v3.git", "fa8d779b2d746d47dfbb48536653d4313fc1a3bc"),
    ("TT08", "https://github.com/Sud-ana/TT08.git", "8263d4744b040f021b9e221a463cc260b6d362a3"),
    ("tt08-analog-bias-generator", "https://github.com/rburt16/tt08-analog-bias-generator.git", "b3c38399e9c92180010d6034b0735829713a2300"),
    ("tt08-analog-adc", "https://github.com/J0NTrollston/tt08-analog-adc.git", "afdac9e8c1c12ef6e0a0849d927fb280e171cca0"),
    ("tt08-analog-ring-osc", "https://github.com/mattvenn/tt08-analog-ring-osc.git", "db6f1fc745746f9820eb904a8aa65b75515335c1"),
    ("ttsky-analog-PLL", "https://github.com/jyblue1001/ttsky-analog-PLL.git", "d77f5233a19eea522c2e7934fe2de25bac496c49"),
    ("tt06-sar", "https://github.com/wulffern/tt06-sar.git", "b4ed33a6b0ac1925fa9e1671819e4d4cb6ef5eb0"),
    ("tt09-analog-opamp-3stage", "https://github.com/rburt16/tt09-analog-opamp-3stage.git", "df3bb6ac84c52f175f521de68a4a437ee782b6c0"),
    ("tt10-OTA_FC", "https://github.com/Elettronica-UnivAQ/tt10-OTA_FC.git", "13bb555c60f222155ff55ceb516e1d1df20644d4"),
    ("tt09-analog-tdc", "https://github.com/13hihi31/tt09-analog-tdc.git", "deefa7b3226d3419c4dee03fefa12a90d651935e"),
    ("tt07-12bit_SAR_ADC", "https://github.com/rnunes2311/tt07-12bit_SAR_ADC.git", "8552c5feec6dc6641e0f6fefb52748b432064b98"),
    ("tt08-bgr", "https://github.com/AsalGolmanesh/tt08-bgr.git", "232fc4d2ddb2331bcfaae091cbc3abde3ca17d70"),
];

// ---------------------------------------------------------------------------
// BenchmarkCircuit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BenchmarkCircuit {
    pub name: String,
    pub suite: Suite,
    pub spice_path: PathBuf,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub ref_gds_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    /// Plain .spice/.sp files sitting directly in benchmarks/fixtures/.
    Local,
    Align,
    Magical,
    TinyTapeout,
    All,
}

impl Suite {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            "align" => Self::Align,
            "magical" => Self::Magical,
            "tinytapeout" => Self::TinyTapeout,
            _ => Self::All,
        }
    }

    fn includes(self, target: Suite) -> bool {
        self == Self::All || self == target
    }
}

// ---------------------------------------------------------------------------
// Fixtures root
// ---------------------------------------------------------------------------

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Submodule checkout of a competitor repo, or `None` (with a hint) when the
/// clone was made without `--recurse-submodules`.
fn competition_repo(name: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("competition").join(name);
    if p.is_dir() && fs::read_dir(&p).map_or(false, |mut d| d.next().is_some()) {
        return Some(p);
    }
    eprintln!("competition/{name} is empty — run `git submodule update --init --depth 1`");
    None
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Loose netlists dropped straight into benchmarks/fixtures/ (survive
/// `cleanup_fixtures`, which only removes cloned repos).
pub fn discover_local(root: &Path) -> Vec<BenchmarkCircuit> {
    let Ok(entries) = fs::read_dir(root) else { return vec![] };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .map_or(false, |ext| ext == "spice" || ext == "sp")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| BenchmarkCircuit {
            name: p.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            suite: Suite::Local,
            spice_path: p,
            description: "local fixture".into(),
            ref_gds_path: None,
        })
        .collect()
}

pub fn discover_align() -> Vec<BenchmarkCircuit> {
    let Some(root) = competition_repo("ALIGN") else { return vec![] };
    let align_dir = root.join("examples");
    let Ok(entries) = fs::read_dir(&align_dir) else { return vec![] };
    let mut circuits = Vec::new();
    let mut names: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let sp = align_dir.join(&name).join(format!("{name}.sp"));
        if sp.is_file() {
            circuits.push(BenchmarkCircuit {
                name,
                suite: Suite::Align,
                spice_path: sp,
                description: String::new(),
                ref_gds_path: None,
            });
        }
    }
    circuits
}

pub fn discover_magical() -> Vec<BenchmarkCircuit> {
    let Some(root) = competition_repo("MAGICAL-CIRCUITS") else { return vec![] };
    let magical_dir = root.join("benchmark_circuits");
    let Ok(categories) = fs::read_dir(&magical_dir) else { return vec![] };
    let mut circuits = Vec::new();
    let mut cats: Vec<_> = categories
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    cats.sort();
    for category in cats {
        let cat_dir = magical_dir.join(&category);
        let Ok(files) = fs::read_dir(&cat_dir) else { continue };
        let mut spice_files: Vec<_> = files
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "sp")
            })
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        spice_files.sort();
        for f in spice_files {
            let name = f.strip_suffix(".sp").unwrap_or(&f).to_owned();
            circuits.push(BenchmarkCircuit {
                name,
                suite: Suite::Magical,
                spice_path: cat_dir.join(&f),
                description: category.clone(),
                ref_gds_path: None,
            });
        }
    }
    circuits
}

pub fn discover_tinytapeout(root: &Path) -> Vec<BenchmarkCircuit> {
    if !root.is_dir() {
        return vec![];
    }
    let Ok(entries) = fs::read_dir(root) else { return vec![] };
    let mut circuits = Vec::new();
    let mut names: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            e.path().is_dir() && (n.starts_with("tt") || n.starts_with("TT"))
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    let search_patterns: &[(&str, &str)] = &[
        ("xschem", "*.spice"),
        ("mag", "*_xschem.spice"),
        ("mag", "*.spice"),
        ("spi", "*.spice"),
    ];

    for name in names {
        let d = root.join(&name);
        let mut sp = None;
        for &(subdir, _ext) in search_patterns {
            let pattern = format!("{}/**/*.spice", d.display());
            let candidates = find_spice_recursive(&d, subdir);
            if let Some(c) = candidates.first() {
                sp = Some(c.clone());
                break;
            }
            let _ = pattern;
        }
        let Some(spice_path) = sp else { continue };

        let gds_dir = d.join("gds");
        let ref_gds = fs::read_dir(&gds_dir)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| {
                        p.file_name()
                            .map_or(false, |n| {
                                let s = n.to_string_lossy();
                                s.starts_with("tt_") && s.ends_with(".gds")
                            })
                    })
            });

        circuits.push(BenchmarkCircuit {
            name,
            suite: Suite::TinyTapeout,
            spice_path,
            description: "TinyTapeout".into(),
            ref_gds_path: ref_gds,
        });
    }
    circuits
}

fn find_spice_recursive(base: &Path, subdir: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_dir_recursive(base, &mut |path| {
        let path_str = path.to_string_lossy();
        if !path_str.contains(&format!("/{subdir}/")) {
            return;
        }
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !fname.ends_with(".spice") {
            return;
        }
        if path_str.contains("pex") || fname.contains("sim") || fname.contains("lvs") {
            return;
        }
        results.push(path.to_path_buf());
    });
    results.sort();
    results
}

fn walk_dir_recursive(dir: &Path, cb: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, cb);
        } else {
            cb(&path);
        }
    }
}

pub fn discover_all(suite: Suite) -> Vec<BenchmarkCircuit> {
    let root = fixtures_dir();
    let mut circuits = Vec::new();
    if suite.includes(Suite::Local) {
        circuits.extend(discover_local(&root));
    }
    if suite.includes(Suite::Align) {
        circuits.extend(discover_align());
    }
    if suite.includes(Suite::Magical) {
        circuits.extend(discover_magical());
    }
    if suite.includes(Suite::TinyTapeout) {
        circuits.extend(discover_tinytapeout(&root));
    }
    circuits
}

// ---------------------------------------------------------------------------
// Repo management
// ---------------------------------------------------------------------------

pub fn clone_repos_if_needed(suite: Suite) -> std::io::Result<()> {
    let root = fixtures_dir();
    fs::create_dir_all(&root)?;
    // Local fixtures need no clones; ALIGN/MAGICAL come from `competition/` submodules.
    if matches!(suite, Suite::Local | Suite::Align | Suite::Magical) {
        return Ok(());
    }
    for &(name, url, revision) in REPOS {
        let dest = root.join(name);
        let created = !dest.exists();
        if created {
            eprintln!("Fetching pinned fixture {name}@{}...", &revision[..12]);
            fs::create_dir_all(&dest)?;
            if let Err(error) = checkout_revision(&dest, name, url, revision, true) {
                let _ = fs::remove_dir_all(&dest);
                return Err(error);
            }
        } else if !dest.join(".git").exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("fixture path {} exists but is not a git repository", dest.display()),
            ));
        } else if current_revision(&dest)? != revision {
            eprintln!("Updating fixture {name} to pinned revision {}...", &revision[..12]);
            checkout_revision(&dest, name, url, revision, false)?;
        }
    }
    Ok(())
}

fn current_revision(repo: &Path) -> std::io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("cannot read fixture revision in {}", repo.display()),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn run_git(repo: &Path, fixture: &str, args: &[&str]) -> std::io::Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("git {} failed for {fixture}", args.first().copied().unwrap_or("command")),
        ))
    }
}

fn checkout_revision(
    repo: &Path,
    fixture: &str,
    url: &str,
    revision: &str,
    initialize: bool,
) -> std::io::Result<()> {
    if initialize {
        run_git(repo, fixture, &["init", "--quiet"])?;
        run_git(repo, fixture, &["remote", "add", "origin", url])?;
    }
    run_git(
        repo,
        fixture,
        &["fetch", "--quiet", "--depth", "1", "origin", revision],
    )?;
    run_git(repo, fixture, &["checkout", "--quiet", "--detach", revision])?;
    if current_revision(repo)? != revision {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("fixture {fixture} did not resolve to pinned revision {revision}"),
        ));
    }
    Ok(())
}

pub fn cleanup_fixtures() {
    let root = fixtures_dir();
    let Ok(entries) = fs::read_dir(&root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(".git").is_dir() {
            if fs::remove_dir_all(&path).is_ok() {
                eprintln!("Removed {}", entry.file_name().to_string_lossy());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SPICE preprocessing: generic netlists → PDK-compatible format
// ---------------------------------------------------------------------------

/// MIM cap density per PDK (fF/µm²), matched by PDK name prefix.
const CAP_DENSITY: &[(&str, f64)] = &[
    ("sky130", 2.0),
    ("ihp", 1.5),
    ("gf180", 1.0),
];

/// FinFET nfin → planar W mapping (µm per fin).
const UM_PER_FIN: f64 = 0.1;

const SI_SUFFIXES: &[(&str, f64)] = &[
    ("meg", 1e6),
    ("t", 1e12),
    ("g", 1e9),
    ("k", 1e3),
    ("m", 1e-3),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

fn parse_si(s: &str) -> f64 {
    let s = s.trim().to_ascii_lowercase();
    if s.contains('e') && s.as_bytes().last().map_or(false, |b| b.is_ascii_digit()) {
        if let Ok(v) = s.parse::<f64>() {
            return v;
        }
    }
    for &(suffix, mult) in SI_SUFFIXES {
        if let Some(prefix) = s.strip_suffix(suffix) {
            if let Ok(v) = prefix.parse::<f64>() {
                return v * mult;
            }
        }
    }
    s.parse().unwrap_or(0.0)
}

fn is_numeric(s: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    let s = s.strip_prefix('+').or_else(|| s.strip_prefix('-')).unwrap_or(&s);
    if s.is_empty() {
        return false;
    }
    let base = SI_SUFFIXES
        .iter()
        .find_map(|&(suf, _)| s.strip_suffix(suf))
        .unwrap_or(s);
    if base.is_empty() {
        return false;
    }
    base.parse::<f64>().is_ok()
}

fn resolve_param<'a>(s: &'a str, params: &'a HashMap<String, String>) -> String {
    let mut val = s.to_ascii_lowercase();
    let mut seen = std::collections::HashSet::new();
    while let Some(next) = params.get(&val) {
        if !seen.insert(val.clone()) {
            break;
        }
        val = next.to_ascii_lowercase();
    }
    val
}

fn collect_spice_params(text: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim().to_ascii_lowercase();
        if !trimmed.starts_with(".param") {
            continue;
        }
        for tok in line.split_whitespace().skip(1) {
            if let Some((k, v)) = tok.split_once('=') {
                let v = v.trim_end_matches(',');
                params.insert(
                    k.trim().to_ascii_lowercase(),
                    v.trim().to_owned(),
                );
            }
        }
    }
    params
}

fn join_backslash(text: &str) -> String {
    let mut joined = Vec::new();
    for line in text.split('\n') {
        if let Some(last) = joined.last_mut() {
            let s: &mut String = last;
            if s.ends_with('\\') {
                s.pop();
                while s.ends_with(' ') {
                    s.pop();
                }
                s.push(' ');
                s.push_str(line.trim_start());
                continue;
            }
        }
        joined.push(line.to_owned());
    }
    joined.join("\n")
}

fn fmt_um(val_um: f64) -> String {
    format!("{:.4}", val_um)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
        + "u"
}

/// PDK data extracted for SPICE preprocessing.
struct PdkPreprocess {
    #[allow(dead_code)]
    name: String,
    cap_model: Option<String>,
    res_model: Option<String>,
    cap_density: f64,
    r_sheet: f64,
    res_w: f64,
}

impl PdkPreprocess {
    /// `name` is the deck's file stem (the rebuilt decks carry no `name` key);
    /// it only feeds the [`CAP_DENSITY`] prefix match.
    fn from_json(pdk: &Value, name: &str) -> Self {
        // Rebuilt deck schema: `device_recognition` is an array of
        // `{kind, marker, model, terminals}`.
        let recogniser = |kind: &str| -> Option<&Value> {
            pdk["device_recognition"]
                .as_array()?
                .iter()
                .find(|d| d["kind"].as_str() == Some(kind))
        };

        let res = recogniser("resistor");
        let res_model = res.and_then(|d| d["model"].as_str()).map(String::from);
        // Neither deck defines a capacitor recogniser — `library::parse`
        // classifies X-instances by model substring and `cells::capacitor`
        // draws them, so a bare "cap" model word is all a rewrite needs.
        let cap_model = Some("cap".to_owned());

        let cap_density = {
            let low = name.to_ascii_lowercase();
            CAP_DENSITY
                .iter()
                .find(|&&(prefix, _)| low.contains(prefix))
                .map_or(1.0, |&(_, d)| d)
        };

        // Sheet resistance of the resistor's marker layer, straight from the
        // deck's per-layer pex block (ohm/sq); poly as fallback.
        let r_sheet = res
            .and_then(|d| d["marker"].as_str())
            .and_then(|body| pdk["pex"][body]["sheet_res_ohm_sq"].as_f64())
            .or_else(|| pdk["pex"]["poly"]["sheet_res_ohm_sq"].as_f64())
            .filter(|&v| v > 0.0)
            .unwrap_or(48.2);
        // ponytail: fixed 0.33 µm body width — the rebuilt decks carry no
        // per-device default; derive from the marker layer's min_width rule if
        // resistor sizing ever matters to a benchmark.
        let res_w = 0.33;

        Self { name: name.to_owned(), cap_model, res_model, cap_density, r_sheet, res_w }
    }
}

/// Rewrite generic SPICE into PDK-compatible format.
///
/// - Backslash continuation joining
/// - Bare caps/resistors with real W/L from cap density / sheet-R
/// - Bare R/C with .param value references resolved
/// - FinFET nfin→W synthesis when W is absent on MOSFET lines
pub fn preprocess_spice(text: &str, pdk_path: &Path) -> Result<String, String> {
    let text = join_backslash(text);
    let pdk_json: Value =
        serde_json::from_str(&fs::read_to_string(pdk_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let deck_name = pdk_path.file_stem().unwrap_or_default().to_string_lossy();
    let pdk = PdkPreprocess::from_json(&pdk_json, &deck_name);
    let spice_params = collect_spice_params(&text);

    let mut out = Vec::new();
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty()
            || stripped.starts_with('*')
            || stripped.starts_with('.')
            || stripped.starts_with('+')
        {
            out.push(line.to_owned());
            continue;
        }

        let tokens: Vec<&str> = stripped.split_whitespace().collect();
        if tokens.len() < 3 {
            out.push(line.to_owned());
            continue;
        }

        let first = tokens[0].as_bytes()[0].to_ascii_lowercase();

        // --- nfin→W synthesis for MOSFET lines ---
        if first == b'm' || first == b'x' {
            let kv: HashMap<String, &str> = tokens
                .iter()
                .filter_map(|t| t.split_once('='))
                .map(|(k, v)| (k.to_ascii_lowercase(), v))
                .collect();
            let mut extra = String::new();
            if !kv.contains_key("w")
                && (kv.contains_key("nfin") || kv.contains_key("nf"))
            {
                let nfin_raw = kv.get("nfin").or_else(|| kv.get("nf")).unwrap_or(&"1");
                let resolved = resolve_param(nfin_raw, &spice_params);
                let nfin_val = parse_si(&resolved).max(1.0);
                extra.push_str(&format!(" w={}", fmt_um(nfin_val * UM_PER_FIN)));
            }
            if !kv.contains_key("l") {
                extra.push_str(" l=0.15u");
            }
            if !extra.is_empty() {
                out.push(format!("{stripped}{extra}"));
                continue;
            }
            out.push(line.to_owned());
            continue;
        }

        // --- bare C/R rewriting ---
        if first != b'c' && first != b'r' {
            out.push(line.to_owned());
            continue;
        }

        let kv_start = tokens[1..]
            .iter()
            .position(|t| t.contains('='))
            .map(|i| i + 1)
            .unwrap_or(tokens.len());
        let positional = &tokens[1..kv_start];

        if positional.len() < 2 {
            out.push(line.to_owned());
            continue;
        }

        let kv: HashMap<String, String> = tokens[kv_start..]
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| {
                (
                    k.to_ascii_lowercase(),
                    resolve_param(v, &spice_params),
                )
            })
            .collect();
        let last_pos = positional.last().unwrap();
        let last_lower = last_pos.to_ascii_lowercase();
        let generic_model = matches!(
            last_lower.as_str(),
            "resistor" | "res" | "capacitor" | "cap"
        );
        let resolved = resolve_param(last_pos, &spice_params);

        let (nodes, value) = if is_numeric(&resolved) {
            (&positional[..positional.len() - 1], parse_si(&resolved))
        } else if generic_model {
            if first == b'c' && kv.contains_key("w") && kv.contains_key("l") {
                if let Some(ref cap_model) = pdk.cap_model {
                    let inst = tokens[0];
                    let new_inst = if inst.to_ascii_uppercase().starts_with('X') {
                        inst.to_owned()
                    } else {
                        format!("X{inst}")
                    };
                    let kvs: Vec<String> = tokens[kv_start..]
                        .iter()
                        .filter_map(|t| t.split_once('='))
                        .map(|(k, v)| {
                            format!(
                                "{}={}",
                                k.to_ascii_lowercase(),
                                resolve_param(v, &spice_params)
                            )
                        })
                        .collect();
                    out.push(format!(
                        "{new_inst} {} {cap_model} {}",
                        positional[..positional.len() - 1].join(" "),
                        kvs.join(" "),
                    ));
                    continue;
                }
            }
            let val_kv = if first == b'r' { kv.get("r") } else { kv.get("c") };
            match val_kv.filter(|v| is_numeric(v)) {
                Some(v) => (&positional[..positional.len() - 1], parse_si(v)),
                None => {
                    out.push(line.to_owned());
                    continue;
                }
            }
        } else {
            out.push(line.to_owned());
            continue;
        };
        let kv_params: Vec<String> = tokens[kv_start..]
            .iter()
            .filter_map(|t| t.split_once('='))
            .filter(|(k, _)| {
                let k = k.to_ascii_lowercase();
                k != "r" && k != "c"
            })
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    k.to_ascii_lowercase(),
                    resolve_param(v, &spice_params)
                )
            })
            .collect();
        let inst = tokens[0];

        if first == b'c' {
            if let Some(ref cap_model) = pdk.cap_model {
                if value != 0.0 {
                    let c_ff = value.abs() * 1e15;
                    let area_um2 = c_ff / pdk.cap_density;
                    let side = area_um2.sqrt().max(0.5);
                    let new_inst = if inst.to_ascii_uppercase().starts_with('X') {
                        inst.to_owned()
                    } else {
                        format!("X{inst}")
                    };
                    let new_line = format!(
                        "{new_inst} {} {cap_model} W={} L={} {}",
                        nodes.join(" "),
                        fmt_um(side),
                        fmt_um(side),
                        kv_params.join(" "),
                    );
                    out.push(new_line.trim_end().to_owned());
                    continue;
                }
            }
        } else if first == b'r' {
            if let Some(ref res_model) = pdk.res_model {
                if value != 0.0 {
                    let r_val = value.abs();
                    let w = pdk.res_w;
                    let l = (r_val * w / pdk.r_sheet).max(w);
                    let new_inst = if inst.to_ascii_uppercase().starts_with('X') {
                        inst.to_owned()
                    } else {
                        format!("X{inst}")
                    };
                    let new_line = format!(
                        "{new_inst} {} {res_model} W={} L={} {}",
                        nodes.join(" "),
                        fmt_um(w),
                        fmt_um(l),
                        kv_params.join(" "),
                    );
                    out.push(new_line.trim_end().to_owned());
                    continue;
                }
            }
        }

        out.push(line.to_owned());
    }

    let mut result = out.join("\n");
    result.push('\n');
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_si_values() {
        assert!((parse_si("10k") - 10_000.0).abs() < 1e-6);
        assert!((parse_si("2.5u") - 2.5e-6).abs() < 1e-15);
        assert!((parse_si("1.2e3") - 1200.0).abs() < 1e-6);
        assert!((parse_si("100meg") - 1e8).abs() < 1e-6);
        assert!((parse_si("3.3p") - 3.3e-12).abs() < 1e-21);
        assert_eq!(parse_si("garbage"), 0.0);
    }

    #[test]
    fn fmt_um_no_double_suffix() {
        assert_eq!(fmt_um(6.9282), "6.9282u");
        assert_eq!(fmt_um(2.5000), "2.5u");
        assert_eq!(fmt_um(10.0), "10u");
        assert_eq!(fmt_um(0.33), "0.33u");
    }

    #[test]
    fn generic_model_word_rc_rewrites() {
        let pdk_json = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("pdks/generic_finfet.json");
        let text = ".param rl=500 Csw=5u\n\
                    R1 vps vout resistor r=rl\n\
                    C4 a b capacitor w=Csw l=Csw\n";
        let out = preprocess_spice(text, &pdk_json).expect("preprocess");
        let r_line = out.lines().find(|l| l.contains("vps vout")).unwrap();
        assert!(r_line.starts_with("XR1 "), "resistor instance: {r_line}");
        assert!(r_line.contains("W=") && r_line.contains("L="), "{r_line}");
        assert!(!r_line.to_ascii_lowercase().contains("r=rl"), "{r_line}");
        let c_line = out.lines().find(|l| l.contains("a b")).unwrap();
        assert!(c_line.starts_with("XC4 "), "cap instance: {c_line}");
        assert!(c_line.contains("w=5u") && c_line.contains("l=5u"), "{c_line}");
        assert!(!c_line.contains("capacitor"), "{c_line}");
    }

    #[test]
    fn is_numeric_checks() {
        assert!(is_numeric("10k"));
        assert!(is_numeric("2.5u"));
        assert!(is_numeric("1.2e3"));
        assert!(is_numeric("-3.3p"));
        assert!(!is_numeric("vdd"));
        assert!(!is_numeric(""));
    }

    #[test]
    fn backslash_join() {
        let input = "line1 \\\n  continued\nline2";
        assert_eq!(join_backslash(input), "line1 continued\nline2");
    }

    #[test]
    fn param_resolution() {
        let mut params = HashMap::new();
        params.insert("wn".into(), "0.5u".into());
        params.insert("wp".into(), "wn".into());
        assert_eq!(resolve_param("wp", &params), "0.5u");
    }

    #[test]
    fn spice_param_collection() {
        let text = ".param wn=0.5u wp=1u\n.param rval=10k";
        let p = collect_spice_params(text);
        assert_eq!(p["wn"], "0.5u");
        assert_eq!(p["rval"], "10k");
    }

    #[test]
    fn suite_filter() {
        assert!(Suite::All.includes(Suite::Align));
        assert!(Suite::Align.includes(Suite::Align));
        assert!(!Suite::Align.includes(Suite::Magical));
    }

    #[test]
    fn fixture_revisions_are_immutable_commit_ids() {
        let mut names = std::collections::HashSet::new();
        for &(name, _, revision) in REPOS {
            assert!(names.insert(name), "duplicate fixture {name}");
            assert_eq!(revision.len(), 40, "unpinned fixture {name}");
            assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }
}
