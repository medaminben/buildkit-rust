use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::{FakeRunner, SystemRunner};
use std::fs;
use std::path::Path;
use std::process::Command;

fn args(name: Option<&str>, crates: &[&str]) -> NewArgs {
    NewArgs {
        name: name.map(str::to_string),
        crates: crates.iter().map(|c| (*c).to_string()).collect(),
        repo: None,
        flow: Flow::Direct,
        no_build: true,
    }
}

fn all_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            all_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn writes_workspace_crates_and_console_app() {
    let tmp = tempfile::tempdir().unwrap();
    let report = new_project(
        tmp.path(),
        &args(Some("Foo"), &["core", "physics"]),
        &FakeRunner::new(),
    )
    .unwrap();
    assert_ne!(report.created.len(), 0);
    let root = fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
    assert!(root.contains("[workspace.metadata.buildkit]"));
    assert!(root.contains("flow = \"direct\""));
    assert!(root.contains("repo = \"\""));
    let core = fs::read_to_string(tmp.path().join("crates/core/Cargo.toml")).unwrap();
    assert!(core.contains("name = \"foo-core\""));
    let phys = fs::read_to_string(tmp.path().join("crates/physics/Cargo.toml")).unwrap();
    assert!(phys.contains("name = \"foo-physics\""));
    assert!(tmp.path().join("crates/physics/src/lib.rs").is_file());
    assert!(tmp.path().join("crates/physics/tests/physics.rs").is_file());
    assert!(tmp.path().join("apps/foo-console/src/main.rs").is_file());
    assert!(tmp.path().join("apps/foo-console/build.rs").is_file());
    let rustfmt = fs::read_to_string(tmp.path().join("rustfmt.toml")).unwrap();
    assert!(rustfmt.starts_with("# buildkit-rust v"));

    let mut files = Vec::new();
    all_files(tmp.path(), &mut files);
    // about.hbs is a handlebars template for cargo-about and keeps its braces.
    for f in files.iter().filter(|f| !f.ends_with("about.hbs")) {
        let text = fs::read_to_string(f).unwrap();
        assert!(
            !text.contains("{{"),
            "{} has an unrendered placeholder",
            f.display()
        );
    }
}

#[test]
fn name_defaults_to_folder_and_crates_to_core() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("my-game");
    new_project(&dir, &args(None, &[]), &FakeRunner::new()).unwrap();
    let core = fs::read_to_string(dir.join("crates/core/Cargo.toml")).unwrap();
    assert!(core.contains("name = \"my-game-core\""));
    assert!(dir.join("apps/my-game-console/Cargo.toml").is_file());
}

#[test]
fn second_run_creates_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let a = args(Some("Foo"), &["core"]);
    new_project(tmp.path(), &a, &FakeRunner::new()).unwrap();
    let again = new_project(tmp.path(), &a, &FakeRunner::new()).unwrap();
    assert_eq!(again.created.len(), 0);
    assert_ne!(again.kept.len(), 0);
}

#[test]
fn refuses_a_foreign_non_empty_folder() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("notes.txt"), "x").unwrap();
    let err = new_project(tmp.path(), &args(Some("Foo"), &[]), &FakeRunner::new()).unwrap_err();
    assert!(
        err.to_string()
            .contains("folder is not empty and is not a buildkit-rust product"),
        "{err}"
    );
}

#[test]
fn runs_build_test_run_unless_no_build() {
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new().with_output("cargo", 0, "");
    let mut a = args(Some("Foo"), &["core"]);
    a.no_build = false;
    new_project(tmp.path(), &a, &runner).unwrap();
    let calls = runner.calls();
    let shown: Vec<_> = calls.iter().map(|c| c.args.join(" ")).collect();
    assert_eq!(shown, ["build", "test", "run -p foo-console"]);
    assert!(calls.iter().all(|c| c.cwd.as_deref() == Some(tmp.path())));
}

#[test]
fn a_failing_build_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new().with_output("cargo", 101, "");
    let mut a = args(Some("Foo"), &["core"]);
    a.no_build = false;
    assert!(new_project(tmp.path(), &a, &runner).is_err());
}

#[test]
#[ignore = "builds a generated product with a real cargo"]
fn generated_product_builds_and_reports_its_version() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("My Projékt dir").join("foo");
    new_project(&dir, &args(None, &["core", "physics"]), &SystemRunner).unwrap();

    let bk = Path::new(env!("CARGO_MANIFEST_DIR")).join("../buildkit");
    let bk = fs::canonicalize(bk).unwrap();
    // The github repository is not reachable from every machine, so point the
    // dependency at the local checkout instead of patching the git source.
    let manifest = dir.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let git_dep = text
        .lines()
        .find(|l| l.starts_with("buildkit = { git"))
        .unwrap();
    let local = format!("buildkit = {{ path = \"{}\" }}", bk.display());
    fs::write(&manifest, text.replace(git_dep, &local)).unwrap();
    let cargo = |extra: &[&str]| {
        let out = Command::new("cargo")
            .args(extra)
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", tmp.path().join("target"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "cargo {extra:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    cargo(&["test"]);
    let out = cargo(&["run", "-q", "-p", "foo-console", "--", "--version"]);
    assert_eq!(out.trim(), "foo-console 0.1.0-dev");
}
