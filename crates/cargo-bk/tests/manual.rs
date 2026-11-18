use assert_cmd::Command;
use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use cargo_bk::os::Os;
use cargo_bk::python::find_python;
use std::fs;
use std::path::Path;

#[test]
fn find_python_falls_back_to_python_when_python3_is_missing() {
    let r = FakeRunner::new().with_output("python", 0, "Python 3.12.1\n");
    let cmd = find_python(&r, Os::Linux).unwrap();
    assert_eq!(cmd.program, "python");
    assert_eq!(cmd.args.len(), 0);
    assert_eq!(r.calls().len(), 2);
}

#[test]
fn find_python_rejects_3_10() {
    let r = FakeRunner::new()
        .with_output("python3", 0, "Python 3.10.12\n")
        .with_output("python", 0, "Python 3.10.1\n");
    let err = find_python(&r, Os::Linux).unwrap_err().to_string();
    assert_eq!(err, "Python 3.11+ not found; run bootstrap (group python)");
}

#[test]
fn find_python_takes_the_first_new_enough_one() {
    let r = FakeRunner::new()
        .with_output("python3", 0, "Python 3.11.0\n")
        .with_output("python", 0, "Python 3.13.0\n");
    assert_eq!(find_python(&r, Os::Linux).unwrap().program, "python3");
    let r = FakeRunner::new()
        .with_output("python3", 0, "Python 3.9.2\n")
        .with_output("python", 0, "Python 3.11.4\n");
    assert_eq!(find_python(&r, Os::Linux).unwrap().program, "python");
}

#[test]
fn find_python_tries_the_py_launcher_on_windows_only() {
    let r = FakeRunner::new().with_output("py", 0, "Python 3.12.0\n");
    let cmd = find_python(&r, Os::Windows).unwrap();
    assert_eq!(
        (cmd.program.as_str(), cmd.args.as_slice()),
        ("py", ["-3".to_string()].as_slice())
    );
    assert!(find_python(&r, Os::Linux).is_err());
}

#[test]
fn find_python_skips_a_failing_or_garbled_version_output() {
    let r = FakeRunner::new()
        .with_output("python3", 9, "Python 3.12.0\n")
        .with_output("python", 0, "not python\n");
    assert!(find_python(&r, Os::Linux).is_err());
}

fn product(dir: &Path) {
    let args = NewArgs {
        name: Some("Foo".to_string()),
        crates: vec!["core".to_string()],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
        fixed_guid: None,
        fixed_year: None,
        fixed_bk_version: None,
    };
    new_project(dir, &args, &FakeRunner::new()).unwrap();
}

fn bk(dir: &Path) -> Command {
    let mut c = Command::cargo_bin("cargo-bk").unwrap();
    c.current_dir(dir);
    c
}

#[test]
#[ignore = "needs python 3.11+"]
fn manual_check_and_build_work_on_a_generated_product() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let root = tmp.path();
    bk(root).args(["manual", "--check"]).assert().success();
    assert!(!root.join("build/manual").exists());
    assert!(root.join("build/tools/manual/build_manual.py").is_file());
    assert!(!root.join("build/tools/manual/capture.sh").exists());
    assert!(
        !root
            .join("build/tools/manual/test_build_manual.py")
            .exists()
    );
    bk(root).arg("manual").assert().success();
    assert!(root.join("build/manual/index.html").is_file());
}

#[test]
#[ignore = "needs python 3.11+"]
fn a_broken_link_fails_the_check() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let usage = tmp.path().join("docs/manual/content/en/usage.md");
    let mut text = fs::read_to_string(&usage).unwrap();
    text.push_str("\nSee [gone](no-such-page.md).\n");
    fs::write(&usage, text).unwrap();
    bk(tmp.path()).args(["manual", "--check"]).assert().code(1);
}

#[test]
#[ignore = "needs python 3.11+"]
fn manual_works_in_a_path_with_spaces() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("my product");
    fs::create_dir(&dir).unwrap();
    product(&dir);
    bk(&dir).args(["manual", "--check"]).assert().success();
}

#[test]
fn write_tool_removes_stale_files_and_skips_tests_and_capture() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("build/tools/manual");
    fs::create_dir_all(dest.join("old")).unwrap();
    fs::write(dest.join("stale.py"), "x").unwrap();
    fs::write(dest.join("old/deep.txt"), "x").unwrap();
    cargo_bk::commands::manual::write_tool(&dest).unwrap();
    assert!(!dest.join("stale.py").exists());
    assert!(!dest.join("old").exists());
    assert!(dest.join("build_manual.py").is_file());
    assert!(dest.join("template.html").is_file());
    assert!(!dest.join("test_build_manual.py").exists());
    assert!(!dest.join("capture.sh").exists());
    assert!(!dest.join("__pycache__").exists());
    cargo_bk::commands::manual::write_tool(&dest).unwrap();
}
