mod common;

use cargo_bk::commands::manual::capture;
use cargo_bk::exec::FakeRunner;
use cargo_bk::os::Os;
use cargo_bk::workspace::Workspace;
use common::bk;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

/// The fake capture program, compiled once with the rustc that builds the tests.
fn fake_program() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fake-capture");
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(format!("fake_capture{}", std::env::consts::EXE_SUFFIX));
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_capture.rs");
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let st = Command::new(rustc)
            .args(["--edition", "2024", "-o"])
            .arg(&exe)
            .arg(&src)
            .status()
            .unwrap();
        assert!(st.success(), "cannot compile the fake capture program");
        exe
    })
}

struct Product {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    log: PathBuf,
}

fn toml_str(p: &Path) -> String {
    format!("{:?}", p.to_string_lossy())
}

fn product(command: bool) -> Product {
    product_with(command, None)
}

/// `build_extra`: arguments after the fake program in the optional `build` command.
fn product_with(command: bool, build_extra: Option<&str>) -> Product {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let log = root.join("calls.log");
    let mut manifest = String::from("[workspace]\nmembers = []\n");
    if command {
        write!(
            manifest,
            "[workspace.metadata.buildkit.capture]\ncommand = [{}, \"--log={}\"]\n",
            toml_str(fake_program()),
            log.display().to_string().replace('\\', "/"),
        )
        .unwrap();
    }
    if let Some(extra) = build_extra {
        let extra = if extra.is_empty() {
            String::new()
        } else {
            format!(", {extra:?}")
        };
        writeln!(
            manifest,
            "build = [{}, \"--log={}\"{extra}]",
            toml_str(fake_program()),
            log.display().to_string().replace('\\', "/"),
        )
        .unwrap();
    }
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    Product {
        _tmp: tmp,
        root,
        log,
    }
}

fn runner(shots: &[(&str, &str)]) -> FakeRunner {
    let mut listing = String::new();
    for (l, n) in shots {
        writeln!(listing, "{l} {n}").unwrap();
    }
    FakeRunner::new()
        .with_output("python3", 0, "Python 3.12.0\n")
        .with_output_for("python3", "--list-shots", 0, &listing)
}

fn shot(root: &Path, lang: &str, name: &str) -> PathBuf {
    root.join(format!("docs/manual/assets/shots/{lang}/{name}.png"))
}

fn run(p: &Product, shots: &[(&str, &str)], only: &[&str], secs: u64) -> anyhow::Result<()> {
    let ws = Workspace::find(&p.root).unwrap();
    let only: Vec<String> = only.iter().map(|s| (*s).to_string()).collect();
    capture::run(
        &ws,
        &runner(shots),
        Os::Linux,
        &only,
        Duration::from_secs(secs),
    )
}

