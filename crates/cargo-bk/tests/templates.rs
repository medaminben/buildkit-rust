use std::collections::BTreeMap;
use std::path::Path;

use cargo_bk::templates::{Vars, render, render_path, write_tree};
use include_dir::{Dir, include_dir};

static FIXTURE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/templates");

fn vars() -> Vars {
    let mut m = BTreeMap::new();
    m.insert("name", "My Name".to_string());
    m.insert("kebab", "x".to_string());
    m.insert("crate", "core".to_string());
    Vars(m)
}

#[test]
fn render_replaces_known_key() {
    assert_eq!(render("a {{kebab}} b", &vars()).unwrap(), "a x b");
}

#[test]
fn render_unknown_key_errors_naming_it() {
    let err = render("a {{nope}}", &vars()).unwrap_err();
    assert!(err.to_string().contains("nope"), "{err}");
}

#[test]
fn render_path_replaces_segments_and_strips_tmpl() {
    let p = render_path(Path::new("crates/__crate__/Cargo.toml.tmpl"), &vars()).unwrap();
    assert_eq!(p, Path::new("crates/core/Cargo.toml"));
}

#[test]
fn render_path_unknown_key_errors() {
    let err = render_path(Path::new("__nope__/a"), &vars()).unwrap_err();
    assert!(err.to_string().contains("nope"), "{err}");
}

#[test]
fn write_tree_creates_then_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let first = write_tree(&FIXTURE, dir.path(), &vars()).unwrap();
    assert_eq!(first.created.len(), 3, "{first:?}");
    assert_eq!(first.kept.len(), 0);

    let cargo = dir.path().join("sub/core/Cargo.toml");
    let text = std::fs::read_to_string(&cargo).unwrap();
    assert_eq!(text, "name=My Name\nkebab=x\n");
    assert!(!dir.path().join("sub/.keep").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("plain.txt")).unwrap(),
        "plain\nline\n"
    );
    assert_eq!(
        std::fs::read(dir.path().join("icon.png")).unwrap(),
        b"\x89PNG\xff\xfe{{name}}"
    );

    std::fs::write(&cargo, "edited\r\n").unwrap();
    let second = write_tree(&FIXTURE, dir.path(), &vars()).unwrap();
    assert_eq!(second.created.len(), 0, "{second:?}");
    assert_eq!(second.kept.len(), 3);
    assert_eq!(std::fs::read(&cargo).unwrap(), b"edited\r\n");
}

#[test]
fn write_tree_error_names_template_path() {
    static BAD: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/bad");
    let dir = tempfile::tempdir().unwrap();
    let err = write_tree(&BAD, dir.path(), &vars()).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("nope") && msg.contains("bad.tmpl"), "{msg}");
}
