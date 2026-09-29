//! Finds GPurify's `pdks/` in the checkout Cargo fetched for the rev
//! `Cargo.lock` pins, and hands it to the crate as `GPURIFY_PDKS`: the
//! fallback for a sidecar naming a deck that is not vendored in `pdks/decks/`
//! (those are compiled in, src/decks.rs). `GPURIFY_DIR` (a local GPurify
//! tree) overrides it.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=GPURIFY_DIR");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    println!("cargo:rerun-if-changed=../../pdks");
    let dir = std::env::var_os("GPURIFY_DIR").map(PathBuf::from).or_else(checkout);
    if let Some(d) = dir {
        println!("cargo:rustc-env=GPURIFY_PDKS={}", d.join("pdks").display());
    }
}

/// `$CARGO_HOME/git/checkouts/gpurify-*/<rev7>`, the rev from the lockfile.
fn checkout() -> Option<PathBuf> {
    let lock = std::fs::read_to_string("../../Cargo.lock").ok()?;
    let rev = lock.lines().find_map(|l| l.strip_prefix("source = \"git+https://github.com/UW-ASIC/GPurify")?.split('#').nth(1))?;
    let rev = rev.trim_end_matches('"').get(..7)?;
    let home = std::env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))?;
    std::fs::read_dir(home.join("git/checkouts")).ok()?.flatten().map(|e| e.path().join(rev)).find(|p| p.join("pdks").is_dir())
}
