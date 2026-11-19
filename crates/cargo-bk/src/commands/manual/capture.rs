//! `cargo bk manual capture`: runs the product's capture command once per
//! screenshot name and places the PNG for every language that references it.

use super::write_tool;
use crate::exec::Runner;
use crate::os::Os;
use crate::python::find_python;
use crate::workspace::Workspace;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The product has no usable capture configuration; the command exits 2.
#[derive(Debug)]
pub struct NoConfig(pub String);

impl fmt::Display for NoConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NoConfig {}

const NO_COMMAND: &str = "no [workspace.metadata.buildkit.capture] command in Cargo.toml";

fn capture_command(ws: &Workspace) -> Result<Vec<String>> {
    let item = ["workspace", "metadata", "buildkit", "capture", "command"]
        .iter()
        .try_fold(ws.doc.as_item(), |item, key| {
            item.as_table_like().and_then(|t| t.get(key))
        });
    let Some(item) = item else {
        return Err(NoConfig(NO_COMMAND.to_string()).into());
    };
    let words: Option<Vec<String>> = item.as_array().and_then(|a| {
        a.iter()
            .map(|v| v.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
    });
    match words {
        Some(w) if !w.is_empty() => Ok(w),
        _ => Err(NoConfig(
            "[workspace.metadata.buildkit.capture] command must be a non-empty array of strings"
                .to_string(),
        )
        .into()),
    }
}

/// The optional `build` command; `None` when absent.
fn build_command(ws: &Workspace) -> Result<Option<Vec<String>>> {
    let item = ["workspace", "metadata", "buildkit", "capture", "build"]
        .iter()
        .try_fold(ws.doc.as_item(), |item, key| {
            item.as_table_like().and_then(|t| t.get(key))
        });
    let Some(item) = item else {
        return Ok(None);
    };
    let words: Option<Vec<String>> = item.as_array().and_then(|a| {
        a.iter()
            .map(|v| v.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
    });
    match words {
        Some(w) if !w.is_empty() => Ok(Some(w)),
        _ => Err(NoConfig(
            "[workspace.metadata.buildkit.capture] build must be a non-empty array of strings"
                .to_string(),
        )
        .into()),
    }
}

/// Runs the build command once, streaming its output, with no timeout.
fn run_build(argv: &[String], cwd: &Path) -> Result<()> {
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("capture build failed: cannot run `{}`", argv[0]))?;
    if !status.success() {
        bail!("capture build failed ({status})");
    }
    Ok(())
}

/// Distinct shot names with the languages that reference each, from the manual tool.
fn list_shots(ws: &Workspace, r: &dyn Runner, os: Os) -> Result<BTreeMap<String, Vec<String>>> {
    let python = find_python(r, os)?;
    let tool_dir = ws.root.join("build/tools/manual");
    write_tool(&tool_dir)?;
    let mut cmd = python;
    cmd.args
        .push(tool_dir.join("build_manual.py").display().to_string());
    cmd.args.push("--root".to_string());
    cmd.args
        .push(ws.root.join("docs/manual").display().to_string());
    cmd.args.push("--list-shots".to_string());
    cmd.cwd = Some(ws.root.clone());
    let out = r.run(&cmd)?;
    if out.status != 0 {
        bail!("cannot list the shots:\n{}{}", out.stdout, out.stderr);
    }
    let mut shots: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in out.stdout.lines().filter(|l| !l.trim().is_empty()) {
        let Some((lang, name)) = line.split_once(' ') else {
            bail!("unexpected line from --list-shots: {line}");
        };
        shots
            .entry(name.trim().to_string())
            .or_default()
            .push(lang.to_string());
    }
    Ok(shots)
}

fn kill_tree(child: &mut Child) {
    let pid = child.id().to_string();
    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else {
        // The child leads its own process group, so this reaches what it spawned
        // (`cargo run` starts the product as a child).
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn(argv: &[String], cwd: &Path) -> Result<Child> {
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]).current_dir(cwd).stdin(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    cmd.spawn()
        .with_context(|| format!("cannot run `{}`", argv[0]))
}

/// Captures one shot into `part`, then renames it to `dest`.
fn capture_one(
    command: &[String],
    cwd: &Path,
    name: &str,
    part: &Path,
    timeout: Duration,
) -> Result<()> {
    let mut argv = command.to_vec();
    argv.push(format!("--screen={name}"));
    argv.push(format!("--capture={}", part.display()));
    let _ = fs::remove_file(part);
    let mut child = spawn(&argv, cwd)?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            bail!("timed out after {} s", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if !status.success() {
        bail!("{status}");
    }
    match fs::metadata(part) {
        Ok(m) if m.len() > 0 => Ok(()),
        _ => bail!("exited 0 but wrote no file"),
    }
}

fn place(part: &Path, dest: &Path) -> Result<()> {
    fs::rename(part, dest).with_context(|| format!("cannot write {}", dest.display()))
}

/// Runs the capture command for each distinct shot name (or just `only`).
///
/// # Errors
/// [`NoConfig`] when the product has no capture command; otherwise when the
/// shots cannot be listed, a name in `only` is unknown, or any shot fails (all
/// shots are tried first and every failure is listed).
pub fn run(
    ws: &Workspace,
    r: &dyn Runner,
    os: Os,
    only: &[String],
    timeout: Duration,
) -> Result<()> {
    let command = capture_command(ws)?;
    let build = build_command(ws)?;
    let shots = list_shots(ws, r, os)?;
    if let Some(bad) = only.iter().find(|n| !shots.contains_key(*n)) {
        bail!("no shot named {bad} in the manual");
    }
    if let Some(build) = &build {
        run_build(build, &ws.root)?;
    }
    let shots_dir = ws.root.join("docs/manual/assets/shots");
    let mut failures = Vec::new();
    for (name, langs) in &shots {
        if !only.is_empty() && !only.contains(name) {
            continue;
        }
        let target = |lang: &str| -> PathBuf { shots_dir.join(lang).join(format!("{name}.png")) };
        let part =
            |lang: &str| -> PathBuf { shots_dir.join(lang).join(format!("{name}.png.part")) };
        let first = &langs[0];
        let result = (|| -> Result<()> {
            fs::create_dir_all(shots_dir.join(first))?;
            capture_one(&command, &ws.root, name, &part(first), timeout)?;
            place(&part(first), &target(first))?;
            for lang in &langs[1..] {
                fs::create_dir_all(shots_dir.join(lang))?;
                fs::copy(target(first), part(lang))?;
                place(&part(lang), &target(lang))?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => println!("captured {name} ({})", langs.join(", ")),
            Err(e) => {
                for lang in langs {
                    let _ = fs::remove_file(part(lang));
                }
                failures.push(format!("{name}: {e:#}"));
            }
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    bail!(
        "{} shot(s) failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    )
}
