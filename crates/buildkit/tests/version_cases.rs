mod common;

use buildkit::{Git, SystemGit, Version, resolve};
use common::{git, isolated, new_repo};
use std::fs;
use std::path::Path;

fn commit(d: &Path, n: u32) {
    for i in 1..=n {
        fs::write(d.join("f.txt"), format!("c{i}")).unwrap();
        git(d, &["commit", "-am", &format!("c{i}")]);
    }
}

fn tag(d: &Path, t: &str) {
    git(d, &["tag", t]);
}

fn get(d: &Path, fallback: &str) -> Version {
    resolve(d, fallback, &isolated())
}

fn check(d: &Path, fallback: &str, numeric: &str, full: &str) {
    let v = get(d, fallback);
    assert_eq!(v.numeric, numeric);
    assert_eq!(v.full, full);
}

/// `prefix` followed by `+<n>.g<7 hex>`.
fn check_ahead(d: &Path, numeric: &str, prefix: &str, n: u32) {
    let v = get(d, "0.0.0");
    assert_eq!(v.numeric, numeric);
    let rest = v
        .full
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("{}", v.full));
    let sha = rest
        .strip_prefix(&format!("+{n}.g"))
        .unwrap_or_else(|| panic!("{}", v.full));
    assert!(
        sha.len() == 7
            && sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "{sha}"
    );
}

fn root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn tag_at_head() {
    let r = root();
    let d = new_repo(r.path(), "tagged");
    tag(&d, "v1.5.4");
    check(&d, "0.0.0", "1.5.4", "1.5.4");
}

#[test]
fn two_commits_after_tag() {
    let r = root();
    let d = new_repo(r.path(), "tagged");
    tag(&d, "v1.5.4");
    commit(&d, 2);
    check_ahead(&d, "1.5.4", "1.5.4", 2);
}

#[test]
fn beta_at_head() {
    let r = root();
    let d = new_repo(r.path(), "beta");
    tag(&d, "v1.6.0-beta.3");
    check(&d, "0.0.0", "1.6.0", "1.6.0-beta.3");
}

#[test]
fn beta_plus_one() {
    let r = root();
    let d = new_repo(r.path(), "beta");
    tag(&d, "v1.6.0-beta.3");
    commit(&d, 1);
    check_ahead(&d, "1.6.0", "1.6.0-beta.3", 1);
}

#[test]
fn unrelated_tags_ignored() {
    let r = root();
    let d = new_repo(r.path(), "other");
    tag(&d, "v1.2.3");
    commit(&d, 1);
    tag(&d, "v9.9.9-rc.1");
    tag(&d, "release-7");
    check_ahead(&d, "1.2.3", "1.2.3", 1);
}

#[test]
fn stable_beats_beta_both_orders() {
    let r = root();
    let a = new_repo(r.path(), "both1");
    tag(&a, "v1.6.0");
    tag(&a, "v1.6.0-beta.3");
    check(&a, "0.0.0", "1.6.0", "1.6.0");
    let b = new_repo(r.path(), "both2");
    tag(&b, "v1.6.0-beta.3");
    tag(&b, "v1.6.0");
    check(&b, "0.0.0", "1.6.0", "1.6.0");
}

#[test]
fn highest_beta_wins() {
    let r = root();
    let d = new_repo(r.path(), "betas");
    tag(&d, "v1.6.0-beta.10");
    tag(&d, "v1.6.0-beta.2");
    check(&d, "0.0.0", "1.6.0", "1.6.0-beta.10");
}

#[test]
fn leading_zeros_rejected() {
    let r = root();
    let d = new_repo(r.path(), "zeros");
    tag(&d, "v01.2.3");
    tag(&d, "v1.2.3-beta.01");
    check(&d, "5.0.0", "5.0.0", "5.0.0-dev");
}

#[test]
fn untagged_with_fallback() {
    let r = root();
    let d = new_repo(r.path(), "untagged");
    check(&d, "1.4.0", "1.4.0", "1.4.0-dev");
}

#[test]
fn no_git_dir() {
    let r = root();
    let d = r.path().join("nogit");
    fs::create_dir_all(&d).unwrap();
    check(&d, "0.0.0", "0.0.0", "0.0.0-dev");
}

#[test]
fn subfolder_of_tagged_repo() {
    let r = root();
    let d = new_repo(r.path(), "tagged");
    tag(&d, "v1.5.4");
    let sub = d.join("sub");
    fs::create_dir_all(&sub).unwrap();
    check(&sub, "3.0.0", "3.0.0", "3.0.0-dev");
}

#[test]
fn system_git_runs() {
    let r = root();
    assert!(SystemGit.run(r.path(), &["--version"]).is_some());
    assert_eq!(get(r.path(), "2.0.0").full, "2.0.0-dev");
    assert_eq!(resolve(r.path(), "2.0.0", &SystemGit).full, "2.0.0-dev");
}
