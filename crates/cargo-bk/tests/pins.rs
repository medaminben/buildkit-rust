use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use cargo_bk::pins::{PinSite, find_pins, mismatches, upgrade};
use cargo_bk::workspace::Workspace;
use std::fs;
use std::path::Path;

const OWN: &str = env!("BK_VERSION");

fn product(dir: &Path) {
    let args = NewArgs {
        name: Some("Foo".into()),
        crates: vec![],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
    };
    new_project(dir, &args, &FakeRunner::new()).unwrap();
    let wf = dir.join(".github/workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(
        wf.join("ci.yml"),
        format!(
            "name: ci\njobs:\n  ci:\n    uses: medaminben/buildkit-rust/.github/workflows/ci.yml@v{OWN}\n"
        ),
    )
    .unwrap();
}

fn cargo_bk(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    assert_cmd::Command::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(dir)
        .args(args)
        .assert()
}

#[test]
fn fresh_product_has_no_mismatch() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let ws = Workspace::find(t.path()).unwrap();
    let pins = find_pins(&ws).unwrap();
    assert!(pins.iter().any(|p| p.site == PinSite::Metadata));
    assert!(pins.iter().any(|p| p.site == PinSite::GitDep));
    assert!(
        pins.iter()
            .any(|p| matches!(p.site, PinSite::Workflow { .. }))
    );
    assert_eq!(
        pins.iter()
            .filter(|p| matches!(p.site, PinSite::LintHeader { .. }))
            .count(),
        3
    );
    assert_eq!(mismatches(&pins).len(), 0);
    cargo_bk(t.path(), &["check-pins"]).success();
}

#[test]
fn bad_workflow_pin_is_named_with_file_and_line() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let ci = t.path().join(".github/workflows/ci.yml");
    let text = fs::read_to_string(&ci).unwrap().replace(OWN, "9.9.9");
    fs::write(&ci, text).unwrap();
    let out = cargo_bk(t.path(), &["check-pins"]).failure().code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        err.contains(&format!(
            ".github/workflows/ci.yml:4: v9.9.9, expected v{OWN}"
        )),
        "{err}"
    );
}

#[test]
fn upgrade_to_running_version_repairs_and_lists_changed_files() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let ci = t.path().join(".github/workflows/ci.yml");
    let text = fs::read_to_string(&ci).unwrap().replace(OWN, "9.9.9");
    fs::write(&ci, text).unwrap();
    let mut ws = Workspace::find(t.path()).unwrap();
    let changed = upgrade(&mut ws, OWN).unwrap();
    assert_eq!(changed, vec![ci]);
    cargo_bk(t.path(), &["check-pins"]).success();
}

#[test]
fn upgrade_rewrites_every_site_when_metadata_is_stale() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    for rel in ["Cargo.toml", "rustfmt.toml", ".github/workflows/ci.yml"] {
        let p = t.path().join(rel);
        let text = fs::read_to_string(&p).unwrap().replace(OWN, "0.0.1");
        fs::write(&p, text).unwrap();
    }
    let mut ws = Workspace::find(t.path()).unwrap();
    let changed = upgrade(&mut ws, OWN).unwrap();
    assert_eq!(changed.len(), 3, "{changed:?}");
    cargo_bk(t.path(), &["check-pins"]).success();
    let root = fs::read_to_string(t.path().join("Cargo.toml")).unwrap();
    assert!(root.contains(&format!("tag = \"v{OWN}\" }}")), "{root}");
}

#[test]
fn crlf_files_stay_crlf() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let ci = t.path().join(".github/workflows/ci.yml");
    let text = fs::read_to_string(&ci)
        .unwrap()
        .replace(OWN, "9.9.9")
        .replace('\n', "\r\n");
    fs::write(&ci, text).unwrap();
    let mut ws = Workspace::find(t.path()).unwrap();
    assert_eq!(mismatches(&find_pins(&ws).unwrap()).len(), 1);
    upgrade(&mut ws, OWN).unwrap();
    let after = fs::read_to_string(&ci).unwrap();
    assert!(after.contains(&format!("@v{OWN}\r\n")));
    assert!(!after.replace("\r\n", "").contains('\n'));
}

#[test]
fn upgrade_to_another_version_is_refused() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let mut ws = Workspace::find(t.path()).unwrap();
    let e = upgrade(&mut ws, "9.9.9").unwrap_err().to_string();
    assert_eq!(
        e,
        "install cargo-bk v9.9.9 first: cargo install --git https://github.com/medaminben/buildkit-rust --tag v9.9.9 cargo-bk"
    );
    cargo_bk(t.path(), &["upgrade", "v9.9.9"])
        .failure()
        .stderr(predicates::str::contains("install cargo-bk v9.9.9 first"));
}

#[test]
fn missing_workflows_dir_is_fine() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    fs::remove_dir_all(t.path().join(".github")).unwrap();
    let ws = Workspace::find(t.path()).unwrap();
    assert_eq!(mismatches(&find_pins(&ws).unwrap()).len(), 0);
}
