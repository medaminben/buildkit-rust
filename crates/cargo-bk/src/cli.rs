//! The command line.

use crate::commands::add::AddAppArgs;
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
    AddCrate {
        /// Crate name; becomes `crates/<name>` and package `<product>-<name>`.
        name: String,
    },
    /// Add an app crate.
    AddApp(AddAppArgs),
    /// Print the product version.
    Version,
    /// Build the installer package.
    Package {
        /// Format to build: inno, appimage, deb or all (default: all of this OS).
        #[arg(long, value_name = "inno|appimage|deb|all")]
        format: Option<String>,
    },
    /// Build the manual.
    Manual {
        /// Check links and content; write nothing.
        #[arg(long)]
        check: bool,
    },
    /// Check the toolchain and the product.
    Doctor {
        /// Check the machine only, even inside a product.
        #[arg(long)]
        machine: bool,
    },
    /// Check the pinned tool versions.
    CheckPins,
    /// Move the product to this cargo-bk version.
    Upgrade {
        /// Target version, with or without a leading `v`.
        version: String,
    },
    /// Link the product to a local buildkit-rust.
    Link {
        /// The checkout; default `../buildkit-rust`.
        path: Option<std::path::PathBuf>,
        /// Rewrite the root Cargo.toml dependency instead of patching the parent cargo config.
        #[arg(long)]
        rewrite_dep: bool,
        /// Write the patch even when the parent folder is your home folder.
        #[arg(long)]
        force: bool,
    },
    /// Undo `link`.
    Unlink {
        /// Restore the git dependency in the root Cargo.toml.
        #[arg(long)]
        rewrite_dep: bool,
    },
    /// Set up the repository.
    RepoSetup,
}
