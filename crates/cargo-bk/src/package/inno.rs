//! The Inno Setup installer format.

use super::PackageCtx;
use crate::exec::{Cmd, Runner};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

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

/// The `ISCC` arguments. Each is one argv entry, so paths with spaces need no
/// quoting. `/O` and `/F` override the script's `OutputDir` and
/// `OutputBaseFilename`. `name` is the display name with spaces as `-`.
#[must_use]
pub fn iscc_args(
    iss: &Path,
    stage: &Path,
    out: &Path,
    full: &str,
    numeric: &str,
    name: &str,
) -> Vec<String> {
    vec![
        format!("/DAppVersion={full}"),
        format!("/DAppVersionNumeric={numeric}"),
        format!("/DStageDir={}", stage.display()),
        format!("/O{}", out.display()),
        format!("/F{name}_Setup_{full}"),
        iss.display().to_string(),
    ]
}

/// Builds `<out>/<Name>_Setup_<version>.exe` and a copy `<Name>_Setup.exe`.
/// Returns both paths, the versioned one first.
///
/// # Errors
/// When `ISCC` is missing or fails, or the installer was not written.
pub fn build_inno(
    ctx: &PackageCtx,
    r: &dyn Runner,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<PathBuf>> {
    let iscc = find_iscc(r, env)?;
    fs::create_dir_all(ctx.out).with_context(|| format!("cannot create {}", ctx.out.display()))?;
    let name = ctx.meta.display_name.replace(' ', "-");
    let full = &ctx.version.full;
    let res = r.run(&Cmd {
        program: iscc.display().to_string(),
        args: iscc_args(
            &ctx.root.join(&ctx.meta.iss),
            ctx.stage,
            ctx.out,
            full,
            &ctx.version.numeric,
            &name,
        ),
        cwd: Some(ctx.root.to_path_buf()),
        env: Vec::new(),
    })?;
    if res.status != 0 {
        // ISCC writes most diagnostics to stdout.
        let detail = if res.stderr.trim().is_empty() {
            &res.stdout
        } else {
            &res.stderr
        };
        bail!("ISCC failed: {}", detail.trim());
    }
    let versioned = ctx.out.join(format!("{name}_Setup_{full}.exe"));
    if !versioned.is_file() {
        bail!("ISCC did not write {}", versioned.display());
    }
    let fixed = ctx.out.join(format!("{name}_Setup.exe"));
    fs::copy(&versioned, &fixed).with_context(|| format!("cannot copy to {}", fixed.display()))?;
    Ok(vec![versioned, fixed])
}
