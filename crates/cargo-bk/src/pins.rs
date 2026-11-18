//! Where a product pins buildkit-rust, and how to keep those pins in step.

use crate::commands::OWN_VERSION;
use crate::templates::{TEMPLATES, Vars, render};
use crate::workspace::Workspace;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use toml_edit::{Item, Value};

const WORKFLOW_PREFIX: &str = "medaminben/buildkit-rust/.github/workflows/";
const LINT_FILES: [&str; 3] = ["rustfmt.toml", "clippy.toml", "deny.toml"];
/// Plain (unrendered) embedded copies, as paths inside the product and the embedded tree.
const PLAIN_LINT_FILES: [(&str, &str); 2] = [
    (
        "lint/commitlint.config.cjs",
        "project/lint/commitlint.config.cjs",
    ),
    (".editorconfig", "project/.editorconfig"),
];
const LINT_HEADER: &str = "# buildkit-rust v";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinSite {
    Metadata,
    GitDep,
    Workflow { file: PathBuf, line: usize },
    LintHeader { file: PathBuf },
}

/// One place that pins buildkit-rust; `version` has no leading `v`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub site: PinSite,
    pub version: String,
}

/// Byte range of the version in a `uses: medaminben/buildkit-rust/.github/workflows/x.yml@vX` line.
fn workflow_version_range(line: &str) -> Option<(usize, usize)> {
    let trimmed_line = line.trim_start();
    if trimmed_line.starts_with('#') {
        return None;
    }
    let uses = line.find("uses:")?;
    let rest = line[uses + "uses:".len()..].trim_start();
    let rest = rest
        .strip_prefix('"')
        .or_else(|| rest.strip_prefix('\''))
        .unwrap_or(rest);
    let body = rest.strip_prefix(WORKFLOW_PREFIX)?;
    let at = body.find('@')?;
    if at == 0 {
        return None;
    }
    let after = body[at + 1..].strip_prefix('v')?;
    let len = after
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '}' | '#'))
        .unwrap_or(after.len());
    if len == 0 {
        return None;
    }
    let start = line.len() - after.len();
    Some((start, start + len))
}

fn workflow_files(root: &Path) -> Vec<PathBuf> {
    let dir = root.join(".github/workflows");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "yml"))
        .collect();
    files.sort();
    files
}

fn lint_header_version(text: &str) -> Option<String> {
    let first = text.lines().next()?;
    let rest = first.strip_prefix(LINT_HEADER)?;
    Some(rest[..rest.find(':')?].to_string())
}

fn git_dep_tag(ws: &Workspace) -> Option<&str> {
    ws.doc
        .get("workspace")?
        .get("dependencies")?
        .get("buildkit")?
        .get("tag")?
        .as_str()
}

/// Every place the product pins buildkit-rust.
///
/// # Errors
/// When a workflow or lint file cannot be read.
pub fn find_pins(ws: &Workspace) -> Result<Vec<Pin>> {
    let mut pins = Vec::new();
    if let Some(version) = ws.bk_version() {
        pins.push(Pin {
            site: PinSite::Metadata,
            version,
        });
    }
    if let Some(tag) = git_dep_tag(ws) {
        pins.push(Pin {
            site: PinSite::GitDep,
            version: tag.trim_start_matches('v').to_string(),
        });
    }
    for file in workflow_files(&ws.root) {
        let text = std::fs::read_to_string(&file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        for (i, line) in text.lines().enumerate() {
            if let Some((a, b)) = workflow_version_range(line) {
                pins.push(Pin {
                    site: PinSite::Workflow {
                        file: file.clone(),
                        line: i + 1,
                    },
                    version: line[a..b].to_string(),
                });
            }
        }
    }
    for name in LINT_FILES {
        let file = ws.root.join(name);
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        if let Some(version) = lint_header_version(&text) {
            pins.push(Pin {
                site: PinSite::LintHeader { file },
                version,
            });
        }
    }
    Ok(pins)
}

/// The pins that differ from the metadata pin. Empty when there is no metadata pin.
#[must_use]
pub fn mismatches(pins: &[Pin]) -> Vec<&Pin> {
    let Some(expected) = pins.iter().find(|p| p.site == PinSite::Metadata) else {
        return Vec::new();
    };
    pins.iter()
        .filter(|p| p.version != expected.version)
        .collect()
}

/// The embedded copy of a rendered lint file for `version`.
fn render_lint(name: &str, version: &str) -> Result<String> {
    let vars = Vars(BTreeMap::from([("bk_version", version.to_string())]));
    let template = TEMPLATES
        .get_file(format!("project/{name}.tmpl"))
        .and_then(|f| f.contents_utf8())
        .with_context(|| format!("embedded template for {name} is missing"))?;
    render(template, &vars)
}

fn embedded_plain(path: &str) -> Result<&'static str> {
    TEMPLATES
        .get_file(path)
        .and_then(|f| f.contents_utf8())
        .with_context(|| format!("embedded file {path} is missing"))
}

