use assert_cmd::Command as Cmd;
use cargo_bk::commands::link::{link, restore_dep, rewrite_dep, unlink};
use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::exec::FakeRunner;
use cargo_bk::workspace::Workspace;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MARK_START: &str = "# >>> cargo bk link (remove with cargo bk unlink)";
const MARK_END: &str = "# <<< cargo bk link";
const URL: &str = "https://github.com/medaminben/buildkit-rust";

/// A fake checkout and a product folder inside a parent that owns `.cargo/`.
fn setup(checkout_name: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let checkout = tmp.path().join(checkout_name);
    fs::create_dir_all(checkout.join("crates/buildkit")).unwrap();
    fs::write(checkout.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    let product = tmp.path().join("product");
    fs::create_dir_all(&product).unwrap();
    (tmp, product, checkout)
}

fn config(product: &Path) -> PathBuf {
    product.parent().unwrap().join(".cargo/config.toml")
}

#[test]
fn link_writes_the_marked_block_with_the_absolute_path() {
    let (_t, product, checkout) = setup("buildkit-rust");
    link(&product, &checkout, None, false).unwrap();
    let text = fs::read_to_string(config(&product)).unwrap();
    let abs = fs::canonicalize(&checkout).unwrap();
    let abs = abs.to_string_lossy().replace('\\', "/");
    let abs = abs.trim_start_matches("//?/");
    assert_eq!(
        text,
        format!(
            "{MARK_START}\n[patch.\"{URL}\"]\nbuildkit = {{ path = \"{abs}/crates/buildkit\" }}\n{MARK_END}\n"
        )
    );
    let doc: toml_edit::DocumentMut = text.parse().unwrap();
    assert!(doc["patch"][URL]["buildkit"]["path"].is_str());
}

#[test]
fn second_link_replaces_the_block() {
    let (t, product, checkout) = setup("one");
    let other = t.path().join("two");
    fs::create_dir_all(other.join("crates/buildkit")).unwrap();
    fs::write(other.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    link(&product, &checkout, None, false).unwrap();
    link(&product, &other, None, false).unwrap();
    let text = fs::read_to_string(config(&product)).unwrap();
    assert_eq!(text.matches(MARK_START).count(), 1);
    assert!(text.contains("/two/crates/buildkit"));
    assert!(!text.contains("/one/crates/buildkit"));
}

#[test]
fn other_content_is_kept_and_unlink_restores_the_bytes() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let original = "[build]\njobs = 4\n\n# note\n";
    fs::write(&cfg, original).unwrap();
    link(&product, &checkout, None, false).unwrap();
    link(&product, &checkout, None, false).unwrap();
    let linked = fs::read_to_string(&cfg).unwrap();
    assert!(linked.starts_with(original));
    assert!(unlink(&product).unwrap());
    assert_eq!(fs::read_to_string(&cfg).unwrap(), original);
}

#[test]
fn unlink_deletes_a_file_it_empties_and_tolerates_none() {
    let (_t, product, checkout) = setup("buildkit-rust");
    assert!(!unlink(&product).unwrap());
    link(&product, &checkout, None, false).unwrap();
    assert!(unlink(&product).unwrap());
    assert!(!config(&product).exists());
}

#[test]
fn wrong_path_errors() {
    let (t, product, _c) = setup("buildkit-rust");
    let bad = t.path().join("nope");
    let e = link(&product, &bad, None, false).unwrap_err().to_string();
    assert_eq!(
        e,
        format!(
            "{} is not a buildkit-rust checkout (crates/buildkit/Cargo.toml missing)",
            bad.display()
        )
    );
    assert!(!config(&product).exists());
}

#[test]
fn existing_patch_table_is_refused_and_file_untouched() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let original = format!("[patch.\"{URL}\"]\nbuildkit = {{ path = \"/x\" }}\n");
    fs::write(&cfg, &original).unwrap();
    let e = link(&product, &checkout, None, false)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("already has [patch.") && e.contains("config.toml"),
        "{e}"
    );
    assert_eq!(fs::read_to_string(&cfg).unwrap(), original);
}

#[test]
fn invalid_existing_config_is_refused() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    fs::write(&cfg, "[build\n").unwrap();
    assert!(link(&product, &checkout, None, false).is_err());
    assert_eq!(fs::read_to_string(&cfg).unwrap(), "[build\n");
}

