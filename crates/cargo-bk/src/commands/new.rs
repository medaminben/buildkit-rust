//! `cargo bk new`: a workspace, its crates, a console and a GUI app, and the shared files.

use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};

use super::OWN_VERSION;
use crate::exec::{Cmd, Runner};
use crate::names::{CrateName, ProductName};
use crate::templates::{TEMPLATES, Vars, WriteReport, write_tree};

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

    /// The branch pull requests target.
    fn pr_target(self) -> &'static str {
        match self {
            Flow::Direct => "main",
            Flow::Beta => "Beta",
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
}

/// The rust-toolchain channel of this repository, the one products start on.
fn rust_version() -> Result<String> {
    let doc: toml_edit::DocumentMut = include_str!("../../../../rust-toolchain.toml").parse()?;
    doc["toolchain"]["channel"]
        .as_str()
        .map(str::to_string)
        .context("rust-toolchain.toml has no channel")
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

fn vars(product: &ProductName, krate: &CrateName, args: &NewArgs, rust: &str) -> Vars {
    let repo = args.repo.clone().unwrap_or_default();
    let owner = repo.split('/').next().unwrap_or_default().to_string();
    let entries = [
        ("name", product.display.clone()),
        ("kebab", product.kebab.clone()),
        ("snake", product.snake.clone()),
        ("crate", krate.dir.clone()),
        ("crate_snake", krate.snake.clone()),
        ("crate_package", krate.package(product)),
        ("app", format!("{}-console", product.kebab)),
        ("repo", repo),
        ("owner", owner),
        ("bk_version", OWN_VERSION.to_string()),
        ("app_id_guid", new_guid()),
        ("year", this_year().to_string()),
        ("flow", args.flow.as_str().to_string()),
        ("pr_target", args.flow.pr_target().to_string()),
        ("workflow", args.flow.workflow().to_string()),
        ("branches", args.flow.branches().to_string()),
        ("rust_version", rust.to_string()),
    ];
    Vars(BTreeMap::from(entries))
}

fn merge(into: &mut WriteReport, from: WriteReport) {
    into.created.extend(from.created);
    into.kept.extend(from.kept);
}

fn cargo(runner: &dyn Runner, dir: &Path, args: &[&str]) -> Result<()> {
    let cmd = Cmd {
        program: "cargo".to_string(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        cwd: Some(dir.to_path_buf()),
        env: Vec::new(),
    };
    let out = runner.run(&cmd)?;
    if out.status != 0 {
        bail!("`cargo {}` failed:\n{}", args.join(" "), out.stderr);
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
    let rust = rust_version()?;
    let tree = |name: &str| {
        TEMPLATES
            .get_dir(name)
            .with_context(|| format!("missing template folder {name}"))
    };

    let first = &crates[0];
    let mut report = write_tree(tree("project")?, dir, &vars(&product, first, args, &rust))?;
    for krate in &crates {
        merge(
            &mut report,
            write_tree(tree("crate")?, dir, &vars(&product, krate, args, &rust))?,
        );
    }
    merge(
        &mut report,
        write_tree(
            tree("app-console")?,
            dir,
            &vars(&product, first, args, &rust),
        )?,
    );
    let mut gui_vars = vars(&product, first, args, &rust);
    gui_vars.0.insert("app", format!("{}-gui", product.kebab));
    merge(&mut report, write_tree(tree("app-gui")?, dir, &gui_vars)?);
    if args.repo.is_some() {
        merge(
            &mut report,
            write_tree(tree("community")?, dir, &vars(&product, first, args, &rust))?,
        );
    }

    if !args.no_build {
        cargo(runner, dir, &["build"])?;
        cargo(runner, dir, &["test"])?;
        let app = format!("{}-console", product.kebab);
        cargo(runner, dir, &["run", "-p", &app])?;
    }
    Ok(report)
}
