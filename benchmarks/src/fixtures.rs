//! Discover and preprocess benchmark circuit fixtures (ALIGN, MAGICAL, TinyTapeout).
//!
//! TinyTapeout repos clone on demand into `benchmarks/fixtures/` at pinned
//! revisions and are cleaned up after use; ALIGN and MAGICAL are git
//! submodules under `benchmarks/competition/`. Generic SPICE netlists are
//! preprocessed to a PDK-compatible format before parsing.
//!
//! This machinery is PDK/flow-agnostic: it only produces preprocessed
//! `.spice` text and paths.

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

/// One discovered fixture netlist.
#[derive(Debug, Clone)]
pub struct BenchmarkCircuit {
    /// Display and artifact name: the file stem (local, MAGICAL), the example
    /// directory (ALIGN) or the repo directory (TinyTapeout). Unique within a
    /// suite.
    pub name: String,
    /// The suite it was discovered in (never [`Suite::All`]).
    pub suite: Suite,
    /// The raw (unpreprocessed) netlist.
    pub spice_path: PathBuf,
}

/// A fixture family; [`Suite::All`] selects every one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    /// Plain .spice/.sp files sitting directly in benchmarks/fixtures/.
    Local,
    /// `competition/ALIGN/examples/<name>/<name>.sp`.
    Align,
    /// `competition/MAGICAL-CIRCUITS/benchmark_circuits/<category>/*.sp`.
    Magical,
    /// The pinned [`REPOS`] under `benchmarks/fixtures/`.
    TinyTapeout,
    /// Every suite above.
    All,
}

impl Suite {
    /// Parses a suite name, case-insensitively. Anything unrecognised is
    /// [`Suite::All`].
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            "align" => Self::Align,
            "magical" => Self::Magical,
            "tinytapeout" => Self::TinyTapeout,
            _ => Self::All,
        }
    }

    /// Whether selecting `self` runs the fixtures of `target`.
    fn includes(self, target: Suite) -> bool {
        self == Self::All || self == target
    }
}

// ---------------------------------------------------------------------------
// Fixtures root
// ---------------------------------------------------------------------------

/// `benchmarks/fixtures/`: local netlists and the TinyTapeout clones.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Submodule checkout of a competitor repo, or `None` (with a hint) when the
/// clone was made without `--recurse-submodules`.
fn competition_repo(name: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("competition").join(name);
    if fs::read_dir(&p).is_ok_and(|mut d| d.next().is_some()) {
        return Some(p);
    }
    eprintln!("competition/{name} is empty — run `git submodule update --init --depth 1`");
    None
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// The entries of `dir` that `keep` accepts, sorted by path. Unreadable
/// directories and entries are skipped.
fn sorted_entries(dir: &Path, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| keep(p)).collect();
    paths.sort();
    paths
}

/// `p`'s file name, lossily as UTF-8 (empty for a path without one).
fn file_name(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().into_owned()
}

/// Whether `p` is a file with one of the extensions `exts`.
fn has_extension(p: &Path, exts: &[&str]) -> bool {
    p.is_file() && p.extension().is_some_and(|e| exts.iter().any(|x| e == *x))
}

/// Loose `.spice`/`.sp` netlists directly in `root`, sorted by path (they
/// survive `cleanup_fixtures`, which only removes cloned repos).
pub fn discover_local(root: &Path) -> Vec<BenchmarkCircuit> {
    sorted_entries(root, |p| has_extension(p, &["spice", "sp"]))
        .into_iter()
        .map(|p| BenchmarkCircuit {
            name: p.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            suite: Suite::Local,
            spice_path: p,
        })
        .collect()
}

/// ALIGN examples: every `examples/<name>/<name>.sp`, sorted by name. Empty
/// when the submodule is not checked out.
pub fn discover_align() -> Vec<BenchmarkCircuit> {
    let Some(root) = competition_repo("ALIGN") else { return vec![] };
    sorted_entries(&root.join("examples"), Path::is_dir)
        .into_iter()
        .filter_map(|dir| {
            let name = file_name(&dir);
            let sp = dir.join(format!("{name}.sp"));
            sp.is_file().then_some(BenchmarkCircuit { name, suite: Suite::Align, spice_path: sp })
        })
        .collect()
}

/// MAGICAL benchmark circuits: every `*.sp` in each category directory,
/// sorted by category then file. Empty when the submodule is not checked out.
pub fn discover_magical() -> Vec<BenchmarkCircuit> {
    let Some(root) = competition_repo("MAGICAL-CIRCUITS") else { return vec![] };
    sorted_entries(&root.join("benchmark_circuits"), Path::is_dir)
        .into_iter()
        .flat_map(|cat| sorted_entries(&cat, |p| has_extension(p, &["sp"])))
        .map(|sp| BenchmarkCircuit {
            name: sp.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            suite: Suite::Magical,
            spice_path: sp,
        })
        .collect()
}