/// Lint files of the product that differ from the embedded copies of the running
/// cargo-bk, compared with `\r\n` folded to `\n`. A missing file is not checked.
/// Each entry is a warning line; `github_actions` adds the annotation prefix.
///
/// # Errors
/// When an embedded copy cannot be rendered or a product file cannot be read.
pub fn lint_drift(root: &Path, github_actions: bool) -> Result<Vec<String>> {
    let mut expected = Vec::new();
    for name in LINT_FILES {
        expected.push((name, render_lint(name, OWN_VERSION)?));
    }
    for (name, embedded) in PLAIN_LINT_FILES {
        expected.push((name, embedded_plain(embedded)?.to_string()));
    }
    let mut warnings = Vec::new();
    for (name, want) in expected {
        let file = root.join(name);
        if !file.is_file() {
            continue;
        }
        let have = std::fs::read_to_string(&file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        if have.replace("\r\n", "\n") != want.replace("\r\n", "\n") {
            let prefix = if github_actions {
                format!("::warning file={name}::")
            } else {
                String::new()
            };
            warnings.push(format!(
                "{prefix}warning: {name} differs from buildkit-rust v{OWN_VERSION}"
            ));
        }
    }
    Ok(warnings)
}

fn write_if_changed(path: &Path, old: &str, new: &str, changed: &mut Vec<PathBuf>) -> Result<()> {
    if old != new {
        std::fs::write(path, new).with_context(|| format!("cannot write {}", path.display()))?;
        changed.push(path.to_path_buf());
    }
    Ok(())
}

fn set_metadata_version(ws: &mut Workspace, version: &str) -> Result<()> {
    let Some(Item::Value(old)) = ws
        .doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("metadata"))
        .and_then(|m| m.get_mut("buildkit"))
        .and_then(|b| b.get_mut("version"))
    else {
        bail!("[workspace.metadata.buildkit] version is missing");
    };
    let mut new = Value::from(version);
    *new.decor_mut() = old.decor().clone();
    *old = new;
    Ok(())
}

fn set_tag(ws: &mut Workspace, version: &str) {
    let Some(Item::Value(old)) = ws
        .doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        .and_then(|d| d.get_mut("buildkit"))
        .and_then(|b| b.get_mut("tag"))
    else {
        return;
    };
    let mut new = Value::from(format!("v{version}"));
    *new.decor_mut() = old.decor().clone();
    *old = new;
}

/// Moves every pin to `version`, which must be the running cargo-bk version.
/// Returns the files that changed.
///
/// # Errors
/// When `version` is not the running version, or a file cannot be read or written.
pub fn upgrade(ws: &mut Workspace, version: &str) -> Result<Vec<PathBuf>> {
    let version = version.trim_start_matches('v');
    if version != OWN_VERSION {
        bail!(
            "install cargo-bk v{version} first: cargo install --git https://github.com/medaminben/buildkit-rust --tag v{version} cargo-bk"
        );
    }
    let mut changed = Vec::new();
    let before = ws.doc.to_string();
    set_metadata_version(ws, version)?;
    set_tag(ws, version);
    let after = ws.doc.to_string();
    if before != after {
        ws.save()?;
        changed.push(ws.root.join("Cargo.toml"));
    }
    for file in workflow_files(&ws.root) {
        let old = std::fs::read_to_string(&file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        let new: String = old
            .split_inclusive('\n')
            .map(|line| match workflow_version_range(line) {
                Some((a, b)) => format!("{}{version}{}", &line[..a], &line[b..]),
                None => line.to_string(),
            })
            .collect();
        write_if_changed(&file, &old, &new, &mut changed)?;
    }
    for name in LINT_FILES {
        let file = ws.root.join(name);
        let Ok(old) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut new = render_lint(name, version)?;
        if old.contains("\r\n") {
            new = new.replace('\n', "\r\n");
        }
        write_if_changed(&file, &old, &new, &mut changed)?;
    }
    Ok(changed)
}
