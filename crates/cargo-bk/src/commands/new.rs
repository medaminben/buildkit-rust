//! `cargo bk new`: a workspace, its crates, a console and a GUI app, and the shared files.

use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};

use super::OWN_VERSION;
use crate::docs_rows::{add_layout_row, app_row, crate_row};
use crate::exec::{Cmd, Runner};
use crate::names::{CrateName, ProductName, file_stem};
use crate::pins::is_release_version;
use crate::templates::{TEMPLATES, Vars, WriteReport, write_tree};
use crate::workspace::toolchain_channel;

/// How changes reach the default branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Flow {
    /// Feature branches merge straight into main.
    Direct,
    /// Feature branches merge into Beta, which merges into main.
    Beta,
}

impl Flow {
    fn as_str(self) -> &'static str {
        match self {
            Flow::Direct => "direct",
            Flow::Beta => "beta",
        }
    }

    /// The CONTRIBUTING.md paragraph describing the flow.
    fn workflow(self) -> &'static str {
        match self {
            Flow::Direct => {
                "Branch from `main` and open a pull request into `main`. A release is a tag \
`vX.Y.Z` on `main`."
            }
            Flow::Beta => {
                "Branch from `Beta` and open a pull request into `Beta`. Beta is merged into \
`main` by the maintainer when a release is due, so never open a feature pull request into `main`."
            }
        }
    }

    fn branches(self) -> &'static str {
        match self {
            Flow::Direct => "main",
            Flow::Beta => "main, Beta",
        }
    }
}

#[derive(Args, Debug)]
pub struct NewArgs {
    /// Product name; defaults to the folder name.
    pub name: Option<String>,
    /// Library crates to create; defaults to `core`.
    pub crates: Vec<String>,
    /// GitHub repository as `owner/repo`.
    #[arg(long, value_name = "owner/repo")]
    pub repo: Option<String>,
    #[arg(long, value_enum, default_value = "direct")]
    pub flow: Flow,
    /// Skip the first build, test and run.
    #[arg(long)]
    pub no_build: bool,
    /// Use this installer GUID (golden fixture only).
    #[arg(long, hide = true, value_name = "guid")]
    pub fixed_guid: Option<String>,
    /// Use this year in the licence and notices (golden fixture only).
    #[arg(long, hide = true, value_name = "year")]
    pub fixed_year: Option<u64>,
    /// Use this buildkit-rust version in the pins (golden fixture only).
    #[arg(long, hide = true, value_name = "x.y.z")]
    pub fixed_bk_version: Option<String>,
    /// Version of the first release; defaults to 0.1.0.
    #[arg(long, value_name = "x.y.z")]
    pub first_release: Option<String>,
}

/// The rust-toolchain channel of this repository, the one products start on.
pub(crate) fn rust_version() -> Result<String> {
    toolchain_channel(include_str!("../../../../rust-toolchain.toml"))
        .context("rust-toolchain.toml has no channel")
}

/// The `--first-release` value without a leading `v`, or an error when it is not `X.Y.Z`.
fn first_release(args: &NewArgs) -> Result<Option<String>> {
    let Some(raw) = &args.first_release else {
        return Ok(None);
    };
    let version = raw.strip_prefix('v').unwrap_or(raw);
    if !is_release_version(version) {
        bail!("--first-release {raw}: expected X.Y.Z");
    }
    Ok(Some(version.to_string()))
}

