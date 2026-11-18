//! `cargo bk upgrade <version>`: move every pin to the running cargo-bk version.

use crate::pins;
use crate::workspace::Workspace;
use anyhow::Result;

/// # Errors
/// See [`pins::upgrade`].
pub fn run(ws: &mut Workspace, version: &str) -> Result<()> {
    let changed = pins::upgrade(ws, version)?;
    for file in &changed {
        let shown = file.strip_prefix(&ws.root).unwrap_or(file);
        println!("updated {}", shown.display());
    }
    if changed.is_empty() {
        println!("already at v{}", version.trim_start_matches('v'));
    }
    Ok(())
}
