use assert_cmd::Command;
use cargo_bk::commands::add::{AppKind, add_app, add_crate};
use cargo_bk::commands::new::{Flow, NewArgs, new_project};
use cargo_bk::docs_rows::ensure_row;
use cargo_bk::exec::FakeRunner;
use cargo_bk::workspace::Workspace;
use std::fs;
use std::path::Path;

const MARKER: &str = "<!-- bk:layout -->";

fn product(dir: &Path) {
    let args = NewArgs {
        name: Some("Foo".to_string()),
        crates: vec!["core".to_string()],
        repo: None,
        flow: Flow::Direct,
        no_build: true,
    };
    new_project(dir, &args, &FakeRunner::new()).unwrap();
}

fn read(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(rel)).unwrap()
}

/// `text` with `row` added as a new line right after the line `after`.
fn insert_after(text: &str, after: &str, row: &str) -> String {
    text.replacen(&format!("{after}\n"), &format!("{after}\n{row}\n"), 1)
}

fn last_row(text: &str) -> &str {
    text.lines().rfind(|l| l.starts_with("| ")).unwrap()
}

#[test]
fn ensure_row_adds_after_the_last_row_and_returns_none_for_an_existing_row() {
    let md = format!("# T\n\n{MARKER}\n| A | B |\n| --- | --- |\n| x | y |\n\nAfter.\n");
    let out = ensure_row(&md, MARKER, "| z | w |").unwrap();
    assert_eq!(
        out,
        format!("# T\n\n{MARKER}\n| A | B |\n| --- | --- |\n| x | y |\n| z | w |\n\nAfter.\n")
    );
    assert_eq!(ensure_row(&out, MARKER, "| z | w |"), None);
    // The owner reworded the second cell; the row still counts as present.
    assert_eq!(ensure_row(&out, MARKER, "| z | other |"), None);
    assert_eq!(ensure_row("# T\n| x | y |\n", MARKER, "| z | w |"), None);
}

#[test]
fn ensure_row_handles_a_table_at_the_end_of_the_file_without_newline() {
    let md = format!("{MARKER}\n| A |\n| --- |\n| x |");
    assert_eq!(
        ensure_row(&md, MARKER, "| z |").unwrap(),
        format!("{MARKER}\n| A |\n| --- |\n| x |\n| z |\n")
    );
}

#[test]
fn new_fills_the_layout_rows_of_both_docs() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    for rel in ["README.md", "docs/ARCHITECTURE.md"] {
        let text = read(tmp.path(), rel);
        assert!(text.contains(MARKER), "{rel}");
        assert!(
            text.contains("| [`crates/core`](crates/core) | `foo-core`: "),
            "{rel}"
        );
        assert!(
            text.contains("| [`apps/foo-gui`](apps/foo-gui) | `foo-gui`: "),
            "{rel}"
        );
        assert!(
            text.contains("| [`apps/foo-console`](apps/foo-console) | `foo-console`: "),
            "{rel}"
        );
    }
}

#[test]
fn add_crate_appends_one_row_and_keeps_the_owners_edits() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let root = tmp.path();
    for rel in ["README.md", "docs/ARCHITECTURE.md"] {
        let text = read(root, rel).replace("`foo-core`: ", "`foo-core`: the game rules, ")
            + "\nOwner paragraph.\n";
        fs::write(root.join(rel), text).unwrap();
    }
    let before: Vec<String> = ["README.md", "docs/ARCHITECTURE.md"]
        .iter()
        .map(|r| read(root, r))
        .collect();

    let ws = Workspace::find(root).unwrap();
    let report = add_crate(&ws, "render").unwrap();
    assert_ne!(report.created.len(), 0);
    assert!(root.join("crates/render/src/lib.rs").is_file());
    assert!(read(root, "crates/render/Cargo.toml").contains("name = \"foo-render\""));

    let row =
        "| [`crates/render`](crates/render) | `foo-render`: describe what this crate is for. |";
    for (rel, old) in ["README.md", "docs/ARCHITECTURE.md"].iter().zip(&before) {
        let expected = insert_after(old, last_row(old), row);
        assert_eq!(read(root, rel), expected, "{rel}");
        assert!(read(root, rel).contains("the game rules, "));
    }

    let after: Vec<String> = ["README.md", "docs/ARCHITECTURE.md"]
        .iter()
        .map(|r| read(root, r))
        .collect();
    let err = add_crate(&ws, "render").unwrap_err().to_string();
    assert_eq!(err, "crate render already exists");
    for (rel, text) in ["README.md", "docs/ARCHITECTURE.md"].iter().zip(&after) {
        assert_eq!(&read(root, rel), text);
    }
}

