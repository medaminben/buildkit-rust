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
    let cwd = std::env::current_dir()?;
    match &cli.command {
        Some(Command::Version) => {
            println!("{}", commands::version::run(&cwd)?);
        }
        Some(Command::New(args)) => {
            let report = commands::new::new_project(&cwd, args, &exec::SystemRunner)?;
            println!(
                "created {} files, kept {}",
                report.created.len(),
                report.kept.len()
            );
        }
        Some(Command::AddCrate { name }) => {
            commands::add::add_crate(&commands::open_workspace(&cwd)?, name)?;
        }
        Some(Command::AddApp(args)) => {
            commands::add::add_app(&commands::open_workspace(&cwd)?, &args.name, args.kind())?;
        }
        Some(Command::Package { format }) => {
            commands::package::run(
                &commands::open_workspace(&cwd)?,
                format.as_deref(),
                os::Os::current(),
                &exec::SystemRunner,
                &buildkit::SystemGit,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
            )?;
        }
        Some(Command::Manual { check }) => {
            commands::manual::run(
                &Workspace::find(&cwd)?,
                &exec::SystemRunner,
                os::Os::current(),
                *check,
            )?;
        }
        Some(Command::CheckPins) => commands::check_pins::run(&Workspace::find(&cwd)?)?,
        Some(Command::Upgrade { version }) => {
            commands::upgrade::run(
                &mut Workspace::find(&cwd)?,
                version,
                &exec::SystemRunner,
                &mut std::io::stdout(),
            )?;
        }
        Some(Command::Doctor { machine }) => {
            let ws = if *machine {
                None
            } else {
                Workspace::try_find(&cwd)?
            };
            commands::doctor::run(ws.as_ref(), &exec::SystemRunner, os::Os::current())?;
        }
        Some(Command::Link {
            path,
            rewrite_dep,
            force,
        }) => commands::link::run_link(
            Workspace::find(&cwd)?,
            path.as_deref(),
            *rewrite_dep,
            *force,
        )?,
        Some(Command::Unlink { rewrite_dep }) => {
            commands::link::run_unlink(Workspace::find(&cwd)?, *rewrite_dep)?;
        }
        Some(Command::RepoSetup { dry_run, checks }) => commands::repo_setup::run(
            &Workspace::find(&cwd)?,
            &exec::SystemRunner,
            *dry_run,
            checks.as_deref(),
            &mut std::io::stdout(),
        )?,
        None => bail!("no command given; see cargo bk --help"),
    }
    Ok(())
}