/// Subdirectories a TinyTapeout repo keeps its schematic netlist in, in
/// order of preference.
const TT_NETLIST_DIRS: &[&str] = &["xschem", "mag", "spi"];

/// TinyTapeout repos: every `tt*` directory (case-insensitive) under `root`,
/// sorted, with its first netlist from the first [`TT_NETLIST_DIRS`] entry
/// that has one ([`find_spice_recursive`]). A repo with none is skipped.
pub fn discover_tinytapeout(root: &Path) -> Vec<BenchmarkCircuit> {
    sorted_entries(root, |p| p.is_dir() && file_name(p).to_ascii_lowercase().starts_with("tt"))
        .into_iter()
        .filter_map(|d| {
            let spice_path = TT_NETLIST_DIRS.iter().find_map(|sub| find_spice_recursive(&d, sub).into_iter().next())?;
            Some(BenchmarkCircuit { name: file_name(&d), suite: Suite::TinyTapeout, spice_path })
        })
        .collect()
}

/// Every `.spice` file under `base` that sits below a directory named
/// `subdir`, sorted, excluding extracted (`pex` anywhere in the path),
/// simulation (`sim` in the name) and LVS (`lvs` in the name) netlists.
fn find_spice_recursive(base: &Path, subdir: &str) -> Vec<PathBuf> {
    let marker = format!("/{subdir}/");
    let mut results = Vec::new();
    walk_dir_recursive(base, &mut |path| {
        let path_str = path.to_string_lossy();
        let fname = file_name(path);
        if path_str.contains(&marker)
            && fname.ends_with(".spice")
            && !path_str.contains("pex")
            && !fname.contains("sim")
            && !fname.contains("lvs")
        {
            results.push(path.to_path_buf());
        }
    });
    results.sort();
    results
}

/// Calls `cb` on every non-directory entry below `dir`, depth first.
/// Unreadable directories are skipped.
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

/// Every circuit of `suite`, in suite order: local, ALIGN, MAGICAL,
/// TinyTapeout.
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

/// Makes every [`REPOS`] fixture present at its pinned revision (only for
/// suites that include TinyTapeout): a missing one is fetched shallowly, one
/// at another revision is moved to the pin.
///
/// # Errors
/// `benchmarks/fixtures/` cannot be created, a fixture path exists but is not
/// a git repository, or git fails or resolves to another revision. A failed
/// first fetch removes its partial directory.
pub fn clone_repos_if_needed(suite: Suite) -> std::io::Result<()> {
    let root = fixtures_dir();
    fs::create_dir_all(&root)?;
    // Local fixtures need no clones; ALIGN/MAGICAL come from `competition/` submodules.
    if !suite.includes(Suite::TinyTapeout) {
        return Ok(());
    }
    for &(name, url, revision) in REPOS {
        let dest = root.join(name);
        if !dest.exists() {
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

/// `HEAD`'s commit id in `repo`.
///
/// # Errors
/// git cannot run or `rev-parse` fails.
fn current_revision(repo: &Path) -> std::io::Result<String> {
    let output = Command::new("git").arg("-C").arg(repo).args(["rev-parse", "HEAD"]).output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("cannot read fixture revision in {}", repo.display()),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Runs `git -C repo <args>`.
///
/// # Errors
/// git cannot run or exits non-zero (`fixture` names it in the message).
fn run_git(repo: &Path, fixture: &str, args: &[&str]) -> std::io::Result<()> {
    let status = Command::new("git").arg("-C").arg(repo).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "git {} failed for {fixture}",
            args.first().copied().unwrap_or("command")
        )))
    }
}

/// Fetches `revision` shallowly from `url` into `repo` (first `git init` and
/// adding the remote when `initialize`) and checks it out detached.
///
/// # Errors
/// A git step fails, or `HEAD` does not end at `revision`.
fn checkout_revision(repo: &Path, fixture: &str, url: &str, revision: &str, initialize: bool) -> std::io::Result<()> {
    if initialize {
        run_git(repo, fixture, &["init", "--quiet"])?;
        run_git(repo, fixture, &["remote", "add", "origin", url])?;
    }
    run_git(repo, fixture, &["fetch", "--quiet", "--depth", "1", "origin", revision])?;
    run_git(repo, fixture, &["checkout", "--quiet", "--detach", revision])?;
    if current_revision(repo)? != revision {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("fixture {fixture} did not resolve to pinned revision {revision}"),
        ));
    }
    Ok(())
}

/// Removes every git checkout directly under `benchmarks/fixtures/` (the
/// cloned repos); loose netlists stay. Best effort.
pub fn cleanup_fixtures() {
    let Ok(entries) = fs::read_dir(fixtures_dir()) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.join(".git").is_dir() && fs::remove_dir_all(&path).is_ok() {
            eprintln!("Removed {}", entry.file_name().to_string_lossy());
        }
    }
}

