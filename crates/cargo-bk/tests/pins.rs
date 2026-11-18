use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use cargo_bk::pins::{PinSite, find_pins, mismatches, upgrade};
use cargo_bk::workspace::Workspace;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::path::Path;

const OWN: &str = env!("BK_VERSION");

/// A freshly generated product, with the caller workflows as `new` writes them.
fn generated_product(dir: &Path) {
    let args = NewArgs {
        name: Some("Foo".into()),
        crates: vec![],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
        fixed_guid: None,
        fixed_year: None,
        fixed_bk_version: None,
    };
    new_project(dir, &args, &FakeRunner::new()).unwrap();
}

/// A product whose `ci.yml` is a minimal caller with a known line layout.
fn product(dir: &Path) {
    generated_product(dir);
    let wf = dir.join(".github/workflows");
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
    generated_product(t.path());
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
    let workflow_pins = pins
        .iter()
        .filter(|p| matches!(p.site, PinSite::Workflow { .. }))
        .count();
    assert_eq!(workflow_pins, 3, "ci.yml, codeql.yml and release.yml");
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
            PinSite::Workflow { file, line } if file == ci => Some((line, p.version)),
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

/// A folder shaped like buildkit-rust itself: `crates/cargo-bk`, a metadata pin and docs.
fn own_repo(dir: &Path, readme: &str, guide: &str) {
    fs::create_dir_all(dir.join("crates/cargo-bk")).unwrap();
    fs::create_dir_all(dir.join("docs/nested")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\n[workspace.metadata.buildkit]\nversion = \"1.2.3\"\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), readme).unwrap();
    fs::write(dir.join("docs/nested/guide.md"), guide).unwrap();
}

#[test]
fn own_docs_examples_are_checked() {
    let t = tempfile::tempdir().unwrap();
    own_repo(
        t.path(),
        "uses: medaminben/buildkit-rust/.github/workflows/ci.yml@v1.2.3\n`cargo install --git U --tag v1.2.3 cargo-bk`.\n",
        "intro\nuses: medaminben/buildkit-rust/.github/workflows/release.yml@v1.2.4\ncargo install --tag v1.2.5. Done\n--tag vX.Y.Z and @vX.Y.Z are placeholders\n",
    );
    let ws = Workspace::find(t.path()).unwrap();
    let pins = find_pins(&ws).unwrap();
    let docs: Vec<(String, usize, String)> = pins
        .iter()
        .filter_map(|p| match &p.site {
            PinSite::Doc { file, line } => Some((
                file.strip_prefix(t.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                *line,
                p.version.clone(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        docs,
        [
            ("README.md".to_string(), 1, "1.2.3".to_string()),
            ("README.md".to_string(), 2, "1.2.3".to_string()),
            ("docs/nested/guide.md".to_string(), 2, "1.2.4".to_string()),
            ("docs/nested/guide.md".to_string(), 3, "1.2.5".to_string()),
        ]
    );
    let out = cargo_bk(t.path(), &["check-pins"]).failure().code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        err.contains("docs/nested/guide.md:2: v1.2.4, expected v1.2.3"),
        "{err}"
    );
    assert!(
        err.contains("docs/nested/guide.md:3: v1.2.5, expected v1.2.3"),
        "{err}"
    );
    assert!(!err.contains("README.md"), "{err}");
}

#[test]
fn docs_of_a_product_are_not_scanned() {
    let t = tempfile::tempdir().unwrap();
    own_repo(t.path(), "--tag v9.9.9\n", "--tag v9.9.9\n");
    fs::remove_dir(t.path().join("crates/cargo-bk")).unwrap();
    let ws = Workspace::find(t.path()).unwrap();
    assert!(
        find_pins(&ws)
            .unwrap()
            .iter()
            .all(|p| !matches!(p.site, PinSite::Doc { .. }))
    );
    cargo_bk(t.path(), &["check-pins"]).success();
}

#[test]
fn own_lint_files_are_the_originals_and_give_no_drift_warning() {
    let t = tempfile::tempdir().unwrap();
    own_repo(t.path(), "plain\n", "plain\n");
    fs::write(t.path().join("rustfmt.toml"), "edition = \"2024\"\n").unwrap();
    cargo_bk(t.path(), &["check-pins"])
        .success()
        .stderr(predicates::str::contains("differs").not());
}
