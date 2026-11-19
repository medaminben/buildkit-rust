//! `cargo bk manual`: build or check the product manual with the embedded tool.

use crate::exec::Runner;
use crate::os::Os;
use crate::python::find_python;
use crate::workspace::Workspace;
use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use std::fs;
use std::path::Path;

pub mod capture;

static MANUAL_TOOL: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../tools/manual");

fn is_tool_file(name: &str) -> bool {
    !(name.starts_with("test_")
        || name == "capture.sh"
        || Path::new(name).extension().is_some_and(|e| e == "pyc"))
}

fn name_of(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

fn write_dir(dir: &Dir, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for file in dir.files() {
        let name = name_of(file.path());
        if is_tool_file(name) {
            fs::write(dest.join(name), file.contents())
                .with_context(|| format!("cannot write {name}"))?;
        }
    }
    for sub in dir.dirs() {
        let name = name_of(sub.path());
        if name != "__pycache__" {
            write_dir(sub, &dest.join(name))?;
        }
    }
    Ok(())
}

/// Replaces `dest` with the embedded manual tool, so nothing from an older
/// cargo-bk version lingers.
///
/// # Errors
/// When `dest` cannot be cleaned or written.
pub fn write_tool(dest: &Path) -> Result<()> {
    match fs::remove_dir_all(dest) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).with_context(|| format!("cannot clean {}", dest.display())),
    }
    write_dir(&MANUAL_TOOL, dest)
}

/// Writes the manual tool to `build/tools/manual/` and runs it on `docs/manual`.
///
/// # Errors
/// When Python is missing, the tool cannot be written, or the tool fails.
pub fn run(ws: &Workspace, r: &dyn Runner, os: Os, check: bool) -> Result<()> {
    let python = find_python(r, os)?;
    let tool_dir = ws.root.join("build/tools/manual");
    write_tool(&tool_dir)?;

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
