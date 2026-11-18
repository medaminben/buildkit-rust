//! The command line.

use crate::commands::new::NewArgs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cargo-bk",
    disable_version_flag = true,
    about = "Generate and maintain buildkit-rust products"
)]
pub struct Cli {
    /// Print the cargo-bk version.
    #[arg(short = 'V', long)]
    pub version: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a product.
    New(NewArgs),
    /// Add a library crate.
    AddCrate,
    /// Add an app crate.
    AddApp,
    /// Print the product version.
    Version,
    /// Build the installer package.
    Package,
    /// Build the manual.
    Manual,
    /// Check the toolchain and the product.
    Doctor,
    /// Check the pinned tool versions.
    CheckPins,
    /// Move the product to this cargo-bk version.
    Upgrade,
    /// Link the product to a local buildkit-rust.
    Link,
    /// Undo `link`.
    Unlink,
    /// Set up the repository.
    RepoSetup,
}
