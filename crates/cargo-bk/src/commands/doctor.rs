//! `cargo bk doctor`: checks the machine and the product against what they need.

use super::REPO_URL;
use super::link::MARK_START;
use crate::exec::{Cmd, Runner};
use crate::os::Os;
use crate::package::inno::find_iscc;
use crate::python::find_python;
use crate::workspace::{Workspace, toolchain_channel};
use anyhow::{Result, bail};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::path::PathBuf;

const VSWHERE_TAIL: &str = "Microsoft Visual Studio/Installer/vswhere.exe";
const VSWHERE_DEFAULT_BASE: &str = "C:/Program Files (x86)";

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Ok(String),
    Missing,
    WrongVersion { want: String, got: String },
}

impl Status {
    fn from_found(found: bool) -> Self {
        if found {
            Status::Ok("installed".into())
        } else {
            Status::Missing
        }
    }
}

#[derive(Debug)]
pub struct Check {
    pub name: Cow<'static, str>,
    pub required: bool,
    pub status: Status,
    pub fix: String,
}

fn bootstrap(group: &str) -> String {
    format!("run bootstrap (group {group})")
}

/// First line of `program args`, when it exits 0.
fn first_line(r: &dyn Runner, program: &str, args: &[&str]) -> Option<String> {
    let out = r.run(&Cmd::new(program, args)).ok()?;
    if out.status != 0 {
        return None;
    }
    Some(out.stdout.lines().next().unwrap_or("").trim().to_string())
}

fn status_of(found: Option<String>) -> Status {
    found.map_or(Status::Missing, Status::Ok)
}

fn on_path(r: &dyn Runner, name: &str) -> Option<String> {
    r.which(name).map(|p| p.display().to_string())
}

fn push(
    checks: &mut Vec<Check>,
    name: impl Into<Cow<'static, str>>,
    required: bool,
    status: Status,
    fix: String,
) {
    checks.push(Check {
        name: name.into(),
        required,
        status,
        fix,
    });
}

/// The `channel` of `rust-toolchain.toml`, when the product has one.
fn product_channel(ws: &Workspace) -> Option<String> {
    let text = std::fs::read_to_string(ws.root.join("rust-toolchain.toml")).ok()?;
    toolchain_channel(&text)
}

fn rustc_check(ws: Option<&Workspace>, r: &dyn Runner) -> (Status, String) {
    let Some(line) = first_line(r, "rustc", &["--version"]) else {
        return (Status::Missing, bootstrap("rust"));
    };
    let want = ws
        .and_then(product_channel)
        .filter(|c| c.starts_with(|ch: char| ch.is_ascii_digit()));
    match want {
        Some(want) if !line.starts_with(&format!("rustc {want}")) => {
            let got = line.split_whitespace().nth(1).unwrap_or("").to_string();
            let fix = format!("rustup toolchain install {want}");
            (Status::WrongVersion { want, got }, fix)
        }
        _ => (Status::Ok(line), String::new()),
    }
}

/// The leading `X.Y.Z` of a version, without `-dev`, `-beta.N` or `+N.gSHA` suffixes.
fn numeric_part(version: &str) -> &str {
    version.split(['-', '+']).next().unwrap_or(version)
}

fn cargo_bk_check(ws: Option<&Workspace>, r: &dyn Runner) -> (Status, String) {
    let pin = ws.and_then(Workspace::bk_version);
    let install = pin.as_ref().map_or_else(
        || bootstrap("cargo-tools"),
        |p| format!("cargo install --git {REPO_URL} --tag v{p} cargo-bk --locked"),
    );
    let Some(got) = first_line(r, "cargo-bk", &["--version"]) else {
        return (Status::Missing, install);
    };
    match pin {
        Some(want) if numeric_part(&got) != want => (Status::WrongVersion { want, got }, install),
        _ => (Status::Ok(got), String::new()),
    }
}

/// Path of `vswhere.exe`: under `%ProgramFiles(x86)%`, or its usual place.
#[must_use]
pub fn vswhere_path() -> String {
    let base = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| VSWHERE_DEFAULT_BASE.into());
    PathBuf::from(base).join(VSWHERE_TAIL).display().to_string()
}

fn msvc_check(r: &dyn Runner) -> Status {
    let args = [
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "installationPath",
    ];
    status_of(first_line(r, &vswhere_path(), &args).filter(|p| !p.is_empty()))
}

fn apt_installed(r: &dyn Runner, pkg: &str) -> bool {
    r.run(&Cmd::new("dpkg-query", &["-W", "-f=${Status}", pkg]))
        .is_ok_and(|o| o.status == 0 && o.stdout.contains("install ok installed"))
}

fn link_active(ws: &Workspace) -> bool {
    let Some(parent) = ws.root.parent() else {
        return false;
    };
    std::fs::read_to_string(parent.join(".cargo/config.toml"))
        .is_ok_and(|t| t.lines().any(|l| l.trim().starts_with(MARK_START)))
}

