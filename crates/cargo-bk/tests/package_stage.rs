use cargo_bk::exec::FakeRunner;
use cargo_bk::os::Os;
use cargo_bk::package::notices::notices;
use cargo_bk::package::stage::{apply_stage, plan_stage};
use cargo_bk::package::{Format, formats_for};
use cargo_bk::workspace::Workspace;
use std::fs;
use std::path::{Path, PathBuf};

fn product(root: &Path, extra: &str, apps: &[&str]) -> Workspace {
    let list = apps
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "[workspace]\nmembers = []\n[workspace.metadata.bk.package]\n\
         display-name = \"P\"\nmain-app = \"{}\"\napps = [{list}]\n\
         iss = \"packaging/p.iss\"\ndesktop = \"packaging/p.desktop\"\n\
         icon = \"packaging/p.png\"\nextra-files = [{extra}]\n",
        apps[0]
    );
    Workspace {
        root: root.to_path_buf(),
        doc: text.parse().unwrap(),
    }
}

fn touch(p: &Path) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, "x").unwrap();
}

#[test]
fn formats_default_per_os() {
    assert_eq!(
        formats_for(None, Os::Linux).unwrap(),
        vec![Format::AppImage, Format::Deb]
    );
    assert_eq!(
        formats_for(Some("all"), Os::Windows).unwrap(),
        vec![Format::Inno]
    );
    assert_eq!(
        formats_for(Some("deb"), Os::Linux).unwrap(),
        vec![Format::Deb]
    );
}

#[test]
fn inno_on_linux_is_an_error() {
    let e = formats_for(Some("inno"), Os::Linux).unwrap_err();
    assert!(
        e.to_string()
            .contains("inno installers are built on Windows")
    );
}

#[test]
fn plan_includes_binary_license_and_extra_globs() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path();
    touch(&root.join("build/target/release/app"));
    touch(&root.join("assets/a/b.png"));
    touch(&root.join("LICENSE"));
    let ws = product(root, "\"assets/**\"", &["app"]);
    let plan = plan_stage(&ws, &ws.package_meta().unwrap(), Os::Linux).unwrap();
    let dests: Vec<_> = plan.files.iter().map(|(_, d)| d.clone()).collect();
    assert!(dests.contains(&PathBuf::from("app")));
    assert!(dests.contains(&PathBuf::from("LICENSE")));
    assert!(dests.contains(&PathBuf::from("assets/a/b.png")));
}

#[test]
fn windows_binary_has_exe() {
    let t = tempfile::tempdir().unwrap();
    touch(&t.path().join("build/target/release/app.exe"));
    let ws = product(t.path(), "", &["app"]);
    let plan = plan_stage(&ws, &ws.package_meta().unwrap(), Os::Windows).unwrap();
    assert!(plan.files.iter().any(|(_, d)| d == Path::new("app.exe")));
}

#[test]
fn missing_binary_is_an_error() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path(), "", &["app"]);
    let e = plan_stage(&ws, &ws.package_meta().unwrap(), Os::Linux).unwrap_err();
    assert_eq!(
        e.to_string(),
        "app app listed in [workspace.metadata.bk.package] apps has no binary"
    );
}

#[test]
fn glob_escaping_the_root_is_rejected() {
    let t = tempfile::tempdir().unwrap();
    touch(&t.path().join("build/target/release/app"));
    let ws = product(t.path(), "\"../secret/*\"", &["app"]);
    assert!(plan_stage(&ws, &ws.package_meta().unwrap(), Os::Linux).is_err());
}

#[test]
fn root_with_space_and_accent_stages() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("Bowling Box é");
    touch(&root.join("build/target/release/app"));
    touch(&root.join("assets/x.png"));
    let ws = product(&root, "\"assets/*\"", &["app"]);
    let plan = plan_stage(&ws, &ws.package_meta().unwrap(), Os::Linux).unwrap();
    let stage = root.join("build/stage");
    touch(&stage.join("stale"));
    apply_stage(&plan, &stage).unwrap();
    assert!(stage.join("app").is_file());
    assert!(stage.join("assets/x.png").is_file());
    assert!(!stage.join("stale").exists());
}

#[test]
fn apply_refuses_stage_dir_outside_build() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("precious");
    touch(&dir.join("keep"));
    let plan = cargo_bk::package::stage::StagePlan { files: vec![] };
    assert!(apply_stage(&plan, &dir).is_err());
    assert!(dir.join("keep").exists());
    assert!(apply_stage(&plan, &t.path().join("build")).is_err());
}

#[test]
fn notices_join_crate_and_extra_text() {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir_all(t.path().join("packaging")).unwrap();
    fs::write(t.path().join("packaging/NOTICES-extra.txt"), "font X").unwrap();
    let ws = product(t.path(), "", &["app"]);
    let r = FakeRunner::new().with_output("cargo", 0, "crate notices");
    assert_eq!(notices(&ws, &r).unwrap(), "crate notices\n\nfont X\n");
    let c = &r.calls()[0];
    assert_eq!(
        c.args,
        ["about", "generate", "--config", "about.toml", "about.hbs"]
    );
    assert_eq!(c.cwd.as_deref(), Some(t.path()));
}

#[test]
fn notices_fail_when_cargo_about_fails() {
    let t = tempfile::tempdir().unwrap();
    let ws = product(t.path(), "", &["app"]);
    let r = FakeRunner::new().with_output("cargo", 1, "");
    assert!(notices(&ws, &r).is_err());
}
