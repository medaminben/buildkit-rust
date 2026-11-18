//! Derives a product version from git tags with the rules of buildkit's
//! `CMake` `buildkit_git_version`. Meant for a product's `build.rs`.

mod git;
mod tags;

use std::path::{Path, PathBuf};

pub use git::{Git, SystemGit};
pub use tags::select_tag;

/// The version used when git gives no tag, e.g. `Fallback("0.1.0")`.
#[derive(Debug, Clone, Copy)]
pub struct Fallback(pub &'static str);

/// A resolved version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// `X.Y.Z`.
    pub numeric: String,
    /// `X.Y.Z[-beta.N]`, followed by `+<ahead>.g<sha7>` when HEAD is past the
    /// tag, or `<fallback>-dev` without a tag.
    pub full: String,
}

/// Canonical path, without the `\\?\` prefix Windows adds.
fn canonical(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    let s = c.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return Some(PathBuf::from(format!(r"\\{rest}")));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return Some(PathBuf::from(rest));
    }
    Some(c)
}

fn from_tag(source_dir: &Path, git: &dyn Git) -> Option<Version> {
    let top = git.run(source_dir, &["rev-parse", "--show-toplevel"])?;
    // A tree unpacked inside another checkout must not borrow its tags.
    if canonical(source_dir)? != canonical(Path::new(&top))? {
        return None;
    }
    let decorations = git.run(
        source_dir,
        &[
            "log",
            "--simplify-by-decoration",
            "--decorate=full",
            "--format=%D",
            "HEAD",
        ],
    )?;
    let tag = select_tag(&decorations)?;
    let numeric_part = tag.strip_prefix('v')?;
    let numeric = numeric_part.split('-').next()?.to_string();
    let mut full = numeric_part.to_string();
    let range = format!("{tag}..HEAD");
    let ahead: u64 = git
        .run(source_dir, &["rev-list", "--count", &range])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if ahead > 0 {
        let sha = git.run(source_dir, &["rev-parse", "--short=7", "HEAD"])?;
        full = format!("{full}+{ahead}.g{sha}");
    }
    Some(Version { numeric, full })
}

/// Resolves the version of the repository at `source_dir`.
///
/// Without git, without a matching tag, or when `source_dir` is not the
/// toplevel of its own repository, the result is `fallback` and
/// `<fallback>-dev`.
#[must_use]
pub fn resolve(source_dir: &Path, fallback: &str, git: &dyn Git) -> Version {
    from_tag(source_dir, git).unwrap_or_else(|| Version {
        numeric: fallback.to_string(),
        full: format!("{fallback}-dev"),
    })
}

/// `build.rs` entry point. Resolves the version of the package being built and
/// exports it as `BK_VERSION` and `BK_VERSION_FULL`.
///
/// # Panics
/// When `CARGO_MANIFEST_DIR` is not set, i.e. outside a build script.
pub fn emit_version(fallback: Fallback) {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let v = resolve(Path::new(&dir), fallback.0, &SystemGit);
    println!("cargo:rustc-env=BK_VERSION={}", v.numeric);
    println!("cargo:rustc-env=BK_VERSION_FULL={}", v.full);
    println!("cargo:rerun-if-changed=build.rs");
}
