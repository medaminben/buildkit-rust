use buildkit::Git;
use cargo_bk::commands::link::rewrite_dep;
use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::commands::package;
use cargo_bk::exec::{FakeRunner, SystemRunner};
use cargo_bk::os::Os;
use cargo_bk::package::deb::{build_deb, deb_control, deb_version};
use cargo_bk::workspace::{PackageMeta, Workspace};
use std::fs;
use std::path::Path;
use std::process::Command;

struct NoGit;
impl Git for NoGit {
    fn run(&self, _dir: &Path, _args: &[&str]) -> Option<String> {
        None
    }
}

fn meta(deps: &[&str]) -> PackageMeta {
    PackageMeta {
        display_name: "Foo Bar".into(),
        main_app: "foo-bar-gui".into(),
        apps: vec!["foo-bar-gui".into(), "foo-bar-console".into()],
        iss: "packaging/foo-bar.iss".into(),
        desktop: "packaging/foo-bar.desktop".into(),
        icon: "packaging/icon.png".into(),
        extra_files: vec![],
        system_deps_apt: deps.iter().map(|d| (*d).to_string()).collect(),
    }
}

#[test]
fn deb_versions() {
    assert_eq!(
        deb_version("1.2.3-beta.4+5.gabc1234"),
        "1.2.3~beta.4+5.gabc1234"
    );
    assert_eq!(deb_version("1.2.3"), "1.2.3");
    assert_eq!(deb_version("1.2.3-dev"), "1.2.3~dev");
}

#[test]
fn prerelease_sorts_before_release() {
    let Ok(out) = Command::new("dpkg")
        .args(["--compare-versions", "1.2.3~beta.4", "lt", "1.2.3"])
        .status()
    else {
        return;
    };
    assert!(out.success());
}

#[test]
fn control_has_exact_fields() {
    let text = deb_control(
        &meta(&["libgl1", "libx11-6"]),
        "foo-bar",
        "acme",
        "1.2.3",
        42,
    );
    assert_eq!(
        text,
        "Package: foo-bar\nVersion: 1.2.3\nArchitecture: amd64\nMaintainer: acme\n\
         Installed-Size: 42\nDepends: libgl1, libx11-6\nDescription: Foo Bar\n"
    );
    let none = deb_control(&meta(&[]), "foo-bar", "foo-bar", "1.2.3", 1);
    assert!(!none.contains("Depends"));
}

fn product(root: &Path, repo: &str) -> Workspace {
    let text = format!(
        "[workspace]\nmembers = []\n[workspace.metadata.buildkit]\nrepo = \"{repo}\"\n\
         [workspace.metadata.bk.package]\ndisplay-name = \"Foo Bar\"\nmain-app = \"foo-bar-gui\"\n\
         apps = [\"foo-bar-gui\", \"foo-bar-console\"]\niss = \"packaging/foo-bar.iss\"\n\
         desktop = \"packaging/foo-bar.desktop\"\nicon = \"packaging/icon.png\"\n\
         [workspace.metadata.bk.package.system-deps]\napt = [\"libgl1\"]\n"
    );
    for (rel, data) in [
        ("packaging/foo-bar.desktop", "[Desktop Entry]\n"),
        ("packaging/icon.png", "png"),
        ("LICENSE", "MIT text"),
        ("Cargo.lock", ""),
        ("build/target/release/foo-bar-gui", "bin"),
        ("build/target/release/foo-bar-console", "bin"),
    ] {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
    Workspace {
        root: root.to_path_buf(),
        doc: text.parse().unwrap(),
    }
}

#[test]
fn untagged_repo_warns_about_dev_version() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = product(tmp.path(), "acme/foo");
    let r = FakeRunner::new()
        .with_output("cargo", 0, "notices body")
        .with_output("dpkg-deb", 0, "");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let built = package::run(&ws, Some("deb"), Os::Linux, &r, &NoGit, &mut out, &mut err).unwrap();
    let err = String::from_utf8(err).unwrap();
    assert!(err.contains(
        "warning: no release tag found (shallow clone? use fetch-depth: 0); packaging as 0.0.0-dev"
    ));
    assert_eq!(built.len(), 1);
    assert!(built[0].ends_with("build/dist/foo-bar_0.0.0~dev_amd64.deb"));
    assert!(
        String::from_utf8(out)
            .unwrap()
            .contains("foo-bar_0.0.0~dev_amd64.deb")
    );

    let calls = r.calls();
    let builds: Vec<_> = calls
        .iter()
        .filter(|c| c.program == "cargo" && c.args.first().is_some_and(|a| a == "build"))
        .collect();
    assert_eq!(builds.len(), 2);
    assert_eq!(
        builds[0].args,
        ["build", "--release", "--locked", "-p", "foo-bar-gui"]
    );
    let dpkg = calls.iter().find(|c| c.program == "dpkg-deb").unwrap();
    assert_eq!(dpkg.args[0], "--root-owner-group");
    assert_eq!(dpkg.args[1], "--build");

    let pkg = tmp.path().join("build/deb-root");
    let control = fs::read_to_string(pkg.join("DEBIAN/control")).unwrap();
    assert!(control.contains("Maintainer: acme\n"));
    assert!(control.contains("Version: 0.0.0~dev\n"));
    let copyright = fs::read_to_string(pkg.join("usr/share/doc/foo-bar/copyright")).unwrap();
    assert!(copyright.starts_with("MIT text"));
    assert!(copyright.contains("notices body"));
    assert!(pkg.join("usr/share/applications/foo-bar.desktop").is_file());
    assert!(
        pkg.join("usr/share/icons/hicolor/256x256/apps/foo-bar.png")
            .is_file()
    );
}

