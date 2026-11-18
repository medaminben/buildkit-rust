//! Building installer packages.

pub mod inno;
pub mod notices;
pub mod stage;

use crate::os::Os;
use anyhow::{Result, bail};

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
