#![allow(dead_code)]

use assert_cmd::Command;
use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use cargo_bk::workspace::PackageMeta;
use std::fs;
use std::path::Path;
use std::process::Command as Proc;

/// `cargo bk new` arguments with no repo, the direct flow and no build.
pub fn new_args(name: Option<&str>, crates: &[&str]) -> NewArgs {
    NewArgs {
        name: name.map(str::to_string),
        crates: crates.iter().map(|c| (*c).to_string()).collect(),
        repo: None,
        flow: Flow::Direct,
        no_build: true,
        fixed_guid: None,
        fixed_year: None,
        fixed_bk_version: None,
        first_release: None,
    }
}

/// Generates the product `Foo` with `crates` into `dir`, without running cargo.
pub fn generate_product(dir: &Path, crates: &[&str]) {
    new_project(dir, &new_args(Some("Foo"), crates), &FakeRunner::new()).unwrap();
}

/// The `cargo-bk` binary, run in `dir`.
pub fn bk(dir: &Path) -> Command {
    let mut c = Command::cargo_bin("cargo-bk").unwrap();
    c.current_dir(dir);
    c
}

/// Runs git in `dir` with no user or system configuration.
pub fn git(dir: &Path, args: &[&str]) {
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

/// The package metadata of `Foo Bar`, with the given apps, Inno script and apt dependencies.
pub fn meta(apps: &[&str], iss: &str, deps: &[&str]) -> PackageMeta {
    let strings = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect();
    PackageMeta {
        display_name: "Foo Bar".into(),
        main_app: "foo-bar-gui".into(),
        apps: strings(apps),
        iss: iss.into(),
        desktop: "packaging/foo-bar.desktop".into(),
        icon: "packaging/icon.png".into(),
        extra_files: vec![],
        system_deps_apt: strings(deps),
    }
}

/// Version `1.2.3-beta.4`.
pub fn version() -> buildkit::Version {
    buildkit::Version {
        numeric: "1.2.3".into(),
        full: "1.2.3-beta.4".into(),
    }
}
