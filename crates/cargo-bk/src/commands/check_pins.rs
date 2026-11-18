//! `cargo bk check-pins`: every buildkit-rust pin must equal the metadata version.

use crate::pins::{PinSite, find_pins, is_buildkit_rust, lint_drift, mismatches};
use crate::workspace::Workspace;
use anyhow::{Result, bail};

/// 1-based line of the first line of `Cargo.toml` that contains all of `needles`, or 1.
fn manifest_line(ws: &Workspace, needles: &[&str]) -> usize {
    std::fs::read_to_string(ws.root.join("Cargo.toml"))
        .ok()
        .and_then(|t| {
            t.lines()
                .position(|l| needles.iter().all(|n| l.contains(n)))
                .map(|i| i + 1)
        })
        .unwrap_or(1)
}

/// Drift warnings go to stderr and never change the result. buildkit-rust itself has no copies
/// to compare: its lint files are the originals.
///
/// # Errors
/// When a pin differs from the metadata version; the message lists each one.
pub fn run(ws: &Workspace) -> Result<()> {
    let github_actions = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");
    if !is_buildkit_rust(&ws.root) {
        for warning in lint_drift(&ws.root, github_actions)? {
            eprintln!("{warning}");
        }
    }
    let pins = find_pins(ws)?;
    let Some(expected) = ws.bk_version() else {
        bail!("[workspace.metadata.buildkit] version is missing");
    };
    let bad = mismatches(&pins);
    if bad.is_empty() {
        return Ok(());
    }
    let rel = |p: &std::path::Path| match p.strip_prefix(&ws.root) {
        Ok(r) => r
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => p.display().to_string(),
    };
    let lines: Vec<String> = bad
        .iter()
        .map(|p| {
            let place = match &p.site {
                PinSite::Metadata => format!("Cargo.toml:{}", manifest_line(ws, &["version"])),
                PinSite::GitDep => {
                    format!("Cargo.toml:{}", manifest_line(ws, &["buildkit", "tag"]))
                }
                PinSite::Workflow { file, line } | PinSite::Doc { file, line } => {
                    format!("{}:{line}", rel(file))
                }
                PinSite::LintHeader { file } => format!("{}:1", rel(file)),
            };
            format!("{place}: v{}, expected v{expected}", p.version)
        })
        .collect();
    bail!("{}", lines.join("\n"))
}
