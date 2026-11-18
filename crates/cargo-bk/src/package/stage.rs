//! Staging the install tree into `build/stage/`.

use crate::os::Os;
use crate::workspace::{PackageMeta, Workspace};
use anyhow::{Context, Result, bail};
use globset::GlobBuilder;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct StagePlan {
    /// `(source, destination relative to the stage directory)`.
    pub files: Vec<(PathBuf, PathBuf)>,
}

/// Lists what goes into the stage: app binaries, `LICENSE` if present and the
/// files matching `extra-files`. The notices file is written by the caller.
///
/// # Errors
/// When an app has no built binary, or an extra-files glob is invalid or
/// leaves the workspace root.
pub fn plan_stage(ws: &Workspace, meta: &PackageMeta, os: Os) -> Result<StagePlan> {
    let mut files = Vec::new();
    let exe = if os == Os::Windows { ".exe" } else { "" };
    for app in &meta.apps {
        let name = format!("{app}{exe}");
        let src = ws.root.join("build/target/release").join(&name);
        if !src.is_file() {
            bail!("app {app} listed in [workspace.metadata.bk.package] apps has no binary");
        }
        files.push((src, PathBuf::from(name)));
    }
    let license = ws.root.join("LICENSE");
    if license.is_file() {
        files.push((license, PathBuf::from("LICENSE")));
    }
    extra_files(&ws.root, &meta.extra_files, &mut files)?;
    Ok(StagePlan { files })
}

fn extra_files(
    root: &Path,
    patterns: &[String],
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    let mut matchers = Vec::new();
    for p in patterns {
        let escapes = Path::new(p).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
        if escapes {
            bail!("extra-files glob `{p}` leaves the workspace root");
        }
        let glob = GlobBuilder::new(p)
            .literal_separator(true)
            .build()
            .with_context(|| format!("invalid extra-files glob `{p}`"))?;
        matchers.push(glob.compile_matcher());
    }
    if matchers.is_empty() {
        return Ok(());
    }
    let walker = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.depth() != 1 || !matches!(e.file_name().to_str(), Some("build" | "target" | ".git"))
        });
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root)?;
        if matchers.iter().any(|m| m.is_match(rel)) && !files.iter().any(|(_, d)| d == rel) {
            files.push((entry.path().to_path_buf(), rel.to_path_buf()));
        }
    }
    Ok(())
}

/// Replaces `stage_dir` with the planned files. `stage_dir` must lie below a
/// directory named `build`, because the old content is deleted.
///
/// # Errors
/// When `stage_dir` is not below a `build` directory, a destination leaves the
/// stage, or a file operation fails.
pub fn apply_stage(plan: &StagePlan, stage_dir: &Path) -> Result<()> {
    let comps: Vec<Component> = stage_dir.components().collect();
    let below_build = comps
        .iter()
        .rev()
        .skip(1)
        .any(|c| matches!(c, Component::Normal(n) if *n == "build"));
    if !below_build || comps.contains(&Component::ParentDir) {
        bail!(
            "refusing to wipe {}: the stage directory must be inside a build/ folder",
            stage_dir.display()
        );
    }
    for (_, dest) in &plan.files {
        if dest.is_absolute() || dest.components().any(|c| c == Component::ParentDir) {
            bail!("stage destination {} leaves the stage", dest.display());
        }
    }
    if stage_dir.exists() {
        std::fs::remove_dir_all(stage_dir)
            .with_context(|| format!("cannot clear {}", stage_dir.display()))?;
    }
    for (src, dest) in &plan.files {
        let target = stage_dir.join(dest);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        std::fs::copy(src, &target)
            .with_context(|| format!("cannot copy {} to {}", src.display(), target.display()))?;
    }
    if !stage_dir.exists() {
        std::fs::create_dir_all(stage_dir)?;
    }
    Ok(())
}
