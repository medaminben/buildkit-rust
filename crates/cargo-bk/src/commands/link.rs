//! `cargo bk link` and `unlink`: point a product at a local buildkit-rust checkout.

use crate::workspace::Workspace;
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use toml_edit::{InlineTable, Item, Value};

const GIT_URL: &str = "https://github.com/medaminben/buildkit-rust";
const MARK_START: &str = "# >>> cargo bk link";
const MARK_END: &str = "# <<< cargo bk link";
/// The default checkout, next to the product.
pub const DEFAULT_CHECKOUT: &str = "../buildkit-rust";

/// The checkout as an absolute path with forward slashes, valid inside a TOML string.
fn checked_checkout(given: &Path) -> Result<String> {
    if !given.join("crates/buildkit/Cargo.toml").is_file() {
        bail!(
            "{} is not a buildkit-rust checkout (crates/buildkit/Cargo.toml missing)",
            given.display()
        );
    }
    let abs = std::fs::canonicalize(given)
        .with_context(|| format!("cannot resolve {}", given.display()))?;
    let text = abs.to_string_lossy().replace('\\', "/");
    Ok(text.strip_prefix("//?/").unwrap_or(&text).to_string())
}

fn config_path(product: &Path) -> Result<PathBuf> {
    let parent = product
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent folder", product.display()))?;
    Ok(parent.join(".cargo/config.toml"))
}

const ADDED_EOL: &str = "added newline";

/// `text` without the marked block, or `None` when there is no block. A block that
/// `link` put after a final line without newline takes that newline with it, as long
/// as nothing follows the block.
fn without_block(text: &str, file: &Path) -> Result<Option<String>> {
    let mut out = String::new();
    let mut open: Option<usize> = None;
    let mut found = false;
    let mut added_eol = false;
    let mut offset = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        offset += line.len();
        let t = line.trim();
        if open.is_none() && t.starts_with(MARK_START) {
            open = Some(i + 1);
            found = true;
            added_eol = t.contains(ADDED_EOL);
        } else if open.is_some() {
            if t == MARK_END {
                open = None;
                if added_eol && text[offset..].trim().is_empty() && out.ends_with('\n') {
                    out.pop();
                }
            }
        } else {
            out.push_str(line);
        }
    }
    if let Some(line) = open {
        bail!(
            "{}:{line}: `{MARK_START}` has no matching `{MARK_END}`; fix or remove the block by hand",
            file.display()
        );
    }
    Ok(found.then_some(out))
}

/// The config file's text; a missing file is empty, anything else unreadable is an error.
fn read_config(file: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(file) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", file.display())),
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Writes the patch block into `<product>/../.cargo/config.toml`, replacing an earlier one.
/// `home` is the user's home folder, whose `.cargo/config.toml` is cargo's global config.
/// Returns the config file.
///
/// # Errors
/// When `checkout` is not a buildkit-rust checkout, the parent folder is a root or (without
/// `force`) the home folder, the file is unreadable or has an unterminated block, it already
/// patches buildkit-rust outside the block, or it cannot be written.
pub fn link(product: &Path, checkout: &Path, home: Option<&Path>, force: bool) -> Result<PathBuf> {
    let abs = checked_checkout(checkout)?;
    let file = config_path(product)?;
    let parent = product.parent().unwrap_or(product);
    if parent.parent().is_none() {
        bail!(
            "{} is a root folder; put the product in a project folder first",
            parent.display()
        );
    }
    if !force && home.is_some_and(|h| same_dir(h, parent)) {
        bail!(
            "{} is your home folder, so {} is cargo's global config; use --force to write it anyway",
            parent.display(),
            file.display()
        );
    }
    let old = read_config(&file)?.unwrap_or_default();
    let mut text = without_block(&old, &file)?.unwrap_or(old);
    let existing: toml_edit::DocumentMut = text
        .parse()
        .with_context(|| format!("cannot parse {}", file.display()))?;
    if existing.get("patch").and_then(|p| p.get(GIT_URL)).is_some() {
        bail!(
            "{} already has [patch.\"{GIT_URL}\"]; remove or merge it by hand",
            file.display()
        );
    }
    let added_eol = !text.is_empty() && !text.ends_with('\n');
    if added_eol {
        text.push('\n');
    }
    let note = if added_eol {
        format!("; {ADDED_EOL}")
    } else {
        String::new()
    };
    let block = format!(
        "{MARK_START} (remove with cargo bk unlink{note})\n[patch.\"{GIT_URL}\"]\nbuildkit = {{ path = \"{abs}/crates/buildkit\" }}\n{MARK_END}\n"
    );
    text.push_str(&block);
    text.parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("refusing to write an invalid {}", file.display()))?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    }
    std::fs::write(&file, text).with_context(|| format!("cannot write {}", file.display()))?;
    Ok(file)
}

