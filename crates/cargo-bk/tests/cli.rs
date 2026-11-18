use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command as Proc;

fn cargo_bk(dir: &Path) -> Command {
    let empty = dir.join("empty-gitconfig");
    fs::write(&empty, "").unwrap();
    let mut c = Command::cargo_bin("cargo-bk").unwrap();
    c.current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", &empty)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    c
}

fn git(dir: &Path, args: &[&str]) {
    let empty = dir.join("empty-gitconfig");
    fs::write(&empty, "").unwrap();
    let out = Proc::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", &empty)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn workspace(root: &Path, manifest: &str) {
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
}

#[test]
fn version_flag_works_with_and_without_bk() {
    let t = tempfile::tempdir().unwrap();
    for args in [&["--version"][..], &["bk", "--version"][..]] {
        cargo_bk(t.path())
            .args(args)
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
    }
}

#[test]
fn version_outside_workspace_fails() {
    let t = tempfile::tempdir().unwrap();
    cargo_bk(t.path())
        .arg("version")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no Cargo workspace found from"))
        .stderr(predicate::str::contains("run cargo bk inside a product"));
}

#[test]
fn version_prints_the_tag() {
    let t = tempfile::tempdir().unwrap();
    workspace(t.path(), "[workspace]\nmembers = []\n");
    git(t.path(), &["init"]);
    git(t.path(), &["add", "Cargo.toml"]);
    git(t.path(), &["commit", "-m", "c0"]);
    git(t.path(), &["tag", "v0.4.0"]);
    cargo_bk(t.path())
        .arg("version")
        .assert()
        .success()
        .stdout("0.4.0\n");
}

#[test]
fn pin_mismatch_warns_and_carries_on() {
    let t = tempfile::tempdir().unwrap();
    workspace(
        t.path(),
        "[workspace]\nmembers = []\n[workspace.metadata.buildkit]\nversion = \"9.9.9\"\n",
    );
    cargo_bk(t.path())
        .arg("version")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: this product pins buildkit-rust v9.9.9, cargo-bk is v",
        ))
        .stderr(predicate::str::contains("run cargo bk doctor"));
}

#[test]
fn unimplemented_command_bails() {
    let t = tempfile::tempdir().unwrap();
    cargo_bk(t.path())
        .arg("doctor")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not implemented yet"));
}
