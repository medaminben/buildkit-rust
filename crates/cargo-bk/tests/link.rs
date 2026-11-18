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
    link(&product, &checkout).unwrap();
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
    link(&product, &checkout).unwrap();
    link(&product, &other).unwrap();
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
    link(&product, &checkout).unwrap();
    link(&product, &checkout).unwrap();
    let linked = fs::read_to_string(&cfg).unwrap();
    assert!(linked.starts_with(original));
    assert!(unlink(&product).unwrap());
    assert_eq!(fs::read_to_string(&cfg).unwrap(), original);
}

#[test]
fn unlink_deletes_a_file_it_empties_and_tolerates_none() {
    let (_t, product, checkout) = setup("buildkit-rust");
    assert!(!unlink(&product).unwrap());
    link(&product, &checkout).unwrap();
    assert!(unlink(&product).unwrap());
    assert!(!config(&product).exists());
}

#[test]
fn wrong_path_errors() {
    let (t, product, _c) = setup("buildkit-rust");
    let bad = t.path().join("nope");
    let e = link(&product, &bad).unwrap_err().to_string();
    assert_eq!(
        e,
        format!(
            "{} is not a buildkit-rust checkout (crates/buildkit/Cargo.toml missing)",
            bad.display()
        )
    );
    assert!(!config(&product).exists());
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
    assert!(all.contains(
        "Cargo.toml now uses a local buildkit path; do not commit this change (cargo bk unlink --rewrite-dep restores it)"
    ));
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
