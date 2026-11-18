//! `cargo bk manual`: build or check the product manual with the embedded tool.

use crate::exec::Runner;
use crate::os::Os;
use crate::python::find_python;
use crate::workspace::Workspace;
use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use std::fs;

static MANUAL_TOOL: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../tools/manual");

fn is_tool_file(name: &str) -> bool {
    !(name.starts_with("test_") || name == "capture.sh")
}

fn write_tool(dir: &Dir, dest: &std::path::Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for file in dir.files() {
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if is_tool_file(name) {
            fs::write(dest.join(name), file.contents())
                .with_context(|| format!("cannot write {name}"))?;
        }
    }
    Ok(())
}

/// Writes the manual tool to `build/tools/manual/` and runs it on `docs/manual`.
///
/// # Errors
/// When Python is missing, the tool cannot be written, or the tool fails.
pub fn run(ws: &Workspace, r: &dyn Runner, os: Os, check: bool) -> Result<()> {
    let python = find_python(r, os)?;
    let tool_dir = ws.root.join("build/tools/manual");
    write_tool(&MANUAL_TOOL, &tool_dir)?;

    let mut cmd = python;
    cmd.args
        .push(tool_dir.join("build_manual.py").display().to_string());
    cmd.args.push("--root".to_string());
    cmd.args
        .push(ws.root.join("docs/manual").display().to_string());
    if check {
        cmd.args.push("--check".to_string());
    } else {
        cmd.args.push("--out".to_string());
        cmd.args
            .push(ws.root.join("build/manual").display().to_string());
    }
    cmd.cwd = Some(ws.root.clone());
    let out = r.run(&cmd)?;
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    if out.status != 0 {
        bail!("manual {} failed", if check { "check" } else { "build" });
    }
    Ok(())
}