// ---------------------------------------------------------------------------
// SPICE preprocessing: generic netlists → PDK-compatible format
// ---------------------------------------------------------------------------

/// FinFET nfin → planar W mapping (µm per fin), for a deck without a fin
/// pitch.
const UM_PER_FIN: f64 = 0.1;

/// Channel length given to a MOS card that states none, µm.
const DEFAULT_L: &str = "0.15u";

/// Model word a rewritten capacitor card uses. Neither deck defines a
/// capacitor recogniser — `library::parse` classifies X-instances by model
/// token (`cap`) and `cells::capacitor` draws them, so a bare "cap" model
/// word is all a rewrite needs.
const CAP_MODEL: &str = "cap";

/// SPICE scale suffixes (case-insensitive), `meg` before `m` so it wins.
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

/// Parses a SPICE number (`10k`, `2.5u`, `1.2e3`, `100meg`). Anything
/// unparseable is 0.
fn parse_si(s: &str) -> f64 {
    // Non-finite (`inf`, `1e999`) is no SPICE value: 0 like any other junk.
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let s = s.trim().to_ascii_lowercase();
    if s.contains('e') && s.as_bytes().last().is_some_and(u8::is_ascii_digit) {
        if let Ok(v) = s.parse::<f64>() {
            return finite(v);
        }
    }
    for &(suffix, mult) in SI_SUFFIXES {
        if let Some(prefix) = s.strip_suffix(suffix) {
            if let Ok(v) = prefix.parse::<f64>() {
                return finite(v * mult);
            }
        }
    }
    s.parse().map_or(0.0, finite)
}

/// Whether `s` is a SPICE number: optional sign, a float, optional
/// [`SI_SUFFIXES`] scale.
fn is_numeric(s: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    let s = s.strip_prefix('+').or_else(|| s.strip_prefix('-')).unwrap_or(&s);
    if s.is_empty() {
        return false;
    }
    let base = SI_SUFFIXES.iter().find_map(|&(suf, _)| s.strip_suffix(suf)).unwrap_or(s);
    !base.is_empty() && base.parse::<f64>().is_ok_and(f64::is_finite)
}

