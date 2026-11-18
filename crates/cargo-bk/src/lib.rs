//! `cargo bk`: generate and maintain products that use buildkit-rust.

pub mod cli;
pub mod commands;
pub mod exec;
pub mod names;
pub mod os;
pub mod templates;
pub mod workspace;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Command};

/// Runs the CLI with `args` (the program name first) and returns the exit code.
/// A leading `bk` is dropped, so `cargo bk <cmd>` and `cargo-bk <cmd>` both work.
#[must_use]
pub fn main_with(mut args: Vec<String>) -> i32 {
    if args.get(1).is_some_and(|a| a == "bk") {
        args.remove(1);
    }
    match run(&Cli::parse_from(args)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    if cli.version {
        println!("{}", env!("BK_VERSION_FULL"));
        return Ok(());
    }
    match &cli.command {
        Some(Command::Version) => {
            println!("{}", commands::version::run(&std::env::current_dir()?)?);
            Ok(())
        }
        Some(Command::New(args)) => {
            let dir = std::env::current_dir()?;
            let report = commands::new::new_project(&dir, args, &exec::SystemRunner)?;
            println!(
                "created {} files, kept {}",
                report.created.len(),
                report.kept.len()
            );
            Ok(())
        }
        Some(_) => bail!("not implemented yet"),
        None => bail!("no command given; see cargo bk --help"),
    }
}
