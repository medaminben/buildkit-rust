use cargo_bk::commands::doctor::{Check, Status, failed, run_checks, vswhere_path};
use cargo_bk::exec::FakeRunner;
use cargo_bk::os::Os;
use cargo_bk::package::inno::find_iscc;
use cargo_bk::workspace::Workspace;
use std::fs;
use std::path::{Path, PathBuf};

const PIN_INSTALL: &str = "cargo install --git https://github.com/medaminben/buildkit-rust --tag v0.3.0 cargo-bk --locked";

fn product(dir: &Path) -> Workspace {
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\n[workspace.metadata.buildkit]\nversion = \"0.3.0\"\n[workspace.metadata.bk.package]\ndisplay-name = \"X\"\nmain-app = \"x\"\napps = [\"x\"]\niss = \"a.iss\"\ndesktop = \"a.desktop\"\nicon = \"a.png\"\nsystem-deps.apt = [\"libfoo-dev\"]\n",
    )
    .unwrap();
    fs::write(
        dir.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"1.90.0\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join(".husky/_")).unwrap();
    Workspace::find(dir).unwrap()
}

fn linux_runner() -> FakeRunner {
    FakeRunner::new()
        .with_output("rustc", 0, "rustc 1.90.0 (abc 2025-09-14)\n")
        .with_output("cargo", 0, "cargo 1.90.0\n")
        .with_output("rustfmt", 0, "rustfmt 1.8.0\n")
        .with_output("cargo-clippy", 0, "clippy 0.1.90\n")
        .with_output("cargo-bk", 0, "0.3.0\n")
        .with_output("cargo-deny", 0, "cargo-deny 0.18.0\n")
        .with_output("cargo-llvm-cov", 0, "cargo-llvm-cov 0.6.0\n")
        .with_output("cargo-about", 0, "cargo-about 0.7.0\n")
        .with_output("node", 0, "v22.1.0\n")
        .with_output("python3", 0, "Python 3.12.1\n")
        .with_output("dpkg-query", 0, "install ok installed")
        .with_tool("dpkg-deb", "/usr/bin/dpkg-deb")
        .with_tool("appimagetool", "/home/u/.local/bin/appimagetool")
        .with_tool("gh", "/usr/bin/gh")
}

fn windows_runner() -> FakeRunner {
    FakeRunner::new()
        .with_output("rustc", 0, "rustc 1.90.0 (abc 2025-09-14)\n")
        .with_output("cargo", 0, "cargo 1.90.0\n")
        .with_output("rustfmt", 0, "rustfmt 1.8.0\n")
        .with_output("cargo-clippy", 0, "clippy 0.1.90\n")
        .with_output("cargo-bk", 0, "0.3.0\n")
        .with_output("cargo-deny", 0, "cargo-deny 0.18.0\n")
        .with_output("cargo-llvm-cov", 0, "cargo-llvm-cov 0.6.0\n")
        .with_output("cargo-about", 0, "cargo-about 0.7.0\n")
        .with_output("node", 0, "v22.1.0\n")
        .with_output("python3", 0, "Python 3.12.1\n")
        .with_output(&vswhere_path(), 0, "C:\\VS\\BuildTools\n")
        .with_tool("ISCC", "C:\\Inno\\ISCC.exe")
        .with_tool("gh", "C:\\gh.exe")
}

fn find<'a>(checks: &'a [Check], name: &str) -> &'a Check {
    checks
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no check {name}"))
}

fn names(checks: &[Check]) -> Vec<&str> {
    checks.iter().map(|c| &*c.name).collect()
}

#[test]
fn everything_present_passes() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let checks = run_checks(Some(&ws), &linux_runner(), Os::Linux);
    assert!(!failed(&checks), "{checks:?}");
    assert!(names(&checks).contains(&"libfoo-dev"));
    assert!(names(&checks).contains(&"husky"));
}

