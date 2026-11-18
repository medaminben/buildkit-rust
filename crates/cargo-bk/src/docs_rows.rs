//! Append-only edits of the owner-editable markdown: a row is added to a table, nothing else
//! in the file is touched.

use std::path::Path;

use anyhow::{Context, Result};

use crate::names::{CrateName, ProductName};

/// The marker comment that precedes the layout table.
pub const LAYOUT_MARKER: &str = "<!-- bk:layout -->";

/// The files that carry a layout table, relative to the product root.
const LAYOUT_FILES: [&str; 2] = ["README.md", "docs/ARCHITECTURE.md"];

/// The first cell of a table row, which names the row.
fn key(row: &str) -> &str {
    row.trim()
        .trim_start_matches('|')
        .split('|')
        .next()
        .unwrap_or("")
        .trim()
}

/// The line range `[first, end)` of the table that follows `marker`, if there is one.
fn table_lines(lines: &[&str], marker: &str) -> Option<(usize, usize)> {
    let at = lines.iter().position(|l| l.trim() == marker)?;
    let first = at + 1 + lines[at + 1..].iter().position(|l| !l.trim().is_empty())?;
    if !lines[first].starts_with('|') {
        return None;
    }
    let len = lines[first..]
        .iter()
        .take_while(|l| l.starts_with('|'))
        .count();
    Some((first, first + len))
}

/// Whether `markdown` has a table after `marker` that rows can be added to.
#[must_use]
pub fn has_table(markdown: &str, table_marker: &str) -> bool {
    table_lines(&markdown.lines().collect::<Vec<_>>(), table_marker).is_some()
}

/// `markdown` with `row` added after the last row of the table that follows `table_marker`.
/// `None` when the row is already there (same first cell, whatever the owner wrote after it)
/// or when there is no such table; use [`has_table`] to tell the two apart.
#[must_use]
pub fn ensure_row(markdown: &str, table_marker: &str, row: &str) -> Option<String> {
    let lines: Vec<&str> = markdown.lines().collect();
    let (first, end) = table_lines(&lines, table_marker)?;
    if lines[first..end].iter().any(|l| key(l) == key(row)) {
        return None;
    }
    let offset: usize = lines[..end].iter().map(|l| l.len() + 1).sum();
    let offset = offset.min(markdown.len());
    let mut out = String::with_capacity(markdown.len() + row.len() + 2);
    out.push_str(&markdown[..offset]);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(row);
    out.push('\n');
    out.push_str(&markdown[offset..]);
    Some(out)
}

/// The layout row of a library crate.
#[must_use]
pub fn crate_row(product: &ProductName, krate: &CrateName) -> String {
    format!(
        "| [`crates/{dir}`](crates/{dir}) | `{package}`: describe what this crate is for. |",
        dir = krate.dir,
        package = krate.package(product)
    )
}

/// The layout row of an app; `purpose` ends the sentence after the package name.
#[must_use]
pub fn app_row(app: &str, purpose: &str) -> String {
    format!("| [`apps/{app}`](apps/{app}) | `{app}`: {purpose} |")
}

/// Adds `row` to the layout table of README.md and docs/ARCHITECTURE.md under `root`.
/// A file without the marker table is left alone and a note tells the owner the row to add.
///
/// # Errors
/// When a file exists but cannot be read or written.
pub fn add_layout_row(root: &Path, row: &str) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    for rel in LAYOUT_FILES {
        let path = root.join(rel);
        if !path.is_file() {
            notes.push(format!("{rel} not found; add the row by hand: {row}"));
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if !has_table(&text, LAYOUT_MARKER) {
            notes.push(format!(
                "{rel} has no {LAYOUT_MARKER} table; add the row by hand: {row}"
            ));
        } else if let Some(new) = ensure_row(&text, LAYOUT_MARKER, row) {
            std::fs::write(&path, new)
                .with_context(|| format!("cannot write {}", path.display()))?;
        }
    }
    Ok(notes)
}
