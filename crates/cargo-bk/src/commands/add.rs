//! `cargo bk add-crate` and `cargo bk add-app`: one more crate or app, plus its row in the docs.

use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;

use super::new::{Flow, merge, rust_version, vars};
use crate::docs_rows::{add_layout_row, app_row, crate_row};
use crate::names::{CrateName, ProductName};
use crate::templates::{TEMPLATES, WriteReport, write_tree};
use crate::workspace::Workspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum AppKind {
    Console,
    Gui,
}

#[derive(clap::Args, Debug)]
#[command(group(clap::ArgGroup::new("kind").required(true).args(["console", "gui"])))]
pub struct AddAppArgs {
    /// App name; becomes `apps/<product>-<name>`.
    pub name: String,
    /// A command-line app.
    #[arg(long)]
    console: bool,
    /// A desktop app.
    #[arg(long)]
    gui: bool,
}

impl AddAppArgs {
    #[must_use]
    pub fn kind(&self) -> AppKind {
        if self.gui {
            AppKind::Gui
        } else {
            AppKind::Console
        }
    }
}

/// What every added file is built from: the product, its settings and the crate to build on.
struct Ctx {
    product: ProductName,
    repo: String,
    flow: Flow,
    rust: String,
}

fn context(ws: &Workspace) -> Result<Ctx> {
    let product = ProductName::parse(&ws.package_meta()?.display_name)?;
    let flow = match ws.bk_flow() {
        Some(f) => Flow::from_str(&f, true).map_err(|_| {
            anyhow::anyhow!("[workspace.metadata.buildkit] flow {f:?} is not direct or beta")
        })?,
        None => Flow::Direct,
    };
    Ok(Ctx {
        product,
        repo: ws.repo(),
        flow,
        rust: rust_version()?,
    })
}

fn print_notes(notes: &[String]) {
    for note in notes {
        println!("{note}");
    }
}

/// The crate a new app builds on: `core` if there is one, else the first crate by name.
fn base_crate(root: &Path) -> Result<CrateName> {
    let dir = root.join("crates");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.path().join("Cargo.toml").is_file())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    let pick = if names.iter().any(|n| n == "core") {
        "core".to_string()
    } else {
        names
            .into_iter()
            .next()
            .context("no crate in crates/ for the app to build on; run cargo bk add-crate first")?
    };
    Ok(CrateName::parse(&pick)?)
}

/// Adds the library crate `name` under `crates/` and its row to README.md and ARCHITECTURE.md.
///
/// # Errors
/// When the name is invalid, the crate exists, or a file cannot be written.
pub fn add_crate(ws: &Workspace, name: &str) -> Result<WriteReport> {
    let ctx = context(ws)?;
    let krate = CrateName::parse(name)?;
    if ws.root.join("crates").join(&krate.dir).exists() {
        bail!("crate {} already exists", krate.dir);
    }
    let tree = TEMPLATES
        .get_dir("crate")
        .context("missing template folder crate")?;
    let report = write_tree(
        tree,
        &ws.root,
        &vars(&ctx.product, &krate, &ctx.repo, ctx.flow, &ctx.rust),
    )?;
    print_notes(&add_layout_row(&ws.root, &crate_row(&ctx.product, &krate))?);
    println!("added crate {}", krate.package(&ctx.product));
    Ok(report)
}

/// Adds the app `<product>-<name>` under `apps/` and its row to README.md and ARCHITECTURE.md.
/// The app builds on the product's `core` crate (or its first crate). The packaging list
/// `apps` in the root manifest is not changed.
///
/// # Errors
/// When the name is invalid, the app exists, there is no crate, or a file cannot be written.
pub fn add_app(ws: &Workspace, name: &str, kind: AppKind) -> Result<WriteReport> {
    let ctx = context(ws)?;
    let suffix = CrateName::parse(name)?;
    let app = format!("{}-{}", ctx.product.kebab, suffix.dir);
    if ws.root.join("apps").join(&app).exists() {
        bail!("app {app} already exists");
    }
    let base = base_crate(&ws.root)?;
    let (folder, purpose) = match kind {
        AppKind::Console => ("app-console", "a command-line app."),
        AppKind::Gui => ("app-gui", "a desktop app."),
    };
    let tree = TEMPLATES
        .get_dir(folder)
        .with_context(|| format!("missing template folder {folder}"))?;
    let mut v = vars(&ctx.product, &base, &ctx.repo, ctx.flow, &ctx.rust);
    v.0.insert("app", app.clone());
    let mut report = WriteReport::default();
    merge(&mut report, write_tree(tree, &ws.root, &v)?);
    print_notes(&add_layout_row(&ws.root, &app_row(&app, purpose))?);
    println!("added app {app}");
    Ok(report)
}
