//! Shared helpers: temp repos with an isolated git config.
#![allow(dead_code)]
use buildkit::SystemGitWith;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// An empty global config, so the user's own git config never leaks in.
fn empty_config() -> &'static Path {
    static F: OnceLock<PathBuf> = OnceLock::new();
    F.get_or_init(|| {
        let p =
            std::env::temp_dir().join(format!("buildkit-empty-gitconfig-{}", std::process::id()));
        fs::write(&p, "").unwrap();
        p
    })
}

pub fn isolated() -> SystemGitWith {
    SystemGitWith {
        env: vec![(
            "GIT_CONFIG_GLOBAL".to_string(),
            empty_config().to_string_lossy().into_owned(),
        )],
    }
}

pub fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
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
        .env("GIT_CONFIG_GLOBAL", empty_config())
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

pub fn new_repo(root: &Path, name: &str) -> PathBuf {
    let d = root.join(name);
    fs::create_dir_all(&d).unwrap();
    git(&d, &["init"]);
    fs::write(d.join("f.txt"), "0").unwrap();
    git(&d, &["add", "f.txt"]);
    git(&d, &["commit", "-m", "c0"]);
    d
}
