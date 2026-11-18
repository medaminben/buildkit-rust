use cargo_bk::exec::{Cmd, FakeRunner, SystemRunner};
use cargo_bk::package::PackageCtx;
use cargo_bk::package::appimage::{
    APPIMAGETOOL_SHA256, APPIMAGETOOL_URL, build_appimage, fetch_tool,
};
use cargo_bk::workspace::PackageMeta;
use std::fs;
use std::path::Path;

fn meta() -> PackageMeta {
    PackageMeta {
        display_name: "Foo Bar".into(),
        main_app: "foo-bar-gui".into(),
        apps: vec!["foo-bar-gui".into(), "foo-bar-console".into()],
        iss: "packaging/foo-bar.iss".into(),
        desktop: "packaging/foo-bar.desktop".into(),
        icon: "packaging/icon.png".into(),
        extra_files: vec![],
        system_deps_apt: vec![],
    }
}

fn setup(root: &Path) {
    for (rel, data) in [
        ("packaging/foo-bar.desktop", "[Desktop Entry]\n"),
        ("packaging/icon.png", "png"),
        ("build/stage/foo-bar-gui", "bin"),
        ("build/stage/foo-bar-console", "bin"),
    ] {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
}

fn version() -> buildkit::Version {
    buildkit::Version {
        numeric: "1.2.3".into(),
        full: "1.2.3-beta.4".into(),
    }
}

#[test]
fn constants_equal_the_bootstrap() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bootstrap");
    let sh = fs::read_to_string(dir.join("bootstrap.sh")).unwrap();
    assert!(sh.contains(&format!("APPIMAGETOOL_URL=\"{APPIMAGETOOL_URL}\"")));
    assert!(sh.contains(&format!("APPIMAGETOOL_SHA256=\"{APPIMAGETOOL_SHA256}\"")));
    let ps = fs::read_to_string(dir.join("bootstrap.ps1")).unwrap();
    if ps.contains("appimagetool") {
        assert!(ps.contains(APPIMAGETOOL_URL) && ps.contains(APPIMAGETOOL_SHA256));
    }
}

#[cfg(unix)]
#[test]
fn layout_symlink_args_and_env() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    setup(root);
    // leftovers of an earlier run must go
    fs::create_dir_all(root.join("build/appdir/stale")).unwrap();
    let r = FakeRunner::new()
        .with_tool("appimagetool", "/opt/appimagetool")
        .with_output("/opt/appimagetool", 0, "");
    let (m, v) = (meta(), version());
    let dist = root.join("build/dist");
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
    let path = build_appimage(&ctx, &r).unwrap();
    assert_eq!(path, dist.join("Foo-Bar-1.2.3-beta.4-x86_64.AppImage"));

    let ad = root.join("build/appdir");
    assert!(!ad.join("stale").exists());
    assert!(ad.join("usr/bin/foo-bar-gui").is_file());
    assert!(ad.join("usr/bin/foo-bar-console").is_file());
    assert!(ad.join("foo-bar.desktop").is_file());
    assert!(ad.join("foo-bar.png").is_file());
    assert_eq!(
        fs::read_link(ad.join("AppRun")).unwrap(),
        Path::new("usr/bin/foo-bar-gui")
    );

    let calls = r.calls();
    assert_eq!(calls.len(), 1);
    let c: &Cmd = &calls[0];
    assert_eq!(c.program, "/opt/appimagetool");
    assert_eq!(
        c.args,
        [ad.display().to_string(), path.display().to_string()]
    );
    assert!(
        c.env
            .contains(&("APPIMAGE_EXTRACT_AND_RUN".into(), "1".into()))
    );
    assert!(c.env.contains(&("ARCH".into(), "x86_64".into())));
}

#[test]
fn sha_mismatch_errors_and_deletes_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("tools/appimagetool-x86_64.AppImage");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(&dest, "not the tool").unwrap();
    let r = FakeRunner::new().with_output("curl", 0, "");
    let e = fetch_tool(&r, &dest, "https://example.invalid/x", APPIMAGETOOL_SHA256).unwrap_err();
    assert!(e.to_string().contains("SHA-256"), "{e}");
    assert!(!dest.exists());
    let calls = r.calls();
    assert_eq!(calls[0].program, "curl");
    assert_eq!(
        calls[0].args[..4],
        [
            "-fsSL",
            "-o",
            &dest.display().to_string(),
            "https://example.invalid/x"
        ]
        .map(String::from)
    );
}

#[test]
fn matching_sha_keeps_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("tool");
    fs::write(&dest, "abc").unwrap();
    let r = FakeRunner::new().with_output("curl", 0, "");
    // SHA-256 of "abc"
    let sha = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    fetch_tool(&r, &dest, "https://example.invalid/x", sha).unwrap();
    assert!(dest.is_file());
}

#[test]
fn download_failure_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("tool");
    let r = FakeRunner::new().with_failure("curl", 22, "404");
    let e = fetch_tool(&r, &dest, "u", "00").unwrap_err();
    assert!(e.to_string().contains("404"));
}

#[cfg(unix)]
#[test]
#[ignore = "builds a generated product with eframe and runs the real appimagetool; takes minutes"]
fn real_appimage_of_a_generated_product() {
    use buildkit::SystemGit;
    use cargo_bk::commands::link::rewrite_dep;
    use cargo_bk::commands::new::{Flow, NewArgs, new_project};
    use cargo_bk::commands::package;
    use cargo_bk::os::Os;
    use cargo_bk::workspace::Workspace;
    use std::process::Command;

    let tmp = tempfile::tempdir().unwrap();
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
    new_project(tmp.path(), &args, &SystemRunner).unwrap();
    let mut ws = Workspace::find(tmp.path()).unwrap();
    rewrite_dep(
        &mut ws,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )
    .unwrap();
    ws.save().unwrap();
    let ws = Workspace::find(tmp.path()).unwrap();
    assert!(
        Command::new("cargo")
            .arg("generate-lockfile")
            .current_dir(tmp.path())
            .status()
            .unwrap()
            .success()
    );

    // no --format: AppImage and deb both
    let built = package::run(
        &ws,
        None,
        Os::Linux,
        &SystemRunner,
        &SystemGit,
        &mut Vec::new(),
        &mut std::io::stderr(),
    )
    .unwrap();
    println!("{built:?}");
    assert_eq!(built.len(), 2);
    let img = &built[0];
    assert!(img.to_string_lossy().ends_with("-x86_64.AppImage"));
    assert!(built[1].to_string_lossy().ends_with(".deb"));

    let ex = tempfile::tempdir().unwrap();
    let st = Command::new(img)
        .arg("--appimage-extract")
        .current_dir(ex.path())
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "{}",
        String::from_utf8_lossy(&st.stderr)
    );
    println!("{}", String::from_utf8_lossy(&st.stdout));
    let bin = ex.path().join("squashfs-root/usr/bin");
    for f in ["foo-gui", "foo-console", "THIRD_PARTY_NOTICES.txt"] {
        assert!(bin.join(f).is_file(), "the AppImage lacks usr/bin/{f}");
    }
}
