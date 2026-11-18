//! `cargo bk upgrade <version>`: move every pin to the running cargo-bk version.

use crate::exec::{Cmd, Runner};
use crate::pins;
use crate::workspace::Workspace;
use anyhow::{Result, bail};
use std::io::Write;

/// Moves the pins, then refreshes `Cargo.lock` with `cargo update -p buildkit`
/// when the root manifest changed and a lock file exists, so a `--locked`
/// build does not fail on the old tag.
///
/// # Errors
/// See [`pins::upgrade`]; also when `cargo update` fails.
pub fn run(ws: &mut Workspace, version: &str, r: &dyn Runner, out: &mut dyn Write) -> Result<()> {
    let changed = pins::upgrade(ws, version)?;
    for file in &changed {
        let shown = file.strip_prefix(&ws.root).unwrap_or(file);
        writeln!(out, "updated {}", shown.display())?;
    }
    if changed.is_empty() {
        writeln!(out, "already at v{}", version.trim_start_matches('v'))?;
        return Ok(());
    }
    if changed.contains(&ws.root.join("Cargo.toml")) && ws.root.join("Cargo.lock").is_file() {
        let res = r.run(&Cmd::new("cargo", &["update", "-p", "buildkit"]).cwd(&ws.root))?;
        if res.status != 0 {
            bail!("cargo update -p buildkit failed: {}", res.stderr.trim());
        }
        writeln!(out, "updated Cargo.lock")?;
    }
    Ok(())
}
