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
    let fallback = ws
        .root_package_version()
        .unwrap_or_else(|| "0.0.0".to_string());
    Ok(buildkit::resolve(&ws.root, &fallback, &SystemGit).full)
}