/// Whether `version` (`X.Y.Z`, possibly with a `-N-gSHA` suffix) is at least `0.2.0`,
/// the first buildkit-rust release whose `release.yml` reads `first-release`.
fn reads_first_release(version: &str) -> bool {
    let mut parts = version
        .split(['.', '-'])
        .map(|p| p.parse::<u64>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    (major, minor) >= (0, 2)
}

/// Refuses `--first-release` when the product would pin a buildkit-rust that ignores the key.
fn check_first_release_support(bk_version: &str) -> Result<()> {
    if !reads_first_release(bk_version) {
        bail!(
            "--first-release needs buildkit-rust 0.2.0 or later; this cargo-bk pins v{bk_version}. \
             Install cargo-bk from tag v0.2.0."
        );
    }
    Ok(())
}

/// Refuses a `first-release` that differs from the key already in `dir/Cargo.toml`.
fn check_first_release_unchanged(dir: &Path, version: &str) -> Result<()> {
    let path = dir.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let doc: toml_edit::DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let old = doc
        .get("workspace")
        .and_then(|w| w.get("metadata"))
        .and_then(|m| m.get("buildkit"))
        .and_then(|b| b.get("first-release"))
        .and_then(|v| v.as_str());
    if let Some(old) = old
        && old != version
    {
        bail!(
            "--first-release {version}: {} already has first-release = \"{old}\"; edit it by hand",
            path.display()
        );
    }
    Ok(())
}

/// Sets the fallback version and the `first-release` key in the generated root `Cargo.toml`.
fn write_first_release(dir: &Path, version: &str) -> Result<()> {
    let path = dir.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    doc["workspace"]["package"]["version"] = toml_edit::value(version);
    let table = doc["workspace"]["metadata"]["buildkit"]
        .as_table_mut()
        .context("Cargo.toml has no [workspace.metadata.buildkit]")?;
    // toml_edit appends new keys, so move the keys after `version` behind the new one.
    let after: Vec<String> = table
        .iter()
        .map(|(k, _)| k.to_string())
        .skip_while(|k| k != "version")
        .skip(1)
        .collect();
    let moved: Vec<_> = after.iter().filter_map(|k| table.remove_entry(k)).collect();
    table.insert("first-release", toml_edit::value(version));
    for (key, item) in moved {
        table.insert_formatted(&key, item);
    }
    std::fs::write(&path, doc.to_string())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// A random version 4 GUID, e.g. for an installer's app id.
fn new_guid() -> String {
    let state = RandomState::new();
    let half = |salt: u64| {
        let mut h = state.build_hasher();
        h.write_u64(salt);
        h.finish()
    };
    let (hi, lo) = (half(1), half(2));
    let hi = (hi & !0xF000) | 0x4000;
    let lo = (lo & !(0xC000 << 48)) | (0x8000 << 48);
    format!(
        "{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
        hi >> 32,
        (hi >> 16) & 0xFFFF,
        hi & 0xFFFF,
        lo >> 48,
        lo & 0xFFFF_FFFF_FFFF
    )
}

/// The current UTC year.
fn this_year() -> u64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    1970 + secs / 31_556_952
}

fn is_product(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|t| t.contains("[workspace.metadata.buildkit]"))
}

fn check_target(dir: &Path) -> Result<()> {
    // A folder that only holds `.git` is a fresh clone and counts as empty.
    let non_empty = dir
        .read_dir()
        .is_ok_and(|mut entries| entries.any(|e| e.is_ok_and(|e| e.file_name() != ".git")));
    if non_empty && !is_product(dir) {
        bail!("folder is not empty and is not a buildkit-rust product");
    }
    Ok(())
}

fn product_name(dir: &Path, args: &NewArgs) -> Result<ProductName> {
    let name = match &args.name {
        Some(n) => n.clone(),
        None => dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("cannot derive a name from the folder; pass one")?
            .to_string(),
    };
    Ok(ProductName::parse(&name)?)
}

fn crate_names(args: &NewArgs) -> Result<Vec<CrateName>> {
    if args.crates.is_empty() {
        return Ok(vec![CrateName::parse("core")?]);
    }
    args.crates
        .iter()
        .map(|c| Ok(CrateName::parse(c)?))
        .collect()
}

/// Placeholder values for one crate (and the app built on it) of a product.
pub(crate) fn vars(
    product: &ProductName,
    krate: &CrateName,
    repo: &str,
    flow: Flow,
    rust: &str,
) -> Vars {
    let repo = repo.to_string();
    let owner = repo.split('/').next().unwrap_or_default().to_string();
    let entries = [
        ("name", product.display.clone()),
        ("kebab", product.kebab.clone()),
        ("name_file", file_stem(&product.display)),
        ("snake", product.snake.clone()),
        ("crate", krate.dir.clone()),
        ("crate_snake", krate.snake.clone()),
        ("crate_package", krate.package(product)),
        ("app", format!("{}-console", product.kebab)),
        ("repo", repo),
        ("owner", owner),
        ("bk_version", OWN_VERSION.to_string()),
        // Braced, as Inno Setup spells an AppId; the .iss escapes the leading brace.
        ("app_id_guid", format!("{{{}}}", new_guid())),
        ("year", this_year().to_string()),
        ("flow", flow.as_str().to_string()),
        ("workflow", flow.workflow().to_string()),
        ("branches", flow.branches().to_string()),
        ("rust_version", rust.to_string()),
    ];
    Vars(BTreeMap::from(entries))
}

