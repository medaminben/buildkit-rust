// The runner, names and workspace model are used by the commands still to come.
#![allow(dead_code)]

mod cli;
mod commands;
mod exec;
mod names;
mod os;
mod workspace;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Command};

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Version => {
            println!("{}", commands::version::run(&std::env::current_dir()?)?);
            Ok(())
        }
        _ => bail!("not implemented yet"),
    }
}

fn main() {
    // `cargo bk <cmd>` runs `cargo-bk bk <cmd>`.
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "bk") {
        args.remove(1);
    }
    if let Err(e) = run(&Cli::parse_from(args)) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