/// Follows `.param` references from `s` (lowercased) until a word that is
/// not a parameter, stopping on a cycle. The result is lowercase.
fn resolve_param(s: &str, params: &HashMap<String, String>) -> String {
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

/// Every `name=value` of every `.param` line, names lowercased, a trailing
/// comma dropped from values; a later definition wins.
fn collect_spice_params(text: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for line in text.lines().filter(|l| l.trim().to_ascii_lowercase().starts_with(".param")) {
        for (k, v) in line.split_whitespace().skip(1).filter_map(|t| t.split_once('=')) {
            params.insert(k.trim().to_ascii_lowercase(), v.trim_end_matches(',').trim().to_owned());
        }
    }
    params
}

/// Joins each line ending in `\` with the next, the backslash and the
/// spaces around the join collapsed to one space.
fn join_backslash(text: &str) -> String {
    let mut joined: Vec<String> = Vec::new();
    for line in text.split('\n') {
        // A CRLF file's continuation ends in a backslash then a carriage return.
        if let Some(s) = joined.last_mut().filter(|s| s.trim_end_matches('\r').ends_with('\\')) {
            s.truncate(s.trim_end_matches('\r').len() - 1);
            s.truncate(s.trim_end_matches(' ').len());
            s.push(' ');
            s.push_str(line.trim_start());
        } else {
            joined.push(line.to_owned());
        }
    }
    joined.join("\n")
}

/// Formats µm with at most 4 decimals, trailing zeros dropped, and a `u`
/// suffix (`2.5` → `2.5u`, `10.0` → `10u`).
fn fmt_um(val_um: f64) -> String {
    format!("{val_um:.4}").trim_end_matches('0').trim_end_matches('.').to_owned() + "u"
}

/// The deck data the R/C and nfin rewrites need.
struct Rewrite {
    /// Resistor model word, `None` when the deck cannot size one (no default
    /// recipe, no body sheet resistance or no min width): R cards stay as
    /// written.
    res_model: Option<String>,
    /// Capacitance density, fF/µm² (> 0).
    cap_density: f64,
    /// Resistor body sheet resistance, Ω/□.
    r_sheet: f64,
    /// Resistor body minimum width, µm.
    res_w: f64,
    /// W per fin, µm.
    um_per_fin: f64,
}

impl Rewrite {
    /// The resistor is the sidecar's default recipe, sized by its body's
    /// deck sheet resistance and min width; no recipe (or no sheet R), no
    /// rewrite. Cap density is the sidecar's `cap_density_ff_um2`, else 1.
    /// A fin's share of W is the deck's fin pitch, else [`UM_PER_FIN`].
    ///
    /// # Errors
    /// The sidecar does not parse as a deck.
    fn load(sidecar: &str) -> Result<(Self, verify::Pdk), String> {
        use pnr_core::Process;
        let pdk = verify::Pdk::from_json(sidecar)?;
        let recipe = pdk.recipe("resistor", "");
        let body = recipe.clone().map(|recipe| verify::pdk::Overlay { pdk: &pdk, recipe });
        let r_sheet = body.as_ref().and_then(|o| o.sheet_ohm("rpoly")).map_or(0.0, f64::from);
        let res_w = body.as_ref().and_then(|o| o.width("rpoly")).map_or(0.0, |w| f64::from(w) / 1e3);
        let res_model = recipe.map(|r| r.model).filter(|m| !m.is_empty() && r_sheet > 0.0 && res_w > 0.0);
        let cap_density = pdk.cell.get("cap_density_ff_um2").and_then(Value::as_f64).unwrap_or(1.0);
        let um_per_fin = pdk.width("fin").zip(pdk.space("fin")).map_or(UM_PER_FIN, |(w, s)| f64::from(w + s) / 1e3);
        drop(body);
        Ok((Self { res_model, cap_density, r_sheet, res_w, um_per_fin }, pdk))
    }
}

/// Rewrites generic SPICE into PDK-compatible format:
///
/// - Backslash continuation joining
/// - Bare caps/resistors with real W/L from cap density / sheet-R
/// - Bare R/C with .param value references resolved
/// - FinFET nfin→W synthesis when W is absent on MOSFET lines
/// - MOS L and W below the deck's shortest legal channel raised to it
///
/// The result ends in a newline.
///
/// # Errors
/// The deck at `pdk_path` cannot be read or parsed.
pub fn preprocess_spice(text: &str, pdk_path: &Path) -> Result<String, String> {
    let text = join_backslash(text);
    let (rw, pdk) = Rewrite::load(&fs::read_to_string(pdk_path).map_err(|e| e.to_string())?)?;
    let params = collect_spice_params(&text);
    let mut out: String = text.lines().map(|line| rewrite_line(line, &rw, &params) + "\n").collect();
    if out.is_empty() {
        out.push('\n');
    }
    Ok(raise_channels(out, library::model_table(&pdk), |d| pdk.min_channel(d.kind == pnr_core::DeviceKind::Pmos, &d.model)))
}

/// One card rewritten ([`rewrite_mos`], [`rewrite_rc`]), or `line` as is:
/// blank lines, comments, dot-cards, continuations, cards of fewer than
/// three tokens and other element kinds pass through.
fn rewrite_line(line: &str, rw: &Rewrite, params: &HashMap<String, String>) -> String {
    let stripped = line.trim();
    let tokens: Vec<&str> = stripped.split_whitespace().collect();
    if tokens.len() < 3 || matches!(stripped.as_bytes()[0], b'*' | b'.' | b'+') {
        return line.to_owned();
    }
    let rewritten = match stripped.as_bytes()[0].to_ascii_lowercase() {
        b'm' | b'x' => rewrite_mos(stripped, &tokens, rw, params),
        kind @ (b'c' | b'r') => rewrite_rc(kind, &tokens, rw, params),
        _ => None,
    };
    rewritten.unwrap_or_else(|| line.to_owned())
}

/// An `M`/`X` card with the size it lacks appended: `w` from `nfin` (else
/// `nf`) fins when it has no `w`, [`DEFAULT_L`] when it has no `l`. `None`
/// when it states both.
fn rewrite_mos(stripped: &str, tokens: &[&str], rw: &Rewrite, params: &HashMap<String, String>) -> Option<String> {
    let kv: HashMap<String, &str> =
        tokens.iter().filter_map(|t| t.split_once('=')).map(|(k, v)| (k.to_ascii_lowercase(), v)).collect();
    let mut extra = String::new();
    if !kv.contains_key("w") {
        if let Some(nfin) = kv.get("nfin").or_else(|| kv.get("nf")) {
            let fins = parse_si(&resolve_param(nfin, params)).max(1.0);
            extra.push_str(&format!(" w={}", fmt_um(fins * rw.um_per_fin)));
        }
    }
    if !kv.contains_key("l") {
        extra.push_str(&format!(" l={DEFAULT_L}"));
    }
    (!extra.is_empty()).then(|| format!("{stripped}{extra}"))
}

/// The last value of parameter `key` in `kvs`.
fn kv_get<'a>(kvs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    kvs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// `k=v` pairs, space-separated.
fn join_kv<'a>(kvs: impl Iterator<Item = &'a (String, String)>) -> String {
    kvs.map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" ")
}

