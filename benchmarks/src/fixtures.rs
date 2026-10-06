//! Discover benchmark circuit fixtures (ALIGN, MAGICAL, TinyTapeout) and raise
//! their MOS channels to the deck's legal minimum ([`retarget`]).
//!
//! Repos clone on demand into `benchmarks/fixtures/` and are cleaned up after use.
//! What a generic netlist leaves to the PDK (ideal R/C, nfin, missing W/L) is
//! `library::run`'s own (`library::retarget`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
// The benchmark's retarget. Ideal R/C, nfin and missing W/L are the library's
// (`library::retarget`, run by `library::run`); only raising a fixture's MOS
// channel to the deck's legal minimum is the bench's own.
// ---------------------------------------------------------------------------

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

fn fmt_um(val_um: f64) -> String {
    format!("{:.4}", val_um)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
        + "u"
}

/// `text` with each MOS card's numeric `l`/`w` below `pdk`'s legal channel
/// raised to it ([`raise_channels`]).
pub fn retarget(text: &str, pdk: &verify::Pdk) -> String {
    raise_channels(text.to_owned(), library::model_table(pdk), |d| pdk.min_channel(d.kind == pnr_core::DeviceKind::Pmos, &d.model))
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
