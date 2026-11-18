//! `cargo bk version`.

use super::open_workspace;
use anyhow::Result;
use buildkit::SystemGit;
use std::path::Path;

/// Resolves the product version from git tags and returns it.
///
/// # Errors
/// When `cwd` is not inside a product workspace.
pub fn run(cwd: &Path) -> Result<String> {
    let ws = open_workspace(cwd)?;
    Ok(buildkit::resolve(&ws.root, &ws.fallback_version(), &SystemGit).full)
}
