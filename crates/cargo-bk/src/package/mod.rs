//! Building installer packages.

pub mod deb;
pub mod inno;
pub mod notices;
pub mod stage;

use crate::os::Os;
use crate::workspace::PackageMeta;
use anyhow::{Result, bail};
use buildkit::Version;
use std::path::Path;

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
