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

/// `text` without the marked block, or `None` when there is no block.
fn without_block(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut inside = false;
    let mut found = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if !inside && t.starts_with(MARK_START) {
            inside = true;
            found = true;
        } else if inside {
            if t == MARK_END {
                inside = false;
            }
        } else {
            out.push_str(line);
        }
    }
    found.then_some(out)
}

/// Writes the patch block into `<product>/../.cargo/config.toml`, replacing an earlier one.
/// Returns the config file.
///
/// # Errors
/// When `checkout` is not a buildkit-rust checkout or the file cannot be written.
pub fn link(product: &Path, checkout: &Path) -> Result<PathBuf> {
    let abs = checked_checkout(checkout)?;
    let file = config_path(product)?;
    let old = std::fs::read_to_string(&file).unwrap_or_default();
    let mut text = without_block(&old).unwrap_or(old);
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let block = format!(
        "{MARK_START} (remove with cargo bk unlink)\n[patch.\"{GIT_URL}\"]\nbuildkit = {{ path = \"{abs}/crates/buildkit\" }}\n{MARK_END}\n"
    );
    text.push_str(&block);
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
/// When the file cannot be read or written.
pub fn unlink(product: &Path) -> Result<bool> {
    let file = config_path(product)?;
    let Ok(old) = std::fs::read_to_string(&file) else {
        return Ok(false);
    };
    let Some(new) = without_block(&old) else {
        return Ok(false);
    };
    if new.trim().is_empty() {
        std::fs::remove_file(&file).with_context(|| format!("cannot remove {}", file.display()))?;
    } else {
        std::fs::write(&file, new).with_context(|| format!("cannot write {}", file.display()))?;
    }
    Ok(true)
}

fn set_dep(ws: &mut Workspace, build: impl FnOnce(&mut InlineTable)) -> Result<()> {
    let Some(Item::Value(old)) = ws
        .doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        .and_then(|d| d.get_mut("buildkit"))
    else {
        bail!("[workspace.dependencies] buildkit is missing");
    };
    let mut table = InlineTable::new();
    build(&mut table);
    let mut new = Value::InlineTable(table);
    *new.decor_mut() = old.decor().clone();
    *old = new;
    ws.save()
}

/// Rewrites the root manifest's buildkit dependency to the local checkout.
///
/// # Errors
/// When `checkout` is not a buildkit-rust checkout or the dependency is missing.
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
pub fn run_link(mut ws: Workspace, path: Option<&Path>, rewrite: bool) -> Result<()> {
    let checkout = path.map_or_else(|| ws.root.join(DEFAULT_CHECKOUT), Path::to_path_buf);
    if rewrite {
        rewrite_dep(&mut ws, &checkout)?;
        eprintln!(
            "warning: Cargo.toml now uses a local buildkit path; do not commit this change (cargo bk unlink --rewrite-dep restores it)"
        );
    } else {
        let file = link(&ws.root, &checkout)?;
        println!("linked {} in {}", checkout.display(), file.display());
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
