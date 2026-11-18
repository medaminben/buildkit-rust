//! `cargo bk package`.

use crate::exec::{Cmd, Runner};
use crate::names::ProductName;
use crate::os::Os;
use crate::package::stage::{apply_stage, plan_stage};
use crate::package::{Format, deb, formats_for, notices};
use crate::workspace::Workspace;
use anyhow::{Result, bail};
use buildkit::Git;
use std::io::Write;
use std::path::PathBuf;

/// Builds the requested packages into `build/dist` and returns their paths.
///
/// # Errors
/// When the format is not available on `os`, an app fails to build, or a
/// package cannot be made.
pub fn run(
    ws: &Workspace,
    requested: Option<&str>,
    os: Os,
    r: &dyn Runner,
    git: &dyn Git,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Vec<PathBuf>> {
    let formats = formats_for(requested, os)?;
    if !ws.root.join("Cargo.lock").is_file() {
        bail!(
            "Cargo.lock is missing; run cargo generate-lockfile and commit it (packages build with --locked)"
        );
    }
    let meta = ws.package_meta()?;
    let kebab = ProductName::parse(&meta.display_name)?.kebab;
    let fallback = ws
        .root_package_version()
        .unwrap_or_else(|| "0.0.0".to_string());
    let version = buildkit::resolve(&ws.root, &fallback, git);
    if version.full.ends_with("-dev") {
        writeln!(
            err,
            "warning: no release tag found (shallow clone? use fetch-depth: 0); packaging as {}",
            version.full
        )?;
    }

    for app in &meta.apps {
        let res = r.run(&Cmd {
            program: "cargo".into(),
            args: ["build", "--release", "--locked", "-p", app]
                .map(String::from)
                .to_vec(),
            cwd: Some(ws.root.clone()),
            env: Vec::new(),
        })?;
        if res.status != 0 {
            bail!("cargo build of {app} failed: {}", res.stderr.trim());
        }
    }

    let plan = plan_stage(ws, &meta, os)?;
    let stage = apply_stage(&plan, &ws.root)?;
    std::fs::write(
        stage.join("THIRD_PARTY_NOTICES.txt"),
        notices::notices(ws, r)?,
    )?;

    let dist = ws.root.join("build").join("dist");
    let repo = ws.repo();
    let maintainer = repo
        .split('/')
        .next()
        .filter(|o| !o.is_empty())
        .unwrap_or(&kebab);
    let mut built = Vec::new();
    for format in formats {
        let path = match format {
            Format::Deb => deb::build_deb(
                &ws.root,
                &stage,
                &dist,
                &meta,
                &kebab,
                maintainer,
                &deb::deb_version(&version.full),
                r,
            )?,
            Format::AppImage | Format::Inno => bail!("not implemented yet"),
        };
        writeln!(out, "{}", path.display())?;
        built.push(path);
    }
    Ok(built)
}