/// `inst` as a subcircuit instance: `X`-prefixed unless it already is.
fn x_instance(inst: &str) -> String {
    if matches!(inst.as_bytes().first(), Some(b'x' | b'X')) { inst.to_owned() } else { format!("X{inst}") }
}

/// A bare `C`/`R` card (`kind` is `b'c'` or `b'r'`) as a sized deck
/// instance, or `None` to keep it:
///
/// - a generic-model capacitor with `w` and `l` keeps them, model [`CAP_MODEL`];
/// - a capacitor of value C becomes a square of side √(C / density), at
///   least 0.5 µm;
/// - a resistor of value R, when the deck has a resistor model, becomes
///   min width W and length max(R·W / R_sheet, W).
///
/// The value is the last positional token (`.param`s resolved) when numeric,
/// else, for a generic model word (`resistor`, `res`, `capacitor`, `cap`),
/// its `r=`/`c=` parameter. A zero value keeps the card.
fn rewrite_rc(kind: u8, tokens: &[&str], rw: &Rewrite, params: &HashMap<String, String>) -> Option<String> {
    let kv_start = tokens[1..].iter().position(|t| t.contains('=')).map_or(tokens.len(), |i| i + 1);
    let positional = &tokens[1..kv_start];
    if positional.len() < 2 {
        return None;
    }
    // `k=v` parameters, keys lowercased, values resolved, in card order.
    let kvs: Vec<(String, String)> = tokens[kv_start..]
        .iter()
        .filter_map(|t| t.split_once('='))
        .map(|(k, v)| (k.to_ascii_lowercase(), resolve_param(v, params)))
        .collect();
    let (last, nodes) = positional.split_last()?;
    let nodes = nodes.join(" ");
    let inst = x_instance(tokens[0]);
    let resolved = resolve_param(last, params);
    let value = if is_numeric(&resolved) {
        parse_si(&resolved)
    } else if matches!(last.to_ascii_lowercase().as_str(), "resistor" | "res" | "capacitor" | "cap") {
        if kind == b'c' && kv_get(&kvs, "w").is_some() && kv_get(&kvs, "l").is_some() {
            return Some(format!("{inst} {nodes} {CAP_MODEL} {}", join_kv(kvs.iter())));
        }
        parse_si(kv_get(&kvs, if kind == b'r' { "r" } else { "c" }).filter(|v| is_numeric(v))?)
    } else {
        return None;
    };
    if value == 0.0 {
        return None;
    }
    let rest = join_kv(kvs.iter().filter(|(k, _)| k != "r" && k != "c"));
    let (model, w, l) = if kind == b'c' {
        let side = (value.abs() * 1e15 / rw.cap_density).sqrt().max(0.5);
        (CAP_MODEL, side, side)
    } else {
        let model = rw.res_model.as_deref()?;
        (model, rw.res_w, (value.abs() * rw.res_w / rw.r_sheet).max(rw.res_w))
    };
    Some(format!("{inst} {nodes} {model} W={} L={} {rest}", fmt_um(w), fmt_um(l)).trim_end().to_owned())
}