#[test]
fn missing_cargo_about_names_the_group() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let r = linux_runner().with_output("cargo-about", 127, "");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    assert!(failed(&checks));
    let c = find(&checks, "cargo-about");
    assert_eq!(c.status, Status::Missing);
    assert_eq!(c.fix, "run bootstrap (group cargo-tools)");
}

#[test]
fn cargo_bk_against_the_pin() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let r = linux_runner().with_output("cargo-bk", 0, "0.2.0\n");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    let c = find(&checks, "cargo-bk");
    assert_eq!(
        c.status,
        Status::WrongVersion {
            want: "0.3.0".into(),
            got: "0.2.0".into()
        }
    );
    assert_eq!(c.fix, PIN_INSTALL);
    assert!(failed(&checks));
}

#[test]
fn wrong_toolchain_channel() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let r = linux_runner().with_output("rustc", 0, "rustc 1.85.0 (abc 2025-02-01)\n");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    assert!(matches!(
        find(&checks, "rustc").status,
        Status::WrongVersion { .. }
    ));
}

#[test]
fn outside_a_product_the_product_checks_are_skipped() {
    let checks = run_checks(None, &linux_runner(), Os::Linux);
    assert!(!failed(&checks), "{checks:?}");
    let n = names(&checks);
    for skipped in ["husky", "libfoo-dev", "cargo bk link"] {
        assert!(!n.contains(&skipped), "{skipped}");
    }
    // The cargo-bk presence check stays, without a pin to compare.
    assert!(matches!(find(&checks, "cargo-bk").status, Status::Ok(_)));
}

#[test]
fn missing_husky_and_apt_package() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    fs::remove_dir_all(t.path().join(".husky")).unwrap();
    let r = linux_runner().with_output("dpkg-query", 1, "");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    assert_eq!(find(&checks, "husky").status, Status::Missing);
    let c = find(&checks, "libfoo-dev");
    assert_eq!(c.status, Status::Missing);
    assert_eq!(c.fix, "sudo apt-get install libfoo-dev");
}

#[test]
fn old_python_fails() {
    let r = linux_runner().with_output("python3", 0, "Python 3.9.2\n");
    let checks = run_checks(None, &r, Os::Linux);
    assert_eq!(find(&checks, "python").status, Status::Missing);
    assert_eq!(find(&checks, "python").fix, "run bootstrap (group python)");
}

#[test]
fn missing_gh_is_optional() {
    let r = FakeRunner::new()
        .with_output("rustc", 0, "rustc 1.90.0\n")
        .with_output("cargo", 0, "cargo 1.90.0\n")
        .with_output("rustfmt", 0, "rustfmt 1\n")
        .with_output("cargo-clippy", 0, "clippy 1\n")
        .with_output("cargo-bk", 0, "0.3.0\n")
        .with_output("cargo-deny", 0, "x\n")
        .with_output("cargo-llvm-cov", 0, "x\n")
        .with_output("cargo-about", 0, "x\n")
        .with_output("node", 0, "v22\n")
        .with_output("python3", 0, "Python 3.12.1\n")
        .with_tool("dpkg-deb", "/usr/bin/dpkg-deb")
        .with_tool("appimagetool", "/x/appimagetool");
    let checks = run_checks(None, &r, Os::Linux);
    assert_eq!(find(&checks, "gh").status, Status::Missing);
    assert!(!failed(&checks));
}

#[test]
fn windows_has_iscc_and_msvc_but_no_dpkg() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let checks = run_checks(Some(&ws), &windows_runner(), Os::Windows);
    assert!(!failed(&checks), "{checks:?}");
    let n = names(&checks);
    assert!(n.contains(&"ISCC"));
    assert!(n.contains(&"MSVC"));
    for absent in ["dpkg-deb", "appimagetool", "libfoo-dev"] {
        assert!(!n.contains(&absent), "{absent}");
    }
}

#[test]
fn windows_without_msvc_names_base() {
    let r = windows_runner().with_output(&vswhere_path(), 0, "\n");
    let checks = run_checks(None, &r, Os::Windows);
    let c = find(&checks, "MSVC");
    assert_eq!(c.status, Status::Missing);
    assert_eq!(c.fix, "run bootstrap (group base)");
}

