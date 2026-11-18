//! Template rendering and safe, non-clobbering writing of the embedded template tree.
//!
//! Placeholders are `{{key}}` in contents and `__key__` in path segments. Text such as
//! `${{ github.sha }}` is left alone because its inner text is not a bare key. For an
//! unspaced literal, escape the opening braces: `\{{` renders as `{{` and the text after
//! it is not scanned as a placeholder (`$\{{github}}` renders as `${{github}}`).

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use include_dir::{Dir, DirEntry, include_dir};

/// The embedded template tree (repo-root `templates/`).
pub static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../templates");

/// Placeholder values, keyed by placeholder name.
pub struct Vars(pub BTreeMap<&'static str, String>);

/// What `write_tree` did.
#[derive(Debug, Default)]
pub struct WriteReport {
    pub created: Vec<PathBuf>,
    pub kept: Vec<PathBuf>,
}

fn replace_delimited(text: &str, open: &str, close: &str, vars: &Vars) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        if open == "{{" && rest[..start].ends_with('\\') {
            out.push_str(&rest[..start - 1]);
            out.push_str(open);
            rest = after;
            continue;
        }
        let Some(end) = after.find(close) else { break };
        let key = &after[..end];
        let is_key = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if is_key {
            let value = vars
                .0
                .get(key)
                .ok_or_else(|| anyhow!("unknown placeholder `{key}`"))?;
            out.push_str(&rest[..start]);
            out.push_str(value);
            rest = &after[end + close.len()..];
        } else {
            out.push_str(&rest[..start + open.len()]);
            rest = after;
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Replaces `{{key}}` in `text`. An unknown key is an error naming the key.
///
/// # Errors
/// Fails when `text` uses a key missing from `vars`.
pub fn render(text: &str, vars: &Vars) -> Result<String> {
    Ok(replace_delimited(text, "{{", "}}", vars)?.replace("\r\n", "\n"))
}

/// Replaces `__key__` in each path segment and strips a trailing `.tmpl`.
///
/// # Errors
/// Fails when a segment uses a key missing from `vars`.
pub fn render_path(rel: &Path, vars: &Vars) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for comp in rel.components() {
        let seg = comp.as_os_str().to_string_lossy();
        out.push(replace_delimited(&seg, "__", "__", vars)?);
    }
    if out.extension().is_some_and(|e| e == "tmpl") {
        out.set_extension("");
    }
    Ok(out)
}

fn collect<'a>(dir: &'a Dir<'static>, files: &mut Vec<&'a include_dir::File<'static>>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => collect(d, files),
            DirEntry::File(f) => files.push(f),
        }
    }
}

/// Writes `dir` under `dest`, rendering `.tmpl` files. Existing files are never touched.
/// Every file is rendered before the first one is written, so a bad template writes nothing.
///
/// # Errors
/// Fails on an unknown placeholder (naming the template path) or an I/O error.
pub fn write_tree(dir: &Dir<'static>, dest: &Path, vars: &Vars) -> Result<WriteReport> {
    let mut files = Vec::new();
    collect(dir, &mut files);
    let mut planned = Vec::new();
    for file in files {
        let rel = file.path();
        if rel.file_name().is_some_and(|n| n == ".keep") {
            continue;
        }
        let shown = rel.display();
        let target =
            dest.join(render_path(rel, vars).with_context(|| format!("template path {shown}"))?);
        let is_tmpl = rel.extension().is_some_and(|e| e == "tmpl");
        let bytes = match std::str::from_utf8(file.contents()) {
            Ok(text) if is_tmpl => render(text, vars)
                .with_context(|| format!("rendering template {shown}"))?
                .into_bytes(),
            Ok(text) => text.replace("\r\n", "\n").into_bytes(),
            Err(_) if is_tmpl => bail!("template {shown} is not valid UTF-8"),
            Err(_) => file.contents().to_vec(),
        };
        planned.push((target, bytes));
    }
    let mut report = WriteReport::default();
    for (target, bytes) in planned {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut f) => {
                f.write_all(&bytes)
                    .with_context(|| format!("writing {}", target.display()))?;
                report.created.push(target);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => report.kept.push(target),
            Err(e) => return Err(e).with_context(|| format!("creating {}", target.display())),
        }
    }
    Ok(report)
}
