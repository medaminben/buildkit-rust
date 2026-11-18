//! The command line.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo-bk", version = env!("BK_VERSION_FULL"), about = "Generate and maintain buildkit-rust products")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a product.
    New,
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
