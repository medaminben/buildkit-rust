mod common;

use buildkit::{build_script_lines, emit_root};
use common::{git, isolated, new_repo};
use std::fs;
use std::path::Path;

fn rerun_paths(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .filter_map(|l| l.strip_prefix("cargo:rerun-if-changed="))
        .collect()
}

#[test]
fn tagged_repo_emits_version_and_rerun_lines() {
    let r = tempfile::tempdir().unwrap();
    let d = new_repo(r.path(), "p");
    git(&d, &["tag", "v2.0.0"]);
    let lines = build_script_lines(&d, "0.0.0", &isolated());
    assert!(lines.contains(&"cargo:rustc-env=BK_VERSION=2.0.0".to_string()));
    assert!(lines.contains(&"cargo:rustc-env=BK_VERSION_FULL=2.0.0".to_string()));
    let paths = rerun_paths(&lines);
    assert!(
        paths
            .iter()
            .any(|p| p.ends_with("HEAD") && !p.contains("refs"))
    );
    assert!(
        paths.iter().any(|p| p.ends_with("refs/heads/main")),
        "{paths:?}"
    );
    assert!(paths.iter().any(|p| p.ends_with("packed-refs")));
    assert!(paths.iter().any(|p| p.ends_with("refs/tags")));
}

#[test]
fn linked_worktree_points_into_main_repo() {
    let r = tempfile::tempdir().unwrap();
    let d = new_repo(r.path(), "p");
    git(&d, &["tag", "v2.0.0"]);
    let wt = r.path().join("wt");
    git(&d, &["worktree", "add", wt.to_str().unwrap()]);
    let lines = build_script_lines(&wt, "0.0.0", &isolated());
    let paths = rerun_paths(&lines);
    let head = paths.iter().find(|p| p.ends_with("HEAD")).unwrap();
    assert!(head.contains("worktrees"), "{head}");
    let tags = paths.iter().find(|p| p.ends_with("refs/tags")).unwrap();
    assert!(!tags.contains("worktrees"), "{tags}");
    assert!(Path::new(tags).starts_with(d.join(".git")) || tags.contains(".git"));
}

#[test]
fn outside_a_repo_falls_back_without_rerun_paths() {
    let r = tempfile::tempdir().unwrap();
    let lines = build_script_lines(r.path(), "1.2.3", &isolated());
    assert!(lines.contains(&"cargo:rustc-env=BK_VERSION=1.2.3".to_string()));
    assert!(lines.contains(&"cargo:rustc-env=BK_VERSION_FULL=1.2.3-dev".to_string()));
}

#[test]
fn emit_root_walks_up_to_the_workspace() {
    let r = tempfile::tempdir().unwrap();
    let ws = r.path().join("ws");
    let member = ws.join("crates/m");
    fs::create_dir_all(&member).unwrap();
    fs::write(
        ws.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/m\"]\n",
    )
    .unwrap();
    fs::write(member.join("Cargo.toml"), "[package]\nname = \"m\"\n").unwrap();
    assert_eq!(emit_root(&member), ws);
}

#[test]
fn emit_root_single_crate_is_itself() {
    let r = tempfile::tempdir().unwrap();
    fs::write(r.path().join("Cargo.toml"), "[package]\nname = \"m\"\n").unwrap();
    assert_eq!(emit_root(r.path()), r.path());
}

#[test]
fn emit_root_ignores_workspace_in_other_text() {
    let r = tempfile::tempdir().unwrap();
    let m = r.path().join("m");
    fs::create_dir_all(&m).unwrap();
    fs::write(
        r.path().join("Cargo.toml"),
        "# [workspace]\n[workspace.package]\n",
    )
    .unwrap();
    fs::write(m.join("Cargo.toml"), "[package]\n").unwrap();
    assert_eq!(emit_root(&m), m);
}
