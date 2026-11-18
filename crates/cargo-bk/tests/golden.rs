//! The committed fixture product `tests/fixtures/foo` must equal a fresh `cargo bk new` with the
//! same fixed values, so it cannot go stale. The self-test runs the product's CI on it.

mod common;

use cargo_bk::commands::new::new_project;
use cargo_bk::exec::FakeRunner;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const FIXED_GUID: &str = "00000000-0000-4000-8000-000000000000";
const FIXED_YEAR: u64 = 2026;
const FIXED_BK_VERSION: &str = "0.0.0";

const REGENERATE: &str = "regenerate with: rm -rf tests/fixtures/foo && mkdir -p tests/fixtures/foo \
&& cd tests/fixtures/foo && cargo run --manifest-path ../../../Cargo.toml -p cargo-bk -- new Foo core physics --repo x/foo --no-build \
--fixed-guid 00000000-0000-4000-8000-000000000000 --fixed-year 2026 --fixed-bk-version 0.0.0";

/// Text files with `\r\n` folded to `\n`: a checkout applies the fixture's own `.gitattributes`
/// (`*.ps1 text eol=crlf`), while the generator writes `\n`. Binary files stay raw.
fn normalised(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        return bytes;
    }
    let mut out = Vec::with_capacity(bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        if !(*b == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
            out.push(*b);
        }
    }
    out
}

/// Every file under `dir` by relative path, without build output and the lock file that CI
/// generates after it links the product to the buildkit-rust checkout.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path: PathBuf = entry.unwrap().path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                if rel != "build" {
                    walk(root, &path, out);
                }
            } else if rel != "Cargo.lock" {
                out.insert(rel, normalised(fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn fixture_matches_a_fresh_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let mut args = common::new_args(Some("Foo"), &["core", "physics"]);
    args.repo = Some("x/foo".to_string());
    args.fixed_guid = Some(FIXED_GUID.to_string());
    args.fixed_year = Some(FIXED_YEAR);
    args.fixed_bk_version = Some(FIXED_BK_VERSION.to_string());
    new_project(tmp.path(), &args, &FakeRunner::new()).unwrap();
    let fresh = files(tmp.path());

    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/foo");
    assert!(
        fixture_dir.is_dir(),
        "tests/fixtures/foo is missing\n{REGENERATE}"
    );
    let committed = files(&fixture_dir);

    let missing: Vec<_> = fresh
        .keys()
        .filter(|k| !committed.contains_key(*k))
        .collect();
    let extra: Vec<_> = committed
        .keys()
        .filter(|k| !fresh.contains_key(*k))
        .collect();
    let changed: Vec<_> = fresh
        .iter()
        .filter(|(k, v)| committed.get(*k).is_some_and(|c| c != *v))
        .map(|(k, _)| k)
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty() && changed.is_empty(),
        "fixture differs from a fresh generation\nmissing: {missing:?}\nextra: {extra:?}\n\
         changed: {changed:?}\n{REGENERATE}"
    );
}
