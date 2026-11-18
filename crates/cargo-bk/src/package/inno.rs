//! Inno Setup: finding the compiler.

use crate::exec::Runner;
use anyhow::{Result, bail};
use std::path::PathBuf;

/// Finds `ISCC.exe`: `BK_ISCC`, then `PATH`, then the two default install
/// folders (`%ProgramFiles(x86)%`, `%LOCALAPPDATA%\Programs`).
///
/// # Errors
/// When `BK_ISCC` names a missing file, or no compiler is found.
pub fn find_iscc(r: &dyn Runner, env: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf> {
    if let Some(p) = env("BK_ISCC") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Ok(path);
        }
        bail!("BK_ISCC points to {p}, which does not exist");
    }
    if let Some(p) = r.which("ISCC") {
        return Ok(p);
    }
    let defaults = [
        env("ProgramFiles(x86)").map(PathBuf::from),
        env("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Programs")),
    ];
    for base in defaults.into_iter().flatten() {
        let path = base.join("Inno Setup 6").join("ISCC.exe");
        if path.exists() {
            return Ok(path);
        }
    }
    bail!("ISCC.exe not found; install Inno Setup (bootstrap group packaging) or set BK_ISCC")
}