/// Removes the block; deletes the config file when nothing else is left.
/// Returns whether there was a block.
///
/// # Errors
/// When the file cannot be read or written, or has an unterminated block.
pub fn unlink(product: &Path) -> Result<bool> {
    let file = config_path(product)?;
    let Some(old) = read_config(&file)? else {
        return Ok(false);
    };
    let Some(new) = without_block(&old, &file)? else {
        return Ok(false);
    };
    if new.trim().is_empty() {
        std::fs::remove_file(&file).with_context(|| format!("cannot remove {}", file.display()))?;
    } else {
        std::fs::write(&file, new).with_context(|| format!("cannot write {}", file.display()))?;
    }
    Ok(true)
}

/// Replaces `git`, `tag` and `path` of the inline buildkit dependency; other keys stay.
fn set_dep(ws: &mut Workspace, edit: impl FnOnce(&mut InlineTable)) -> Result<()> {
    let Some(item) = ws
        .doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        .and_then(|d| d.get_mut("buildkit"))
    else {
        bail!("buildkit dependency is missing from [workspace.dependencies]");
    };
    if item.is_none() {
        bail!("buildkit dependency is missing from [workspace.dependencies]");
    }
    let Item::Value(Value::InlineTable(table)) = item else {
        bail!("buildkit dependency is not an inline table; edit it by hand");
    };
    for key in ["git", "tag", "path"] {
        table.remove(key);
    }
    edit(table);
    ws.save()
}

/// Rewrites the root manifest's buildkit dependency to the local checkout.
///
/// # Errors
/// When `checkout` is not a buildkit-rust checkout, or the dependency is missing or
/// not an inline table.
pub fn rewrite_dep(ws: &mut Workspace, checkout: &Path) -> Result<()> {
    let abs = checked_checkout(checkout)?;
    set_dep(ws, |t| {
        t.insert("path", Value::from(format!("{abs}/crates/buildkit")));
    })
}

/// Restores the git dependency pinned by `[workspace.metadata.buildkit] version`.
///
/// # Errors
/// When the version or the dependency is missing.
pub fn restore_dep(ws: &mut Workspace) -> Result<()> {
    let version = ws
        .bk_version()
        .ok_or_else(|| anyhow!("[workspace.metadata.buildkit] version is missing"))?;
    set_dep(ws, |t| {
        t.insert("git", Value::from(GIT_URL));
        t.insert("tag", Value::from(format!("v{version}")));
    })
}

/// The `link` command.
///
/// # Errors
/// See [`link`] and [`rewrite_dep`].
pub fn run_link(mut ws: Workspace, path: Option<&Path>, rewrite: bool, force: bool) -> Result<()> {
    let checkout = path.map_or_else(|| ws.root.join(DEFAULT_CHECKOUT), Path::to_path_buf);
    if rewrite {
        rewrite_dep(&mut ws, &checkout)?;
        eprintln!(
            "Cargo.toml now uses a local buildkit path; do not commit this change (cargo bk unlink --rewrite-dep restores it)"
        );
    } else {
        let home =
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
        let file = link(&ws.root, &checkout, home.as_deref(), force)?;
        let parent = ws.root.parent().unwrap_or(&ws.root);
        println!(
            "patch written to {}; it applies to every Cargo project under {}",
            file.display(),
            parent.display()
        );
    }
    println!(
        "cargo install --path {} --locked",
        checkout.join("crates/cargo-bk").display()
    );
    Ok(())
}

/// The `unlink` command.
///
/// # Errors
/// See [`unlink`] and [`restore_dep`].
pub fn run_unlink(mut ws: Workspace, rewrite: bool) -> Result<()> {
    if rewrite {
        restore_dep(&mut ws)?;
        println!("restored the git dependency in Cargo.toml");
    } else if unlink(&ws.root)? {
        println!("unlinked");
    } else {
        println!("no link to remove");
    }
    Ok(())
}