#[test]
fn home_parent_needs_force() {
    let (t, product, checkout) = setup("buildkit-rust");
    let e = link(&product, &checkout, Some(t.path()), false)
        .unwrap_err()
        .to_string();
    assert!(e.contains("home folder") && e.contains("--force"), "{e}");
    assert!(!config(&product).exists());
    link(&product, &checkout, Some(t.path()), true).unwrap();
    assert!(config(&product).exists());
}

#[test]
fn root_parent_is_refused_even_with_force() {
    let (_t, _p, checkout) = setup("buildkit-rust");
    let root = Path::new("/").join("product");
    let e = link(&root, &checkout, None, true).unwrap_err().to_string();
    assert!(e.contains("root folder"), "{e}");
}

#[test]
fn unterminated_block_errors_and_leaves_the_file() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let broken = format!("[build]\njobs = 1\n{MARK_START}\n[patch.\"{URL}\"]\n");
    fs::write(&cfg, &broken).unwrap();
    for e in [
        unlink(&product).unwrap_err().to_string(),
        link(&product, &checkout, None, false)
            .unwrap_err()
            .to_string(),
    ] {
        assert!(e.contains("config.toml:3"), "{e}");
    }
    assert_eq!(fs::read_to_string(&cfg).unwrap(), broken);
}

#[test]
fn non_utf8_config_is_an_error_not_empty() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    fs::write(&cfg, [0xff, 0xfe, 0x00]).unwrap();
    assert!(link(&product, &checkout, None, false).is_err());
    assert!(unlink(&product).is_err());
    assert_eq!(fs::read(&cfg).unwrap(), [0xff, 0xfe, 0x00]);
}

#[test]
fn original_without_trailing_newline_comes_back() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    fs::write(&cfg, "[build]\njobs = 4").unwrap();
    link(&product, &checkout, None, false).unwrap();
    link(&product, &checkout, None, false).unwrap();
    assert!(unlink(&product).unwrap());
    assert_eq!(fs::read_to_string(&cfg).unwrap(), "[build]\njobs = 4");
}

#[test]
fn lines_added_after_the_block_survive_unlink() {
    let (_t, product, checkout) = setup("buildkit-rust");
    let cfg = config(&product);
    fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    fs::write(&cfg, "[build]\njobs = 4").unwrap();
    link(&product, &checkout, None, false).unwrap();
    let mut text = fs::read_to_string(&cfg).unwrap();
    text.push_str("after = 1\n");
    fs::write(&cfg, text).unwrap();
    assert!(unlink(&product).unwrap());
    let left = fs::read_to_string(&cfg).unwrap();
    assert_eq!(left, "[build]\njobs = 4\nafter = 1\n");
    assert!(left.parse::<toml_edit::DocumentMut>().is_ok());
}

#[test]
fn missing_dependency_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("prod");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\n[workspace.metadata.buildkit]\nversion = \"1.0.0\"\n[workspace.dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    let mut ws = Workspace::find(&dir).unwrap();
    let e = restore_dep(&mut ws).unwrap_err().to_string();
    assert_eq!(
        e,
        "buildkit dependency is missing from [workspace.dependencies]"
    );
}

#[test]
fn table_form_dependency_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("prod");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[workspace]\n[workspace.metadata.buildkit]\nversion = \"1.0.0\"\n[workspace.dependencies.buildkit]\ngit = \"{URL}\"\ntag = \"v1.0.0\"\n"),
    )
    .unwrap();
    let mut ws = Workspace::find(&dir).unwrap();
    let e = restore_dep(&mut ws).unwrap_err().to_string();
    assert_eq!(
        e,
        "buildkit dependency is not an inline table; edit it by hand"
    );
}

#[test]
fn extra_dependency_keys_survive_the_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let checkout = tmp.path().join("bk");
    fs::create_dir_all(checkout.join("crates/buildkit")).unwrap();
    fs::write(checkout.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    let dir = tmp.path().join("prod");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[workspace]\n[workspace.metadata.buildkit]\nversion = \"1.0.0\"\n[workspace.dependencies]\nbuildkit = {{ git = \"{URL}\", tag = \"v1.0.0\", features = [\"x\"], default-features = false }}\n"),
    )
    .unwrap();
    let mut ws = Workspace::find(&dir).unwrap();
    rewrite_dep(&mut ws, &checkout).unwrap();
    let mid = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        mid.contains("features = [\"x\"]")
            && mid.contains("default-features = false")
            && mid.contains("path ="),
        "{mid}"
    );
    let mut ws = Workspace::find(&dir).unwrap();
    restore_dep(&mut ws).unwrap();
    let end = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        end.contains("features = [\"x\"]") && end.contains("default-features = false"),
        "{end}"
    );
    assert!(
        end.contains(&format!("git = \"{URL}\""))
            && end.contains("tag = \"v1.0.0\"")
            && !end.contains("path ="),
        "{end}"
    );
}

