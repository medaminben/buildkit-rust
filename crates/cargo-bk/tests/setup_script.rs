//! Runs the generated `scripts/setup.sh --dry-run` against stub tools.

use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use std::fs;
use std::path::Path;
use std::process::Command;

fn generate(dir: &Path) -> String {
    let args = NewArgs {
        name: Some("Foo".to_string()),
        crates: Vec::new(),
        repo: Some("x/foo".to_string()),
        flow: Flow::Direct,
        no_build: true,
    };
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

#[test]
fn a_different_cargo_bk_version_fetches_the_pinned_bootstrap() {
    if cfg!(windows) {
        eprintln!("skipped: setup.ps1 is tested in CI");
        return;
    }
    #[cfg(unix)]
    {
        let (pin, out) = dry_run(Some("9.9.9"));
        let want = format!(
            "git clone --depth 1 --branch v{pin} https://github.com/medaminben/buildkit-rust build/tools/buildkit-rust"
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
}

#[test]
fn a_matching_cargo_bk_version_skips_the_bootstrap() {
    if cfg!(windows) {
        eprintln!("skipped: setup.ps1 is tested in CI");
        return;
    }
    #[cfg(unix)]
    {
        let (_, out) = dry_run(None);
        assert!(!out.contains("git clone"), "{out}");
        assert!(out.contains("cargo bk doctor"), "{out}");
    }
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
