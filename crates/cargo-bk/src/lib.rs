//! `cargo bk`: generate and maintain products that use buildkit-rust.

pub mod cli;
pub mod commands;
pub mod docs_rows;
pub mod exec;
pub mod names;
pub mod os;
pub mod package;
pub mod pins;
pub mod python;
pub mod templates;
pub mod workspace;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Command};
use workspace::Workspace;

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
        Some(Command::AddCrate { name }) => {
            let ws = commands::open_workspace(&std::env::current_dir()?)?;
            commands::add::add_crate(&ws, name)?;
            Ok(())
        }
        Some(Command::AddApp(args)) => {
            let ws = commands::open_workspace(&std::env::current_dir()?)?;
            commands::add::add_app(&ws, &args.name, args.kind())?;
            Ok(())
        }
        Some(Command::Manual { check }) => {
            let ws = Workspace::find(&std::env::current_dir()?)?;
            commands::manual::run(&ws, &exec::SystemRunner, os::Os::current(), *check)
        }
        Some(Command::CheckPins) => {
            commands::check_pins::run(&Workspace::find(&std::env::current_dir()?)?)
        }
        Some(Command::Upgrade { version }) => {
            let mut ws = Workspace::find(&std::env::current_dir()?)?;
            commands::upgrade::run(&mut ws, version)
        }
        Some(Command::Doctor { machine }) => {
            let ws = if *machine {
                None
            } else {
                match Workspace::find(&std::env::current_dir()?) {
                    Ok(ws) => Some(ws),
                    Err(e) if e.to_string().starts_with("no Cargo workspace found") => None,
                    Err(e) => return Err(e),
                }
            };
            commands::doctor::run(ws.as_ref(), &exec::SystemRunner, os::Os::current())
        }
        Some(_) => bail!("not implemented yet"),
        None => bail!("no command given; see cargo bk --help"),
    }
}