fn product_manifest(dir: &Path) -> Workspace {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = []\n\n[workspace.metadata.buildkit]\nversion = \"0.3.1\"\n\n[workspace.dependencies]\n# keep\nbuildkit = {{ git = \"{URL}\", tag = \"v0.3.1\" }}\nserde = \"1\"\n"
        ),
    )
    .unwrap();
    Workspace::find(dir).unwrap()
}

#[test]
fn rewrite_dep_and_restore_round_trip_with_spaces() {
    let tmp = tempfile::tempdir().unwrap();
    let checkout = tmp.path().join("my build kit/buildkit-rust");
    fs::create_dir_all(checkout.join("crates/buildkit")).unwrap();
    fs::write(checkout.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    let dir = tmp.path().join("prod");
    let mut ws = product_manifest(&dir);
    let original = fs::read_to_string(dir.join("Cargo.toml")).unwrap();

    rewrite_dep(&mut ws, &checkout).unwrap();
    let text = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(
        text.contains("my build kit/buildkit-rust/crates/buildkit\" }"),
        "{text}"
    );
    assert!(!text.contains("git ="));
    assert!(text.contains("# keep") && text.contains("serde = \"1\""));
    let doc: toml_edit::DocumentMut = text.parse().unwrap();
    assert!(doc["workspace"]["dependencies"]["buildkit"]["path"].is_str());

    let mut ws = Workspace::find(&dir).unwrap();
    restore_dep(&mut ws).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
        original
    );
}

#[test]
fn rewrite_dep_rejects_a_wrong_checkout() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = product_manifest(&tmp.path().join("prod"));
    let e = rewrite_dep(&mut ws, &tmp.path().join("nope")).unwrap_err();
    assert!(e.to_string().contains("is not a buildkit-rust checkout"));
}

#[test]
fn cli_rewrite_dep_warns_and_unlink_restores() {
    let tmp = tempfile::tempdir().unwrap();
    let checkout = tmp.path().join("a b/buildkit-rust");
    fs::create_dir_all(checkout.join("crates/buildkit")).unwrap();
    fs::write(checkout.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    let dir = tmp.path().join("prod");
    product_manifest(&dir);
    let original = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    let out = Cmd::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(&dir)
        .args(["link", "--rewrite-dep"])
        .arg(&checkout)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        all.lines().any(|l| l
            == "Cargo.toml now uses a local buildkit path; do not commit this change (cargo bk unlink --rewrite-dep restores it)"),
        "{all}"
    );
    assert!(all.contains("cargo install --path"));
    assert!(!tmp.path().join(".cargo/config.toml").exists());
    Cmd::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(&dir)
        .args(["unlink", "--rewrite-dep"])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
        original
    );
}

#[test]
fn cli_link_defaults_to_the_sibling_checkout() {
    let tmp = tempfile::tempdir().unwrap();
    let sib = tmp.path().join("buildkit-rust");
    fs::create_dir_all(sib.join("crates/buildkit")).unwrap();
    fs::write(sib.join("crates/buildkit/Cargo.toml"), "[package]\n").unwrap();
    let dir = tmp.path().join("prod");
    product_manifest(&dir);
    Cmd::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(&dir)
        .arg("link")
        .assert()
        .success();
    assert!(
        fs::read_to_string(tmp.path().join(".cargo/config.toml"))
            .unwrap()
            .contains(MARK_START)
    );
    Cmd::cargo_bin("cargo-bk")
        .unwrap()
        .current_dir(&dir)
        .arg("unlink")
        .assert()
        .success();
    assert!(!tmp.path().join(".cargo/config.toml").exists());
}

#[test]
#[ignore = "builds a generated product with a real cargo"]
fn rewritten_product_builds_against_the_local_crate() {
    // The parent-config patch cannot be built until the repository and tag are
    // published, so this exercises --rewrite-dep.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("My Projékt dir").join("foo");
    let a = NewArgs {
        name: None,
        crates: vec!["core".to_string()],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
        fixed_guid: None,
        fixed_year: None,
        fixed_bk_version: None,
    };
    new_project(&dir, &a, &FakeRunner::new()).unwrap();
    let checkout = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let mut ws = Workspace::find(&dir).unwrap();
    rewrite_dep(&mut ws, &checkout).unwrap();
    let out = Command::new("cargo")
        .args(["test", "-q"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", tmp.path().join("target"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
