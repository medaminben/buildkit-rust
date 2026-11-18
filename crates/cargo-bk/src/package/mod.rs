//! Building installer packages.

pub mod appimage;
pub mod deb;
pub mod inno;
pub mod notices;
pub mod stage;

use crate::os::Os;
use crate::workspace::PackageMeta;
use anyhow::{Context, Result, bail};
use buildkit::Version;
use std::path::Path;
use walkdir::WalkDir;

/// What every package builder needs.
pub struct PackageCtx<'a> {
    /// The workspace root.
    pub root: &'a Path,
    /// The staged install tree, `<root>/build/stage`.
    pub stage: &'a Path,
    /// The directory the package is written to.
    pub out: &'a Path,
    pub meta: &'a PackageMeta,
    pub kebab: &'a str,
    pub maintainer: &'a str,
    pub version: &'a Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Inno,
    AppImage,
    Deb,
}

/// The formats to build. `None` and `all` mean every format of `os`.
///
/// # Errors
/// When the name is unknown or the format cannot be built on `os`.
pub fn formats_for(requested: Option<&str>, os: Os) -> Result<Vec<Format>> {
    match (requested, os) {
        (None | Some("all"), Os::Linux) => Ok(vec![Format::AppImage, Format::Deb]),
        (None | Some("all" | "inno"), Os::Windows) => Ok(vec![Format::Inno]),
        (Some("inno"), Os::Linux) => bail!("inno installers are built on Windows"),
        (Some("appimage"), Os::Linux) => Ok(vec![Format::AppImage]),
        (Some("deb"), Os::Linux) => Ok(vec![Format::Deb]),
        (Some(f @ ("appimage" | "deb")), Os::Windows) => {
            bail!("{f} packages are built on Linux")
        }
        (Some(other), _) => {
            bail!("unknown package format `{other}`; use inno, appimage, deb or all")
        }
    }
}

/// Creates the parent directory of `path`, if it has one.
///
/// # Errors
/// When the directory cannot be created.
pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    Ok(())
}

/// Copies `src` to `dest`, creating the parent directory of `dest`.
///
/// # Errors
/// When the directory cannot be created or the file cannot be copied.
pub fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    ensure_parent(dest)?;
    std::fs::copy(src, dest)
        .with_context(|| format!("cannot copy {} to {}", src.display(), dest.display()))?;
    Ok(())
}

/// Copies every file under `src` to the same relative path under `dest`.
///
/// # Errors
/// When a directory cannot be read or a file cannot be copied.
pub fn copy_tree(src: &Path, dest: &Path) -> Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let target = dest.join(entry.path().strip_prefix(src)?);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("cannot create {}", target.display()))?;
        } else {
            std::fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "cannot copy {} to {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}
