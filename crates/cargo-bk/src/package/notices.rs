//! `THIRD_PARTY_NOTICES.txt` content: cargo-about output plus hand-written extras.

use crate::exec::{Cmd, Runner};
use crate::workspace::Workspace;
use anyhow::{Context, Result, bail};

/// Runs `cargo about generate` at the workspace root and appends
/// `packaging/NOTICES-extra.txt` when it exists.
///
/// # Errors
/// When cargo-about cannot run or fails, or the extra file cannot be read.
pub fn notices(ws: &Workspace, r: &dyn Runner) -> Result<String> {
    let out = r.run(&Cmd {
        program: "cargo".into(),
        args: ["about", "generate", "--config", "about.toml", "about.hbs"]
            .map(String::from)
            .to_vec(),
        cwd: Some(ws.root.clone()),
        env: Vec::new(),
    })?;
    if out.status != 0 {
        bail!(
            "cargo about failed (install it with `cargo install cargo-about`): {}",
            out.stderr.trim()
        );
    }
    let mut text = out.stdout.trim_end().to_string();
    let extra_path = ws.root.join("packaging").join("NOTICES-extra.txt");
    if extra_path.is_file() {
        let extra = std::fs::read_to_string(&extra_path)
            .with_context(|| format!("cannot read {}", extra_path.display()))?;
        let extra = extra.trim();
        if !extra.is_empty() {
            text.push_str("\n\n");
            text.push_str(extra);
        }
    }
    text.push('\n');
    Ok(text)
}