#[test]
fn add_app_console_creates_the_app_and_a_row_and_leaves_the_package_apps_alone() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let root = tmp.path();
    let ws = Workspace::find(root).unwrap();
    add_app(&ws, "tool", AppKind::Console).unwrap();
    let manifest = read(root, "apps/foo-tool/Cargo.toml");
    assert!(manifest.contains("name = \"foo-tool\""), "{manifest}");
    assert!(manifest.contains("foo-core = { path = \"../../crates/core\" }"));
    let main = read(root, "apps/foo-tool/src/main.rs");
    assert!(main.contains("foo-tool {}"), "{main}");
    assert!(read(root, "README.md").contains("| [`apps/foo-tool`](apps/foo-tool) | `foo-tool`: "));
    let ws = Workspace::find(root).unwrap();
    assert_eq!(ws.package_meta().unwrap().apps, ["foo-gui", "foo-console"]);

    add_app(&ws, "viewer", AppKind::Gui).unwrap();
    assert!(read(root, "apps/foo-viewer/Cargo.toml").contains("name = \"foo-viewer\""));
    assert!(read(root, "apps/foo-viewer/src/main.rs").contains("Foo"));
    let err = add_app(&ws, "tool", AppKind::Gui).unwrap_err().to_string();
    assert_eq!(err, "app foo-tool already exists");
}

fn bk(dir: &Path) -> Command {
    let mut c = Command::cargo_bin("cargo-bk").unwrap();
    c.current_dir(dir);
    c
}

#[test]
fn a_missing_marker_prints_the_row_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let root = tmp.path();
    let readme = read(root, "README.md").replace(MARKER, "");
    fs::write(root.join("README.md"), &readme).unwrap();
    let out = bk(root)
        .args(["add-app", "tool", "--console"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains(
            "README.md has no <!-- bk:layout --> table; add the row by hand: | [`apps/foo-tool`](apps/foo-tool) | `foo-tool`: "
        ),
        "{stdout}"
    );
    assert_eq!(read(root, "README.md"), readme);
    assert!(root.join("apps/foo-tool/Cargo.toml").is_file());
    assert!(read(root, "docs/ARCHITECTURE.md").contains("apps/foo-tool"));
}

#[test]
fn the_cli_requires_exactly_one_app_kind() {
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    bk(tmp.path()).args(["add-app", "tool"]).assert().failure();
    bk(tmp.path())
        .args(["add-app", "tool", "--console", "--gui"])
        .assert()
        .failure();
    bk(tmp.path())
        .args(["add-crate", "render"])
        .assert()
        .success();
    assert!(tmp.path().join("crates/render/Cargo.toml").is_file());
}

#[test]
fn the_generated_manual_passes_the_manual_check() {
    let script = Path::new(&std::env::var("HOME").unwrap())
        .join("Dokumente/dev/buildkit/tools/manual/build_manual.py");
    if !script.is_file() {
        eprintln!("skipped: {} not found", script.display());
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    product(tmp.path());
    let out = std::process::Command::new("python3")
        .arg(&script)
        .arg("--root")
        .arg(tmp.path().join("docs/manual"))
        .arg("--check")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.is_empty(), "{text}");
}