/// Runs every check. `ws` is the product, or `None` for a machine-only check.
#[must_use]
pub fn run_checks(ws: Option<&Workspace>, r: &dyn Runner, os: Os) -> Vec<Check> {
    let mut c = Vec::new();
    let (status, fix) = rustc_check(ws, r);
    push(&mut c, "rustc", true, status, fix);
    push(
        &mut c,
        "cargo",
        true,
        status_of(first_line(r, "cargo", &["--version"])),
        bootstrap("rust"),
    );
    for (name, program) in [("rustfmt", "rustfmt"), ("clippy", "cargo-clippy")] {
        let status = status_of(first_line(r, program, &["--version"]));
        push(&mut c, name, true, status, bootstrap("rust"));
    }
    let (status, fix) = cargo_bk_check(ws, r);
    push(&mut c, "cargo-bk", true, status, fix);
    // cargo-llvm-cov wants its own subcommand name before the flag.
    for (name, args) in [
        ("cargo-deny", &["--version"][..]),
        ("cargo-llvm-cov", &["llvm-cov", "--version"][..]),
        ("cargo-about", &["--version"][..]),
    ] {
        let status = status_of(first_line(r, name, args));
        push(&mut c, name, true, status, bootstrap("cargo-tools"));
    }
    push(
        &mut c,
        "node",
        true,
        status_of(first_line(r, "node", &["--version"])),
        bootstrap("node"),
    );
    if let Some(ws) = ws {
        let status = Status::from_found(ws.root.join(".husky/_").is_dir());
        push(&mut c, "husky", true, status, "run npm install".into());
    }
    let python = find_python(r, os).ok().map(|p| p.program);
    push(
        &mut c,
        "python",
        true,
        status_of(python),
        bootstrap("python"),
    );
    push(
        &mut c,
        "gh",
        false,
        status_of(on_path(r, "gh")),
        bootstrap("github"),
    );
    match os {
        Os::Linux => linux_checks(&mut c, ws, r),
        Os::Windows => windows_checks(&mut c, r),
    }
    if let Some(ws) = ws {
        let status = Status::Ok(
            if link_active(ws) {
                "active"
            } else {
                "not linked"
            }
            .into(),
        );
        push(&mut c, "cargo bk link", false, status, String::new());
    }
    c
}

fn linux_checks(c: &mut Vec<Check>, ws: Option<&Workspace>, r: &dyn Runner) {
    for name in ["dpkg-deb", "appimagetool"] {
        push(
            c,
            name,
            true,
            status_of(on_path(r, name)),
            bootstrap("packaging"),
        );
    }
    let Some(ws) = ws else { return };
    let Ok(meta) = ws.package_meta() else { return };
    for pkg in meta.system_deps_apt {
        let status = Status::from_found(apt_installed(r, &pkg));
        let fix = format!("sudo apt-get install {pkg}");
        push(c, pkg, true, status, fix);
    }
}

fn windows_checks(c: &mut Vec<Check>, r: &dyn Runner) {
    let iscc = find_iscc(r, &|k| std::env::var(k).ok());
    let (status, fix) = match iscc {
        Ok(p) => (Status::Ok(p.display().to_string()), String::new()),
        Err(e) => (Status::Missing, e.to_string()),
    };
    push(c, "ISCC", true, status, fix);
    push(c, "MSVC", true, msvc_check(r), bootstrap("base"));
}

/// The report: one line per check, plus the fix under each one that is not ok.
#[must_use]
pub fn render(checks: &[Check]) -> String {
    let mut out = String::new();
    for c in checks {
        let (label, detail) = match &c.status {
            Status::Ok(d) => ("ok", d.clone()),
            Status::Missing => ("missing", String::new()),
            Status::WrongVersion { want, got } => {
                ("wrong version", format!("want {want}, got {got}"))
            }
        };
        let optional = if c.required { "" } else { " (optional)" };
        let _ = writeln!(out, "{label:<14} {}{optional} {detail}", c.name);
        if !matches!(c.status, Status::Ok(_)) && !c.fix.is_empty() {
            let _ = writeln!(out, "{:<14} fix: {}", "", c.fix);
        }
    }
    out
}

#[must_use]
pub fn failed(checks: &[Check]) -> bool {
    checks
        .iter()
        .any(|c| c.required && !matches!(c.status, Status::Ok(_)))
}

/// Prints the report.
///
/// # Errors
/// When a required check fails.
pub fn run(ws: Option<&Workspace>, r: &dyn Runner, os: Os) -> Result<()> {
    let checks = run_checks(ws, r, os);
    print!("{}", render(&checks));
    if failed(&checks) {
        bail!("doctor: required checks failed");
    }
    Ok(())
}