pub(crate) fn merge(into: &mut WriteReport, from: WriteReport) {
    into.created.extend(from.created);
    into.kept.extend(from.kept);
}

fn cargo(runner: &dyn Runner, dir: &Path, args: &[&str]) -> Result<()> {
    let out = runner.run(&Cmd::new("cargo", args).cwd(dir))?;
    if out.status != 0 {
        bail!("`cargo {}` failed:\n{}", args.join(" "), out.stderr);
    }
    Ok(())
}

/// Fills the layout tables of README.md and ARCHITECTURE.md with the crates and both apps.
fn layout_rows(
    dir: &Path,
    product: &ProductName,
    crates: &[CrateName],
    gui_app: &str,
    console_app: &str,
) -> Result<()> {
    let mut rows: Vec<String> = crates.iter().map(|c| crate_row(product, c)).collect();
    rows.push(app_row(gui_app, "the desktop app."));
    rows.push(app_row(console_app, "the command-line app."));
    for row in &rows {
        for note in add_layout_row(dir, row)? {
            println!("{note}");
        }
    }
    Ok(())
}

/// Writes the workspace, the crates, both apps and the shared files into `dir`, then builds,
/// tests and runs it unless `args.no_build` is set. Existing files are kept.
///
/// # Errors
/// When `dir` holds something else, a name is invalid, a file cannot be written
/// or a cargo step fails.
pub fn new_project(dir: &Path, args: &NewArgs, runner: &dyn Runner) -> Result<WriteReport> {
    check_target(dir)?;
    let product = product_name(dir, args)?;
    let crates = crate_names(args)?;
    let first_release_version = first_release(args)?;
    if let Some(version) = &first_release_version {
        check_first_release_support(args.fixed_bk_version.as_deref().unwrap_or(OWN_VERSION))?;
        check_first_release_unchanged(dir, version)?;
    }
    let rust = rust_version()?;
    let tree = |name: &str| {
        TEMPLATES
            .get_dir(name)
            .with_context(|| format!("missing template folder {name}"))
    };

    let first = &crates[0];
    let console_app = format!("{}-console", product.kebab);
    let gui_app = format!("{}-gui", product.kebab);
    let repo = args.repo.clone().unwrap_or_default();
    let vars = |krate: &CrateName| {
        let mut v = vars(&product, krate, &repo, args.flow, &rust);
        if let Some(guid) = &args.fixed_guid {
            v.0.insert("app_id_guid", format!("{{{guid}}}"));
        }
        if let Some(year) = args.fixed_year {
            v.0.insert("year", year.to_string());
        }
        if let Some(version) = &args.fixed_bk_version {
            v.0.insert("bk_version", version.clone());
        }
        v
    };
    let mut report = write_tree(tree("project")?, dir, &vars(first))?;
    for krate in &crates {
        merge(&mut report, write_tree(tree("crate")?, dir, &vars(krate))?);
    }
    merge(
        &mut report,
        write_tree(tree("app-console")?, dir, &vars(first))?,
    );
    let mut gui_vars = vars(first);
    gui_vars.0.insert("app", gui_app.clone());
    merge(&mut report, write_tree(tree("app-gui")?, dir, &gui_vars)?);
    if args.repo.is_some() {
        merge(
            &mut report,
            write_tree(tree("community")?, dir, &vars(first))?,
        );
    }

    if let Some(version) = &first_release_version {
        write_first_release(dir, version)?;
    }
    layout_rows(dir, &product, &crates, &gui_app, &console_app)?;

    if !args.no_build {
        cargo(runner, dir, &["build"])?;
        cargo(runner, dir, &["test"])?;
        cargo(runner, dir, &["run", "-p", &console_app])?;
    }
    Ok(report)
}