/// A generic fixture's MOS cards retargeted to the deck's shortest legal
/// channel: a numeric `l`/`w` below it is raised (gf180's 3.3 V gate is 280 nm,
/// the fixtures' 150). The flow draws what a netlist asks and warns; this is
/// the benchmark's retarget. MOS cards are the ones `library::run`'s parse
/// calls MOS under the deck's `models` table ([`library::model_table`]);
/// `legal` is a device's `(l, w)` minimum.
fn raise_channels(text: String, models: Vec<(String, pnr_core::DeviceKind)>, legal: impl Fn(&pnr_core::Device) -> (i32, i32)) -> String {
    let opts = library::ParseOptions { models, ..Default::default() };
    let Ok(netlist) = library::spice_with(&text, &opts) else { return text };
    let mos: HashMap<&str, (i32, i32)> = netlist
        .devices
        .iter()
        .filter(|d| matches!(d.kind, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos))
        .map(|d| (d.name.as_str(), legal(d)))
        .collect();
    let raise = |tok: &str, (l_min, w_min): (i32, i32)| {
        let Some((k, v)) = tok.split_once('=') else { return tok.to_owned() };
        let min = match k.to_ascii_lowercase().as_str() {
            "l" => l_min,
            "w" => w_min,
            _ => 0,
        };
        // ponytail: a `.param` reference stays as written.
        if is_numeric(v) && parse_si(v) * 1e9 < f64::from(min) - 0.5 {
            format!("{k}={}", fmt_um(f64::from(min) / 1e3))
        } else {
            tok.to_owned()
        }
    };
    let mut out: String = text
        .lines()
        .map(|line| match line.split_whitespace().next() {
            Some(name) if mos.contains_key(name) => line.split_whitespace().map(|t| raise(t, mos[name])).collect::<Vec<_>>().join(" "),
            _ => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
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
            .join("pdks/sky130.json");
        let text = ".param rl=500 Csw=5u\n\
                    R1 vps vout resistor r=rl\n\
                    C4 a b capacitor w=Csw l=Csw\n";
        let out = preprocess_spice(text, &pdk_json).expect("preprocess");
        // The deck states no resistor body sheet R: the card stays as written.
        let r_line = out.lines().find(|l| l.contains("vps vout")).unwrap();
        assert!(r_line.starts_with("R1 "), "resistor untouched: {r_line}");
        let c_line = out.lines().find(|l| l.contains("a b")).unwrap();
        assert!(c_line.starts_with("XC4 "), "cap instance: {c_line}");
        assert!(c_line.contains("w=5u") && c_line.contains("l=5u"), "{c_line}");
        assert!(!c_line.contains("capacitor"), "{c_line}");
    }

    #[test]
    fn mos_channels_below_the_deck_minimum_are_raised() {
        // `XM4`'s model names no MOS token: only the deck table makes it one.
        let text = "XM1 d g s b pfet_01v8 W=1u L=0.15u\nXM2 d g s b nfet_01v8 W=0.1u L=2u\nXR1 a b res_generic_po W=0.1u L=2u\nXM4 d g s b fet33p W=1u L=0.15u\n";
        let out = raise_channels(text.to_owned(), vec![("fet33p".into(), pnr_core::DeviceKind::Pmos)], |_| (280, 220));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "XM1 d g s b pfet_01v8 W=1u L=0.28u");
        assert_eq!(lines[1], "XM2 d g s b nfet_01v8 W=0.22u L=2u");
        assert_eq!(lines[2], "XR1 a b res_generic_po W=0.1u L=2u", "not a MOS");
        assert_eq!(lines[3], "XM4 d g s b fet33p W=1u L=0.28u");
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

    /// A fresh scratch directory under the system temp dir.
    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("philis-fixtures-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "* x\n").unwrap();
    }

    fn names(c: &[BenchmarkCircuit]) -> Vec<&str> {
        c.iter().map(|c| c.name.as_str()).collect()
    }

    fn params(kv: &[(&str, &str)]) -> HashMap<String, String> {
        kv.iter().map(|&(k, v)| (k.to_string(), v.to_string())).collect()
    }

    /// 1 fF/µm², 50 Ω/□ resistor body 0.35 µm wide, 0.1 µm per fin.
    fn rw(res_model: Option<&str>) -> Rewrite {
        Rewrite { res_model: res_model.map(str::to_owned), cap_density: 1.0, r_sheet: 50.0, res_w: 0.35, um_per_fin: 0.1 }
    }

    fn line(l: &str, res_model: Option<&str>, p: &[(&str, &str)]) -> String {
        rewrite_line(l, &rw(res_model), &params(p))
    }

    #[test]
    fn suite_names_are_case_insensitive_and_default_to_all() {
        assert_eq!(Suite::from_str("LOCAL"), Suite::Local);
        assert_eq!(Suite::from_str("Align"), Suite::Align);
        assert_eq!(Suite::from_str("magical"), Suite::Magical);
        assert_eq!(Suite::from_str("TinyTapeout"), Suite::TinyTapeout);
        assert_eq!(Suite::from_str("all"), Suite::All);
        assert_eq!(Suite::from_str("whatever"), Suite::All);
        assert_eq!(Suite::from_str(""), Suite::All);
        assert!(Suite::All.includes(Suite::Local) && Suite::All.includes(Suite::All));
        assert!(!Suite::Local.includes(Suite::All), "a single suite does not select everything");
    }

    #[test]
    fn parse_si_suffixes_and_corners() {
        assert!((parse_si("1meg") - 1e6).abs() < 1e-6, "meg is mega, not milli");
        assert!((parse_si("1MEG") - 1e6).abs() < 1e-6);
        assert!((parse_si("1M") - 1e-3).abs() < 1e-15, "SPICE M is milli");
        assert!((parse_si("2t") - 2e12).abs() < 1.0);
        assert!((parse_si("3g") - 3e9).abs() < 1e-3);
        assert!((parse_si("4n") - 4e-9).abs() < 1e-21);
        assert!((parse_si("5f") - 5e-15).abs() < 1e-27);
        assert!((parse_si("-5k") + 5000.0).abs() < 1e-9);
        assert!((parse_si(" 7 ") - 7.0).abs() < 1e-12);
        assert!((parse_si("1e-6") - 1e-6).abs() < 1e-18);
        assert_eq!(parse_si(""), 0.0);
        assert_eq!(parse_si("1e"), 0.0);
        assert_eq!(parse_si("k"), 0.0);
    }

    #[test]
    fn is_numeric_corners() {
        for yes in ["0", "+1", "-1", "1meg", "1.5F", "1e-3", " 2u "] {
            assert!(is_numeric(yes), "{yes}");
        }
        for no in ["", "+", "-", "k", "meg", "1.2.3", "u1", "{w}"] {
            assert!(!is_numeric(no), "{no}");
        }
    }

    #[test]
    fn non_finite_words_are_not_spice_numbers() {
        for no in ["inf", "infinity", "nan", "NaN", "-inf"] {
            assert!(!is_numeric(no), "{no}");
        }
    }

    #[test]
    fn resolve_param_follows_chains_case_insensitively_and_stops_on_cycles() {
        let p = params(&[("wn", "0.5U"), ("wp", "WN"), ("a", "b"), ("b", "a")]);
        assert_eq!(resolve_param("WP", &p), "0.5u");
        assert_eq!(resolve_param("Other", &p), "other", "a non-parameter is returned lowercased");
        let r = resolve_param("a", &p);
        assert!(r == "a" || r == "b", "{r}");
        assert_eq!(resolve_param("", &p), "");
    }

    #[test]
    fn collect_spice_params_corners() {
        let p = collect_spice_params("  .PARAM A=1, b=2\n.param a=3\nR1 a b c=4\n.param lone\n");
        assert_eq!(p.len(), 2);
        assert_eq!(p["a"], "3", "a later definition wins");
        assert_eq!(p["b"], "2");
        assert!(collect_spice_params("").is_empty());
    }

    #[test]
    fn backslash_join_chains_and_edges() {
        assert_eq!(join_backslash("a \\\n b \\\n   c\nd"), "a b c\nd");
        assert_eq!(join_backslash("a\\"), "a\\", "a trailing backslash with nothing after stays");
        assert_eq!(join_backslash(""), "");
        assert_eq!(join_backslash("x\ny"), "x\ny");
    }

    #[test]
    fn backslash_join_handles_crlf() {
        assert_eq!(join_backslash("a \\\r\nb\r\n"), "a b\r\n");
    }

    #[test]
    fn fmt_um_corners() {
        assert_eq!(fmt_um(0.0), "0u");
        assert_eq!(fmt_um(0.0001), "0.0001u");
        assert_eq!(fmt_um(100.0), "100u");
        assert_eq!(fmt_um(0.12345), "0.1235u");
    }

    #[test]
    fn x_instance_prefixes_once() {
        assert_eq!(x_instance("C1"), "XC1");
        assert_eq!(x_instance("xc1"), "xc1");
        assert_eq!(x_instance("XR2"), "XR2");
    }

    #[test]
    fn kv_helpers() {
        let kvs = vec![("w".to_string(), "1u".to_string()), ("w".to_string(), "2u".to_string())];
        assert_eq!(kv_get(&kvs, "w"), Some("2u"));
        assert_eq!(kv_get(&kvs, "l"), None);
        assert_eq!(join_kv(kvs.iter()), "w=1u w=2u");
        assert_eq!(join_kv([].iter()), "");
    }

    #[test]
    fn non_card_lines_pass_through() {
        for l in ["", "   ", "* M1 a b c d nfet nfin=2", ".param x=1", "+ w=1u l=1u", "M1 a", "V1 a b 1", "I1 a b 1u"] {
            assert_eq!(line(l, None, &[]), l, "{l:?}");
        }
    }

    #[test]
    fn mos_cards_get_w_from_fins_and_a_default_l() {
        assert_eq!(line("M1 d g s b nmos nfin=4", None, &[]), "M1 d g s b nmos nfin=4 w=0.4u l=0.15u");
        assert_eq!(line("  m1 d g s b nmos NF=n", None, &[("n", "3")]), "m1 d g s b nmos NF=n w=0.3u l=0.15u");
        assert_eq!(line("M1 d g s b nmos nfin=0", None, &[]), "M1 d g s b nmos nfin=0 w=0.1u l=0.15u", "at least one fin");
        assert_eq!(line("M1 d g s b nmos W=1u", None, &[]), "M1 d g s b nmos W=1u l=0.15u");
        assert_eq!(line("XM1 d g s b nfet W=1u L=1u", None, &[]), "XM1 d g s b nfet W=1u L=1u", "fully sized: untouched");
        assert_eq!(line("M1 d g s b nmos nfin=2 l=1u", None, &[]), "M1 d g s b nmos nfin=2 l=1u w=0.2u");
    }

    #[test]
    fn numeric_capacitors_become_square_deck_caps() {
        // 1 pF at 1 fF/µm²: 1000 µm², a 31.6228 µm square.
        assert_eq!(line("C1 a b 1p", None, &[]), "XC1 a b cap W=31.6228u L=31.6228u");
        assert_eq!(line("C1 a b 1f", None, &[]), "XC1 a b cap W=1u L=1u");
        assert_eq!(line("C1 a b 0.01f", None, &[]), "XC1 a b cap W=0.5u L=0.5u", "0.5 µm minimum side");
        assert_eq!(line("C1 a b -1f m=2", None, &[]), "XC1 a b cap W=1u L=1u m=2", "magnitude; other params kept");
        assert_eq!(line("C1 a b cval", None, &[("cval", "1f")]), "XC1 a b cap W=1u L=1u");
        assert_eq!(line("C1 a b 0", None, &[]), "C1 a b 0", "zero value kept");
    }

    #[test]
    fn generic_capacitors() {
        assert_eq!(line("C4 a b capacitor w=s l=s", None, &[("s", "5u")]), "XC4 a b cap w=5u l=5u");
        assert_eq!(line("C4 a b cap c=1f", None, &[]), "XC4 a b cap W=1u L=1u");
        assert_eq!(line("C4 a b cap c=big", None, &[]), "C4 a b cap c=big", "non-numeric value kept");
        assert_eq!(line("C4 a b cap w=1u", None, &[]), "C4 a b cap w=1u", "w without l and no c");
    }

    #[test]
    fn resistors_need_a_deck_model() {
        assert_eq!(line("R1 a b 1k", None, &[]), "R1 a b 1k");
        // 1 kΩ at 50 Ω/□, 0.35 µm wide: 7 µm long.
        assert_eq!(line("R1 a b 1k", Some("rpoly"), &[]), "XR1 a b rpoly W=0.35u L=7u");
        assert_eq!(line("R1 a b 10", Some("rpoly"), &[]), "XR1 a b rpoly W=0.35u L=0.35u", "at least square");
        assert_eq!(line("R1 vps vout resistor r=rl", Some("rpoly"), &[("rl", "500")]), "XR1 vps vout rpoly W=0.35u L=3.5u");
        assert_eq!(line("R1 a b mymodel", Some("rpoly"), &[]), "R1 a b mymodel", "a deck model word is not generic");
        assert_eq!(line("R1 a r=5", Some("rpoly"), &[]), "R1 a r=5", "one node");
    }

    #[test]
    fn local_discovery_takes_netlists_sorted() {
        let d = scratch("local");
        for f in ["b.sp", "a.spice", "c.txt", "noext"] {
            touch(&d.join(f));
        }
        fs::create_dir_all(d.join("dir.sp")).unwrap();
        let c = discover_local(&d);
        assert_eq!(names(&c), ["a", "b"]);
        assert!(c.iter().all(|c| c.suite == Suite::Local && c.spice_path.is_file()));
        assert!(discover_local(&d.join("missing")).is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn tinytapeout_discovery_prefers_xschem_and_skips_derived_netlists() {
        let d = scratch("tt");
        touch(&d.join("tt_b/mag/b.spice"));
        touch(&d.join("tt_b/xschem/z.spice"));
        touch(&d.join("TT_a/mag/sub/a.spice"));
        touch(&d.join("tt_c/xschem/c_sim.spice"));
        touch(&d.join("tt_c/xschem/c_lvs.spice"));
        touch(&d.join("tt_c/pex/xschem/c.spice"));
        touch(&d.join("tt_d/spi/d.spice"));
        touch(&d.join("other/xschem/o.spice"));
        touch(&d.join("tt_file"));
        let c = discover_tinytapeout(&d);
        assert_eq!(names(&c), ["TT_a", "tt_b", "tt_d"]);
        assert!(c[1].spice_path.ends_with("xschem/z.spice"), "{:?}", c[1].spice_path);
        assert!(c[0].spice_path.ends_with("mag/sub/a.spice"));
        assert!(c.iter().all(|c| c.suite == Suite::TinyTapeout));
        assert!(discover_tinytapeout(&d.join("missing")).is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn find_spice_recursive_sorts_and_filters() {
        let d = scratch("find");
        touch(&d.join("x/xschem/b.spice"));
        touch(&d.join("x/xschem/a.spice"));
        touch(&d.join("x/xschem/a.sp"));
        touch(&d.join("x/notxschem/n.spice"));
        let found = find_spice_recursive(&d, "xschem");
        assert_eq!(found, [d.join("x/xschem/a.spice"), d.join("x/xschem/b.spice")]);
        assert!(find_spice_recursive(&d, "spi").is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn preprocess_spice_needs_a_readable_deck() {
        assert!(preprocess_spice("R1 a b 1k\n", Path::new("/nonexistent/deck.json")).is_err());
    }

    #[test]
    fn preprocess_spice_ends_in_one_newline() {
        let pdk_json = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("pdks/sky130.json");
        let out = preprocess_spice("* only a comment", &pdk_json).expect("preprocess");
        assert!(out.ends_with('\n') && !out.ends_with("\n\n"), "{out:?}");
        assert!(out.contains("* only a comment"));
    }
}
