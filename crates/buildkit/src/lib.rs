//! Derives a product version from git tags with the rules of buildkit's
//! `CMake` `buildkit_git_version`. Meant for a product's `build.rs`.

mod git;
mod tags;

use std::path::{Path, PathBuf};

pub use git::{Git, SystemGit, SystemGitWith};
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

/// Drops the `\\?\` and `\\?\UNC\` prefixes Windows adds to canonical paths.
fn strip_verbatim(s: &str) -> String {
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    s.to_string()
}

/// Canonical path, without the `\\?\` prefix Windows adds.
fn canonical(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    Some(PathBuf::from(strip_verbatim(&c.to_string_lossy())))
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
            "--no-color",
            "--format=%D",
            "HEAD",
        ],
    )?;
    let tag = select_tag(&decorations)?;
    let numeric_part = tag.strip_prefix('v')?;
    let numeric = numeric_part.split('-').next()?.to_string();
    let mut full = numeric_part.to_string();
    // The full ref name: a bare tag name is ambiguous with a branch of that name.
    let range = format!("refs/tags/{tag}..HEAD");
    let ahead: u64 = git
        .run(source_dir, &["rev-list", "--count", &range])?
        .parse()
        .ok()?;
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

fn has_workspace(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest).is_ok_and(|t| t.lines().any(|l| l.trim() == "[workspace]"))
}

/// The first ancestor of `manifest_dir` (itself included) whose `Cargo.toml`
/// has a `[workspace]` table, or `manifest_dir` for a single-crate product.
#[must_use]
pub fn emit_root(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .find(|d| has_workspace(&d.join("Cargo.toml")))
        .unwrap_or(manifest_dir)
        .to_path_buf()
}

fn rerun(p: &Path) -> String {
    format!("cargo:rerun-if-changed={}", p.display())
}

/// A git-reported directory, resolved against `root` when relative.
fn git_dir(root: &Path, git: &dyn Git, flag: &str) -> Option<PathBuf> {
    let out = git.run(root, &["rev-parse", flag])?;
    let p = PathBuf::from(out);
    Some(if p.is_absolute() { p } else { root.join(p) })
}

/// The `cargo:` lines for a build script: the two version variables, and
/// rerun triggers on HEAD, the branch HEAD points to, `packed-refs` and the
/// tags. Linked worktrees keep their own HEAD but share refs with the main
/// repository, hence the two directories.
#[must_use]
pub fn build_script_lines(root: &Path, fallback: &str, git: &dyn Git) -> Vec<String> {
    let v = resolve(root, fallback, git);
    let mut lines = vec![
        format!("cargo:rustc-env=BK_VERSION={}", v.numeric),
        format!("cargo:rustc-env=BK_VERSION_FULL={}", v.full),
        "cargo:rerun-if-changed=build.rs".to_string(),
    ];
    if let Some(gitdir) = git_dir(root, git, "--git-dir") {
        let common = git_dir(root, git, "--git-common-dir").unwrap_or_else(|| gitdir.clone());
        // Cargo reruns on every build for a path that does not exist, so
        // only existing paths are named.
        let mut push = |p: PathBuf| {
            if p.exists() {
                lines.push(rerun(&p));
            }
        };
        push(gitdir.join("HEAD"));
        if let Some(r) = git.run(root, &["symbolic-ref", "-q", "HEAD"]) {
            let loose = common.join(&r);
            if loose.exists() {
                push(loose);
            } else {
                // A packed branch: a commit recreates the loose ref in its
                // directory.
                push(
                    loose
                        .parent()
                        .map_or_else(|| common.join("refs"), Path::to_path_buf),
                );
            }
        }
        push(common.join("packed-refs"));
        push(common.join("refs").join("tags"));
    }
    lines
}

/// `build.rs` entry point. Resolves the version of the product being built
/// (the workspace root above `CARGO_MANIFEST_DIR`) and exports it as
/// `BK_VERSION` and `BK_VERSION_FULL`.
///
/// # Panics
/// When `CARGO_MANIFEST_DIR` is not set, i.e. outside a build script.
pub fn emit_version(fallback: Fallback) {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let root = emit_root(Path::new(&dir));
    for l in build_script_lines(&root, fallback.0, &SystemGit) {
        println!("{l}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Vec<(&'static str, Option<&'static str>)>);
    impl Git for Fake {
        fn run(&self, _: &Path, args: &[&str]) -> Option<String> {
            self.0
                .iter()
                .find(|(k, _)| args.contains(k))
                .and_then(|(_, v)| v.map(str::to_string))
        }
    }

    fn fake(count: Option<&'static str>, sha: Option<&'static str>) -> Fake {
        Fake(vec![
            ("--show-toplevel", Some(".")),
            ("log", Some("tag: refs/tags/v1.0.0")),
            ("--count", count),
            ("--short=7", sha),
        ])
    }

    #[test]
    fn strips_verbatim_prefix() {
        assert_eq!(strip_verbatim(r"\\?\C:\a\b"), r"C:\a\b");
    }

    #[test]
    fn strips_verbatim_unc_prefix() {
        assert_eq!(strip_verbatim(r"\\?\UNC\srv\share"), r"\\srv\share");
    }

    #[test]
    fn leaves_plain_paths() {
        assert_eq!(strip_verbatim("/home/a"), "/home/a");
        assert_eq!(strip_verbatim(r"C:\a"), r"C:\a");
    }

    #[test]
    fn failed_ahead_count_falls_back() {
        let v = resolve(Path::new("."), "9.9.9", &fake(None, Some("abcdef0")));
        assert_eq!(v.numeric, "9.9.9");
        assert_eq!(v.full, "9.9.9-dev");
    }

    #[test]
    fn failed_sha_falls_back() {
        let v = resolve(Path::new("."), "9.9.9", &fake(Some("3"), None));
        assert_eq!(v.full, "9.9.9-dev");
    }

    #[test]
    fn ahead_count_uses_the_full_ref() {
        struct Spy;
        impl Git for Spy {
            fn run(&self, _: &Path, args: &[&str]) -> Option<String> {
                match args[0] {
                    "rev-parse" if args[1] == "--show-toplevel" => Some(".".into()),
                    "log" => Some("tag: refs/tags/v1.0.0".into()),
                    "rev-list" => {
                        assert_eq!(args[2], "refs/tags/v1.0.0..HEAD");
                        Some("0".into())
                    }
                    _ => None,
                }
            }
        }
        let v = resolve(Path::new("."), "9.9.9", &Spy);
        assert_eq!(v.full, "1.0.0");
    }
}
