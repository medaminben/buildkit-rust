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
    // about.hbs is a handlebars template for cargo-about and keeps its braces; the icon is binary.
    for f in files.iter().filter(|f| {
        !["about.hbs", "icon.png", "foo.iss"]
            .iter()
            .any(|n| f.ends_with(n))
    }) {
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

fn with_repo(flow: Flow) -> NewArgs {
    let mut a = args(Some("Foo"), &["core"]);
    a.repo = Some("x/foo".to_string());
    a.flow = flow;
    a
}

fn app_id(dir: &Path) -> String {
    let iss = fs::read_to_string(dir.join("installer/foo.iss")).unwrap();
    assert!(iss.lines().any(|l| l == "AppId={{#MyAppId}"), "{iss}");
    let line = iss
        .lines()
        .find(|l| l.starts_with("#define MyAppId"))
        .unwrap();
    line.split('"').nth(1).unwrap().to_string()
}

#[test]
fn repo_run_writes_gui_packaging_editor_hooks_setup_and_community_files() {
    let tmp = tempfile::tempdir().unwrap();
    new_project(tmp.path(), &with_repo(Flow::Direct), &FakeRunner::new()).unwrap();
    for f in [
        "apps/foo-gui/Cargo.toml",
        "apps/foo-gui/build.rs",
        "apps/foo-gui/src/main.rs",
        "apps/foo-gui/src/app.rs",
        "installer/foo.iss",
        "packaging/foo.desktop",
        "packaging/icon.png",
        "packaging/NOTICES-extra.txt",
        ".vscode/tasks.json",
        ".vscode/launch.json",
        ".vscode/settings.json",
        ".vscode/extensions.json",
        "package.json",
        ".husky/commit-msg",
        ".husky/pre-commit",
        "lint/commitlint.config.cjs",
        ".editorconfig",
        "scripts/setup.sh",
        "scripts/setup.ps1",
        "LICENSE",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/feature_request.yml",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
    ] {
        assert!(tmp.path().join(f).is_file(), "missing {f}");
    }
    let png = fs::read(tmp.path().join("packaging/icon.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let gui = fs::read_to_string(tmp.path().join("apps/foo-gui/src/main.rs")).unwrap();
    assert!(gui.contains("BK_VERSION_FULL"));
    let iss = fs::read_to_string(tmp.path().join("installer/foo.iss")).unwrap();
    assert!(
        iss.contains("; Never change this AppId: it is how upgrades find the existing install.")
    );
    assert!(iss.contains("foo-gui.exe"));
    let lic = fs::read_to_string(tmp.path().join("LICENSE")).unwrap();
    assert!(lic.contains(" x\n"), "{lic}");
    let ext = fs::read_to_string(tmp.path().join(".vscode/extensions.json")).unwrap();
    for e in [
        "rust-lang.rust-analyzer",
        "vadimcn.vscode-lldb",
        "tamasfe.even-better-toml",
    ] {
        assert!(ext.contains(e));
    }

    let ws = cargo_bk::workspace::Workspace::find(tmp.path()).unwrap();
    let meta = ws.package_meta().unwrap();
    assert_eq!(meta.display_name, "Foo");
    assert_eq!(meta.main_app, "foo-gui");
    assert_eq!(meta.apps, ["foo-gui", "foo-console"]);
    assert_eq!(meta.iss, Path::new("installer/foo.iss"));
    assert_eq!(meta.desktop, Path::new("packaging/foo.desktop"));
    assert_eq!(meta.icon, Path::new("packaging/icon.png"));
    assert_eq!(meta.extra_files.len(), 0);
    assert_eq!(meta.system_deps_apt.len(), 8);
    assert!(meta.system_deps_apt.contains(&"libvulkan1".to_string()));
}

#[test]
fn app_id_is_an_upper_case_guid_and_differs_per_product() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    new_project(a.path(), &with_repo(Flow::Direct), &FakeRunner::new()).unwrap();
    new_project(b.path(), &with_repo(Flow::Direct), &FakeRunner::new()).unwrap();
    let (ia, ib) = (app_id(a.path()), app_id(b.path()));
    for id in [&ia, &ib] {
        assert!(id.starts_with('{') && id.ends_with('}'), "{id}");
        let inner = &id[1..id.len() - 1];
        assert_eq!(inner.len(), 36, "{id}");
        assert!(
            inner
                .chars()
                .all(|c| c == '-' || c.is_ascii_digit() || ('A'..='F').contains(&c))
        );
    }
    assert_ne!(ia, ib);
}

#[test]
fn without_repo_there_are_no_community_files() {
    let tmp = tempfile::tempdir().unwrap();
    new_project(tmp.path(), &args(Some("Foo"), &[]), &FakeRunner::new()).unwrap();
    for f in [
        "LICENSE",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
    ] {
        assert!(!tmp.path().join(f).exists(), "{f} should not exist");
    }
    assert!(!tmp.path().join(".github").exists());
    assert!(tmp.path().join("scripts/setup.sh").is_file());
}

#[test]
fn contributing_follows_the_flow() {
    let beta = tempfile::tempdir().unwrap();
    new_project(beta.path(), &with_repo(Flow::Beta), &FakeRunner::new()).unwrap();
    let text = fs::read_to_string(beta.path().join("CONTRIBUTING.md")).unwrap();
    assert!(text.contains("Beta"), "{text}");
    assert!(text.contains("scripts/setup"));
    assert!(text.contains("semver:"));
    let direct = tempfile::tempdir().unwrap();
    new_project(direct.path(), &with_repo(Flow::Direct), &FakeRunner::new()).unwrap();
    let text = fs::read_to_string(direct.path().join("CONTRIBUTING.md")).unwrap();
    assert!(!text.contains("Beta"), "{text}");
    assert!(text.contains("`main`"));
}

#[test]
fn a_folder_holding_only_dot_git_counts_as_empty() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    fs::write(tmp.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    new_project(tmp.path(), &args(Some("Foo"), &[]), &FakeRunner::new()).unwrap();
    assert!(tmp.path().join("Cargo.toml").is_file());
    fs::write(tmp.path().join("notes.txt"), "x").unwrap();
    fs::remove_file(tmp.path().join("Cargo.toml")).unwrap();
    assert!(new_project(tmp.path(), &args(Some("Foo"), &[]), &FakeRunner::new()).is_err());
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
    cargo(&["build", "-p", "foo-gui"]);
    let out = cargo(&["run", "-q", "-p", "foo-console", "--", "--version"]);
    assert_eq!(out.trim(), "foo-console 0.1.0-dev");
}
