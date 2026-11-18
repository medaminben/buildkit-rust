//! Runs the generated `scripts/setup.sh --dry-run` against stub tools.

mod common;

use cargo_bk::commands::new::new_project;
use cargo_bk::exec::FakeRunner;
use std::fs;
use std::path::Path;
use std::process::Command;

fn generate(dir: &Path) -> String {
    let mut args = common::new_args(Some("Foo"), &[]);
    args.repo = Some("x/foo".to_string());
    new_project(dir, &args, &FakeRunner::new()).unwrap();
    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    let line = cargo
        .lines()
        .find(|l| l.starts_with("version = \""))
        .unwrap();
    line.split('"').nth(1).unwrap().to_string()
}

#[cfg(unix)]
fn stubs(dir: &Path, cargo_bk_version: &str) {
    use std::os::unix::fs::PermissionsExt;
    let write = |name: &str, body: &str| {
        let p = dir.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    };
    write(
        "cargo",
        &format!(
            "if [ \"$1\" = bk ] && [ \"$2\" = --version ]; then echo \"cargo-bk {cargo_bk_version}\"; else echo \"stub cargo $*\"; fi"
        ),
    );
    write("npm", "echo \"stub npm $*\"");
    write("git", "echo \"stub git $*\"");
}

#[cfg(unix)]
fn dry_run(cargo_bk_version: Option<&str>) -> (String, String) {
    dry_run_with(cargo_bk_version, &[])
}

#[cfg(unix)]
fn dry_run_with(cargo_bk_version: Option<&str>, extra: &[&str]) -> (String, String) {
    let tmp = tempfile::tempdir().unwrap();
    let product = tmp.path().join("foo");
    let pin = generate(&product);
    let stub_dir = tmp.path().join("stubs");
    fs::create_dir(&stub_dir).unwrap();
    let version = cargo_bk_version.unwrap_or(&pin).to_string();
    stubs(&stub_dir, &version);
    let path = format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .args(["scripts/setup.sh", "--dry-run"])
        .args(extra)
        .current_dir(&product)
        .env("PATH", path)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (pin, stdout)
}

#[cfg(unix)]
#[test]
fn a_different_cargo_bk_version_fetches_the_pinned_bootstrap() {
    let (pin, out) = dry_run(Some("9.9.9"));
    let want = format!(
        "git clone --depth 1 --branch v{pin} https://github.com/medaminben/buildkit-rust build/tools/buildkit-rust-v{pin}"
    );
    assert!(out.contains(&want), "{out}");
    assert!(
        out.contains("npm ci") || out.contains("npm install"),
        "{out}"
    );
    assert!(out.contains("cargo bk doctor"), "{out}");
    assert!(out.contains("cargo build"), "{out}");
    assert!(out.contains("cargo test"), "{out}");
}

#[cfg(unix)]
#[test]
fn a_matching_cargo_bk_version_skips_the_bootstrap() {
    let (_, out) = dry_run(None);
    assert!(!out.contains("git clone"), "{out}");
    assert!(out.contains("cargo bk doctor"), "{out}");
}

#[test]
fn setup_sh_passes_shellcheck() {
    if Command::new("shellcheck")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipped: shellcheck is not installed");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    generate(tmp.path());
    let out = Command::new("shellcheck")
        .arg(tmp.path().join("scripts/setup.sh"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// Runs setup.sh for real (no `--dry-run`) with stub tools. `dpkg` reports `libvulkan1` as
/// missing, `sudo` logs its arguments. Returns the script's output and the sudo log.
#[cfg(unix)]
fn run_with_missing_package(extra: &[&str]) -> (String, String) {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;
    let tmp = tempfile::tempdir().unwrap();
    let product = tmp.path().join("foo");
    let pin = generate(&product);
    let stub_dir = tmp.path().join("stubs");
    fs::create_dir(&stub_dir).unwrap();
    stubs(&stub_dir, &pin);
    let log = tmp.path().join("sudo.log");
    let write = |name: &str, body: &str| {
        let p = stub_dir.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    };
    write("dpkg", "[ \"$2\" != libvulkan1 ]");
    write("apt-get", "exit 0");
    write("sudo", &format!("echo \"$*\" >> '{}'", log.display()));
    let path = format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg("scripts/setup.sh")
        .args(extra)
        .current_dir(&product)
        .env("PATH", path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{text}");
    (text, fs::read_to_string(&log).unwrap_or_default())
}

#[cfg(target_os = "linux")]
#[test]
fn missing_system_packages_are_installed_only_with_yes() {
    let (out, log) = run_with_missing_package(&[]);
    assert!(out.contains("Missing: libvulkan1"), "{out}");
    assert!(out.contains("[y/N]"), "{out}");
    assert_eq!(log, "", "declined by default, but sudo ran: {log}");

    let (out, log) = run_with_missing_package(&["--yes"]);
    assert!(!out.contains("[y/N]"), "{out}");
    assert!(
        log.contains("apt-get install -y libvulkan1"),
        "log: {log:?}\n{out}"
    );
}

#[test]
fn both_scripts_clone_into_a_folder_per_pin() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates/project/scripts");
    let sh = fs::read_to_string(dir.join("setup.sh.tmpl")).unwrap();
    assert!(
        sh.contains(r#"tools="build/tools/buildkit-rust-v$pin""#),
        "{sh}"
    );
    let ps = fs::read_to_string(dir.join("setup.ps1.tmpl")).unwrap();
    assert!(
        ps.contains(r#"$tools = "build/tools/buildkit-rust-v$pin""#),
        "{ps}"
    );
}