fn log_lines(p: &Product) -> Vec<String> {
    let mut v: Vec<String> = fs::read_to_string(&p.log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    v.sort();
    v
}

fn leftovers(root: &Path) -> Vec<PathBuf> {
    walk(&root.join("docs/manual/assets/shots"))
        .into_iter()
        .filter(|p| p.extension().is_none_or(|e| e != "png"))
        .collect()
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    rd.flat_map(|e| {
        let p = e.unwrap().path();
        if p.is_dir() { walk(&p) } else { vec![p] }
    })
    .collect()
}

#[test]
fn every_shot_runs_once_with_the_screen_and_an_absolute_path() {
    let p = product(true);
    run(
        &p,
        &[("de", "home"), ("en", "home"), ("en", "menu")],
        &[],
        20,
    )
    .unwrap();
    assert_eq!(
        log_lines(&p),
        ["home home.png.part true", "menu menu.png.part true"]
    );
    for (l, n) in [("de", "home"), ("en", "home"), ("en", "menu")] {
        assert_eq!(fs::read(shot(&p.root, l, n)).unwrap()[1..4], *b"PNG");
    }
    assert_none(leftovers(&p.root));
}

#[test]
fn a_missing_configuration_exits_2_with_the_message() {
    let p = product(false);
    let err = run(&p, &[("en", "home")], &[], 5).unwrap_err();
    assert_eq!(
        err.to_string(),
        "no [workspace.metadata.buildkit.capture] command in Cargo.toml"
    );
    bk(&p.root)
        .args(["manual", "capture"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            "no [workspace.metadata.buildkit.capture] command in Cargo.toml",
        ));
}

#[test]
fn a_failing_and_a_hanging_shot_are_listed_and_the_others_are_kept() {
    let p = product(true);
    let started = std::time::Instant::now();
    let err = run(
        &p,
        &[
            ("en", "a-ok"),
            ("en", "fail"),
            ("en", "hang"),
            ("en", "z-ok"),
        ],
        &[],
        1,
    )
    .unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(30));
    let text = format!("{err:#}");
    assert!(text.contains("fail"), "{text}");
    assert!(
        text.contains("hang") && text.contains("timed out"),
        "{text}"
    );
    assert!(!text.contains("a-ok") && !text.contains("z-ok"), "{text}");
    assert!(shot(&p.root, "en", "a-ok").is_file());
    assert!(shot(&p.root, "en", "z-ok").is_file());
    assert!(!shot(&p.root, "en", "fail").exists());
    assert!(!shot(&p.root, "en", "hang").exists());
    assert_none(leftovers(&p.root));
    assert!(
        err.downcast_ref::<capture::NoConfig>().is_none(),
        "failures exit 1, not 2"
    );
}

#[test]
fn an_exit_0_without_a_file_or_a_partial_file_fails_and_leaves_nothing() {
    let p = product(true);
    let err = run(&p, &[("en", "empty"), ("en", "partial")], &[], 20).unwrap_err();
    let text = format!("{err:#}");
    assert!(text.contains("empty") && text.contains("partial"), "{text}");
    assert_none(walk(&p.root.join("docs/manual/assets/shots")));
}

#[test]
fn a_failed_recapture_keeps_the_old_file() {
    let p = product(true);
    let old = shot(&p.root, "en", "fail");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::write(&old, b"old").unwrap();
    run(&p, &[("en", "fail")], &[], 20).unwrap_err();
    assert_eq!(fs::read(&old).unwrap(), b"old");
    assert_none(leftovers(&p.root));
}

#[test]
fn only_limits_the_shots_and_rejects_unknown_names() {
    let p = product(true);
    run(&p, &[("en", "home"), ("en", "menu")], &["menu"], 20).unwrap();
    assert_eq!(log_lines(&p), ["menu menu.png.part true"]);
    assert!(!shot(&p.root, "en", "home").exists());
    let err = run(&p, &[("en", "home")], &["nope"], 20).unwrap_err();
    assert!(err.to_string().contains("nope"), "{err}");
}

#[test]
fn the_build_command_runs_once_before_the_shots() {
    let p = product_with(true, Some(""));
    run(&p, &[("en", "home"), ("en", "menu")], &[], 20).unwrap();
    let log = fs::read_to_string(&p.log).unwrap();
    assert_eq!(log.lines().next(), Some("BUILD"), "{log}");
    assert_eq!(log.matches("BUILD").count(), 1, "{log}");
    assert_eq!(log.lines().count(), 3, "{log}");
}

#[test]
fn a_failing_build_stops_before_any_shot_and_exits_1() {
    let p = product_with(true, Some("--build-fail"));
    let err = run(&p, &[("en", "home")], &[], 20).unwrap_err();
    assert!(err.to_string().contains("capture build failed"), "{err}");
    assert_eq!(log_lines(&p), ["BUILD"]);
    assert!(!shot(&p.root, "en", "home").exists());
}

#[allow(clippy::needless_pass_by_value)]
fn assert_none(v: Vec<PathBuf>) {
    assert_eq!(v.len(), 0, "{v:?}");
}