#[test]
fn maintainer_falls_back_to_kebab_and_dpkg_failure_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = product(tmp.path(), "");
    let ok = FakeRunner::new()
        .with_output("cargo", 0, "n")
        .with_output("dpkg-deb", 0, "");
    package::run(
        &ws,
        Some("deb"),
        Os::Linux,
        &ok,
        &NoGit,
        &mut Vec::new(),
        &mut Vec::new(),
    )
    .unwrap();
    let control = fs::read_to_string(tmp.path().join("build/deb-root/DEBIAN/control")).unwrap();
    assert!(control.contains("Maintainer: foo-bar\n"));

    let bad = FakeRunner::new()
        .with_output("cargo", 0, "n")
        .with_failure("dpkg-deb", 2, "boom");
    let e = package::run(
        &ws,
        Some("deb"),
        Os::Linux,
        &bad,
        &NoGit,
        &mut Vec::new(),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(e.to_string().contains("boom"));
}

#[cfg(unix)]
#[test]
fn modes_are_explicit() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let ws = product(tmp.path(), "acme/foo");
    let stage = tmp.path().join("build/stage");
    fs::create_dir_all(&stage).unwrap();
    for f in ["foo-bar-gui", "foo-bar-console"] {
        fs::write(stage.join(f), "bin").unwrap();
    }
    let r = FakeRunner::new().with_output("dpkg-deb", 0, "");
    build_deb(
        tmp.path(),
        &stage,
        &tmp.path().join("build/dist"),
        &ws.package_meta().unwrap(),
        "foo-bar",
        "acme",
        "1.0.0",
        &r,
    )
    .unwrap();
    let mode = |p: &str| {
        fs::metadata(tmp.path().join("build/deb-root").join(p))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode("usr/bin/foo-bar-gui"), 0o755);
    assert_eq!(mode("usr/share/applications/foo-bar.desktop"), 0o644);
    assert_eq!(mode("usr/share"), 0o755);
}

#[test]
#[ignore = "builds a generated product with eframe; takes minutes"]
fn real_deb_of_a_generated_product() {
    let tmp = tempfile::tempdir().unwrap();
    let args = NewArgs {
        name: Some("Foo".into()),
        crates: vec![],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
    };
    new_project(tmp.path(), &args, &SystemRunner).unwrap();
    let mut ws = Workspace::find(tmp.path()).unwrap();
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    rewrite_dep(&mut ws, &checkout).unwrap();
    ws.save().unwrap();
    let ws = Workspace::find(tmp.path()).unwrap();
    // cargo needs a lock file for `--locked`
    let st = Command::new("cargo")
        .args(["generate-lockfile"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(st.success());

    let mut out = Vec::new();
    let built = package::run(
        &ws,
        Some("deb"),
        Os::Linux,
        &SystemRunner,
        &buildkit::SystemGit,
        &mut out,
        &mut std::io::stderr(),
    )
    .unwrap();
    let contents = Command::new("dpkg-deb")
        .arg("--contents")
        .arg(&built[0])
        .output()
        .unwrap();
    let list = String::from_utf8(contents.stdout).unwrap();
    println!("{list}");
    assert!(list.contains("./usr/bin/foo-gui"));
    assert!(list.contains("./usr/bin/foo-console"));
    let info = Command::new("dpkg-deb")
        .arg("--info")
        .arg(&built[0])
        .output()
        .unwrap();
    println!("{}", String::from_utf8_lossy(&info.stdout));
}

#[test]
fn missing_lockfile_fails_before_building() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = product(tmp.path(), "acme/foo");
    fs::remove_file(tmp.path().join("Cargo.lock")).unwrap();
    let r = FakeRunner::new();
    let e = package::run(
        &ws,
        Some("deb"),
        Os::Linux,
        &r,
        &NoGit,
        &mut Vec::new(),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert_eq!(
        e.to_string(),
        "Cargo.lock is missing; run cargo generate-lockfile and commit it (packages build with --locked)"
    );
    assert_eq!(r.calls().len(), 0);
}
