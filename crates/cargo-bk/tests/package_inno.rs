use cargo_bk::exec::{FakeRunner, SystemRunner};
use cargo_bk::package::PackageCtx;
use cargo_bk::package::inno::{build_inno, iscc_args};
use cargo_bk::workspace::PackageMeta;
use std::fs;
use std::path::Path;

fn meta() -> PackageMeta {
    PackageMeta {
        display_name: "Foo Bar".into(),
        main_app: "foo-bar-gui".into(),
        apps: vec!["foo-bar-gui".into()],
        iss: "installer/foo-bar.iss".into(),
        desktop: "packaging/foo-bar.desktop".into(),
        icon: "packaging/icon.png".into(),
        extra_files: vec![],
        system_deps_apt: vec![],
    }
}

fn version() -> buildkit::Version {
    buildkit::Version {
        numeric: "1.2.3".into(),
        full: "1.2.3-beta.4".into(),
    }
}

#[test]
fn args_keep_paths_with_spaces_whole() {
    let args = iscc_args(
        Path::new("/r/installer/foo bar.iss"),
        Path::new("/r/build/my stage"),
        Path::new("/r/build/dist"),
        "1.2.3-beta.4",
        "1.2.3",
        "Foo-Bar",
    );
    assert_eq!(
        args,
        [
            "/DAppVersion=1.2.3-beta.4",
            "/DAppVersionNumeric=1.2.3",
            "/DStageDir=/r/build/my stage",
            "/O/r/build/dist",
            "/FFoo-Bar_Setup_1.2.3-beta.4",
            "/r/installer/foo bar.iss",
        ]
    );
}

#[test]
fn builds_versioned_and_fixed_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let dist = root.join("build/dist");
    fs::create_dir_all(&dist).unwrap();
    // ISCC would write this; the fake runner does not.
    fs::write(dist.join("Foo-Bar_Setup_1.2.3-beta.4.exe"), "setup").unwrap();
    let exe = tmp.path().join("ISCC.exe");
    fs::write(&exe, "").unwrap();
    let r = FakeRunner::new().with_output(&exe.display().to_string(), 0, "");
    let (m, v) = (meta(), version());
    let stage = root.join("build/stage");
    let ctx = PackageCtx {
        root,
        stage: &stage,
        out: &dist,
        meta: &m,
        kebab: "foo-bar",
        maintainer: "acme",
        version: &v,
    };
    let env = |k: &str| (k == "BK_ISCC").then(|| exe.display().to_string());
    let files = build_inno(&ctx, &r, &env).unwrap();
    assert_eq!(
        files,
        [
            dist.join("Foo-Bar_Setup_1.2.3-beta.4.exe"),
            dist.join("Foo-Bar_Setup.exe")
        ]
    );
    assert_eq!(fs::read_to_string(&files[1]).unwrap(), "setup");
    let calls = r.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].args,
        iscc_args(
            &root.join("installer/foo-bar.iss"),
            &stage,
            &dist,
            "1.2.3-beta.4",
            "1.2.3",
            "Foo-Bar"
        )
    );
}

#[test]
fn iscc_failure_and_missing_output_are_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let dist = root.join("build/dist");
    let (m, v) = (meta(), version());
    let stage = root.join("build/stage");
    let ctx = PackageCtx {
        root,
        stage: &stage,
        out: &dist,
        meta: &m,
        kebab: "foo-bar",
        maintainer: "acme",
        version: &v,
    };
    let exe = tmp.path().join("ISCC.exe");
    fs::write(&exe, "").unwrap();
    let env = |k: &str| (k == "BK_ISCC").then(|| exe.display().to_string());
    let bad = FakeRunner::new().with_failure(&exe.display().to_string(), 1, "boom\n");
    let e = build_inno(&ctx, &bad, &env).unwrap_err().to_string();
    assert_eq!(e, "ISCC failed: boom");
    let ok = FakeRunner::new().with_output(&exe.display().to_string(), 0, "");
    let e = build_inno(&ctx, &ok, &env).unwrap_err().to_string();
    assert!(e.contains("Foo-Bar_Setup_1.2.3-beta.4.exe"), "{e}");
}

#[test]
#[ignore = "needs Inno Setup on Windows"]
fn real_build_produces_both_exes() {
    if !cfg!(windows) {
        eprintln!("skipped: ISCC.exe only runs on Windows");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let stage = root.join("build/stage");
    fs::create_dir_all(&stage).unwrap();
    fs::write(stage.join("a.txt"), "x").unwrap();
    fs::create_dir_all(root.join("installer")).unwrap();
    fs::write(
        root.join("installer/foo-bar.iss"),
        "[Setup]\nAppName=Foo Bar\nAppVersion={#AppVersion}\nDefaultDirName={autopf}\\Foo\n\
         [Files]\nSource: \"{#StageDir}\\*\"; DestDir: \"{app}\"\n",
    )
    .unwrap();
    let dist = root.join("build/dist");
    let (m, v) = (meta(), version());
    let ctx = PackageCtx {
        root,
        stage: &stage,
        out: &dist,
        meta: &m,
        kebab: "foo-bar",
        maintainer: "acme",
        version: &v,
    };
    let env = |k: &str| std::env::var(k).ok();
    let files = build_inno(&ctx, &SystemRunner, &env).unwrap();
    assert!(files.iter().all(|f| f.is_file()));
}

#[test]
fn dev_version_keeps_the_plus() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let dist = root.join("build/dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("Foo-Bar_Setup_1.2.3+2.gabc1234.exe"), "setup").unwrap();
    let exe = tmp.path().join("ISCC.exe");
    fs::write(&exe, "").unwrap();
    let r = FakeRunner::new().with_output(&exe.display().to_string(), 0, "");
    let m = meta();
    let v = buildkit::Version {
        numeric: "1.2.3".into(),
        full: "1.2.3+2.gabc1234".into(),
    };
    let stage = root.join("build/stage");
    let ctx = PackageCtx {
        root,
        stage: &stage,
        out: &dist,
        meta: &m,
        kebab: "foo-bar",
        maintainer: "acme",
        version: &v,
    };
    let env = |k: &str| (k == "BK_ISCC").then(|| exe.display().to_string());
    let files = build_inno(&ctx, &r, &env).unwrap();
    assert_eq!(
        files,
        [
            dist.join("Foo-Bar_Setup_1.2.3+2.gabc1234.exe"),
            dist.join("Foo-Bar_Setup.exe")
        ]
    );
    let args = &r.calls()[0].args;
    assert!(args.contains(&"/FFoo-Bar_Setup_1.2.3+2.gabc1234".to_string()));
    assert!(args.contains(&"/DAppVersion=1.2.3+2.gabc1234".to_string()));
}

#[test]
fn iscc_failure_shows_stdout_when_stderr_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let exe = tmp.path().join("ISCC.exe");
    fs::write(&exe, "").unwrap();
    let r = FakeRunner::new().with_output(&exe.display().to_string(), 1, "Error on line 3\n");
    let (m, v) = (meta(), version());
    let stage = root.join("build/stage");
    let dist = root.join("build/dist");
    let ctx = PackageCtx {
        root,
        stage: &stage,
        out: &dist,
        meta: &m,
        kebab: "foo-bar",
        maintainer: "acme",
        version: &v,
    };
    let env = |k: &str| (k == "BK_ISCC").then(|| exe.display().to_string());
    let e = build_inno(&ctx, &r, &env).unwrap_err().to_string();
    assert_eq!(e, "ISCC failed: Error on line 3");
}
