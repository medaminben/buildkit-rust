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

#[test]
fn quoted_and_commented_pins() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let u = "medaminben/buildkit-rust/.github/workflows/ci.yml";
    let ci = t.path().join(".github/workflows/ci.yml");
    let body = format!(
        "a:\n    uses: \"{u}@v1.2.3\"\nb:\n    uses: '{u}@v1.2.4'\nc:\n    uses: {u}@v1.2.5 # keep\n  # uses: {u}@v0.0.1\n"
    );
    fs::write(&ci, &body).unwrap();
    let ws = Workspace::find(t.path()).unwrap();
    let found: Vec<(usize, String)> = find_pins(&ws)
        .unwrap()
        .into_iter()
        .filter_map(|p| match p.site {
            PinSite::Workflow { line, .. } => Some((line, p.version)),
            _ => None,
        })
        .collect();
    assert_eq!(
        found,
        [
            (2, "1.2.3".into()),
            (4, "1.2.4".into()),
            (6, "1.2.5".into())
        ]
    );
    let mut ws = ws;
    upgrade(&mut ws, OWN).unwrap();
    let after = fs::read_to_string(&ci).unwrap();
    assert_eq!(
        after,
        body.replace("1.2.3", OWN)
            .replace("1.2.4", OWN)
            .replace("1.2.5", OWN)
    );
}

fn crlf(path: &Path) {
    let text = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    fs::write(path, text.replace('\n', "\r\n")).unwrap();
}

fn stderr(out: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&out.get_output().stderr).into_owned()
}

#[test]
fn crlf_lint_copies_give_no_drift_warning() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    for f in [
        "rustfmt.toml",
        "clippy.toml",
        "deny.toml",
        "lint/commitlint.config.cjs",
        ".editorconfig",
    ] {
        assert!(t.path().join(f).exists(), "{f}");
        crlf(&t.path().join(f));
    }
    let out = cargo_bk(t.path(), &["check-pins"]).success();
    assert!(!stderr(&out).contains("differs"), "{}", stderr(&out));
}

#[test]
fn changed_rule_line_warns_and_exits_zero() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    let f = t.path().join("clippy.toml");
    let mut text = fs::read_to_string(&f).unwrap();
    text.push_str("too-many-lines-threshold = 1\n");
    fs::write(&f, text).unwrap();
    let out = cargo_bk(t.path(), &["check-pins"]).success();
    assert!(
        stderr(&out).contains(&format!(
            "warning: clippy.toml differs from buildkit-rust v{OWN}"
        )),
        "{}",
        stderr(&out)
    );
}

#[test]
fn github_actions_prefixes_the_warning() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    fs::write(t.path().join(".editorconfig"), "root = true\n").unwrap();
    let out = assert_cmd::Command::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(t.path())
        .env("GITHUB_ACTIONS", "true")
        .arg("check-pins")
        .assert()
        .success();
    assert!(
        stderr(&out).contains("::warning file=.editorconfig::"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn missing_lint_copy_is_not_checked() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    fs::remove_file(t.path().join("deny.toml")).unwrap();
    let out = cargo_bk(t.path(), &["check-pins"]).success();
    assert!(!stderr(&out).contains("deny.toml"));
}

#[test]
fn pin_mismatch_with_drift_exits_one_with_both_messages() {
    let t = tempfile::tempdir().unwrap();
    product(t.path());
    fs::write(t.path().join(".editorconfig"), "root = true\n").unwrap();
    let ci = t.path().join(".github/workflows/ci.yml");
    let text = fs::read_to_string(&ci).unwrap().replace(OWN, "0.0.1");
    fs::write(&ci, text).unwrap();
    let out = cargo_bk(t.path(), &["check-pins"]).failure().code(1);
    let err = stderr(&out);
    assert!(err.contains("ci.yml:4: v0.0.1"), "{err}");
    assert!(err.contains("warning: .editorconfig differs"), "{err}");
}
