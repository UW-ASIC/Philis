//! Bakes the commit into `philis --version`: `PHILIS_GIT_REV` if set (a
//! packager building from a tarball with no `.git`), else `git describe`.
// ponytail: reruns on HEAD moves and env changes only; a commit on the same
// branch is caught by git's HEAD reflog touching .git/HEAD in practice.

fn main() {
    println!("cargo:rerun-if-env-changed=PHILIS_GIT_REV");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    let rev = std::env::var("PHILIS_GIT_REV").ok().or_else(|| {
        let o = std::process::Command::new("git")
            .args(["describe", "--always", "--dirty"])
            .output()
            .ok()?;
        o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });
    println!("cargo:rustc-env=PHILIS_GIT_REV={}", rev.unwrap_or_else(|| "unknown".into()));
}