#[test]
fn link_marker_is_reported_as_info() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("prod");
    fs::create_dir_all(&root).unwrap();
    let ws = product(&root);
    fs::create_dir_all(t.path().join(".cargo")).unwrap();
    fs::write(
        t.path().join(".cargo/config.toml"),
        "# >>> cargo bk link\n[patch]\n# <<< cargo bk link\n",
    )
    .unwrap();
    let checks = run_checks(Some(&ws), &linux_runner(), Os::Linux);
    let c = find(&checks, "cargo bk link");
    assert!(!c.required);
    assert_eq!(c.status, Status::Ok("active".into()));
}

fn env_of<'a>(pairs: &'a [(&'a str, String)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
}

#[test]
fn iscc_search_order() {
    let t = tempfile::tempdir().unwrap();
    let real = t.path().join("ISCC.exe");
    fs::write(&real, "").unwrap();
    let none = FakeRunner::new();
    let on_path = FakeRunner::new().with_tool("ISCC", "/p/ISCC");

    // BK_ISCC wins over PATH.
    let env = [("BK_ISCC", real.display().to_string())];
    assert_eq!(find_iscc(&on_path, &env_of(&env)).unwrap(), real);
    // A BK_ISCC that does not exist is an error, even with ISCC on PATH.
    let env = [("BK_ISCC", "/nope/ISCC.exe".to_string())];
    assert_eq!(
        find_iscc(&on_path, &env_of(&env)).unwrap_err().to_string(),
        "BK_ISCC points to /nope/ISCC.exe, which does not exist"
    );
    // PATH next.
    assert_eq!(
        find_iscc(&on_path, &env_of(&[])).unwrap(),
        PathBuf::from("/p/ISCC")
    );
    // Then %ProgramFiles(x86)%, then %LOCALAPPDATA%\Programs.
    let pf = t.path().join("pf");
    let la = t.path().join("la");
    for base in [&pf, &la.join("Programs")] {
        fs::create_dir_all(base.join("Inno Setup 6")).unwrap();
        fs::write(base.join("Inno Setup 6/ISCC.exe"), "").unwrap();
    }
    let env = [
        ("ProgramFiles(x86)", pf.display().to_string()),
        ("LOCALAPPDATA", la.display().to_string()),
    ];
    assert_eq!(
        find_iscc(&none, &env_of(&env)).unwrap(),
        pf.join("Inno Setup 6/ISCC.exe")
    );
    let env = [("LOCALAPPDATA", la.display().to_string())];
    assert_eq!(
        find_iscc(&none, &env_of(&env)).unwrap(),
        la.join("Programs/Inno Setup 6/ISCC.exe")
    );
    assert_eq!(
        find_iscc(&none, &env_of(&[])).unwrap_err().to_string(),
        "ISCC.exe not found; install Inno Setup (bootstrap group packaging) or set BK_ISCC"
    );
}

#[test]
fn cargo_bk_suffix_is_ignored_against_the_pin() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path());
    let r = linux_runner().with_output("cargo-bk", 0, "0.3.0+2.gabc1234\n");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    assert!(matches!(find(&checks, "cargo-bk").status, Status::Ok(_)));
    let r = linux_runner().with_output("cargo-bk", 0, "0.3.0-dev\n");
    let checks = run_checks(Some(&ws), &r, Os::Linux);
    assert!(matches!(find(&checks, "cargo-bk").status, Status::Ok(_)));
}

#[test]
fn malformed_manifest_is_reported_not_ignored() {
    let t = tempfile::tempdir().unwrap();
    fs::write(
        t.path().join("Cargo.toml"),
        "[workspace
",
    )
    .unwrap();
    assert_cmd::Command::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(t.path())
        .arg("doctor")
        .assert()
        .code(1)
        .stderr(predicates::str::contains("cannot parse"));
}
