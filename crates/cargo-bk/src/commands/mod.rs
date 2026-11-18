//! One module per subcommand.

pub mod add;
pub mod check_pins;
pub mod doctor;
pub mod link;
pub mod manual;
pub mod new;
pub mod package;
pub mod repo_setup;
pub mod upgrade;
pub mod version;

use crate::workspace::Workspace;
use anyhow::Result;
use std::path::Path;

/// This binary's own pinned version, `X.Y.Z`.
pub const OWN_VERSION: &str = env!("BK_VERSION");

/// Finds the workspace above `start` and warns when it pins another version.
///
/// # Errors
/// When no workspace is found.
pub fn open_workspace(start: &Path) -> Result<Workspace> {
    let ws = Workspace::find(start)?;
    if let Some(pin) = ws.bk_version()
        && pin != OWN_VERSION
    {
        eprintln!(
            "warning: this product pins buildkit-rust v{pin}, cargo-bk is v{OWN_VERSION}; run cargo bk doctor"
        );
    }
    Ok(ws)
}
