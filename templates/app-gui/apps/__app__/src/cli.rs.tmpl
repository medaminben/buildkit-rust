//! The command line: `--screen=<name>` opens a screen, `--capture=<path>` writes a screenshot of it
//! to a PNG file and exits (`cargo bk manual capture` passes both).

use std::path::PathBuf;

/// The screens `--screen` can open. Add yours here and to `Screen::parse`.
const SCREEN_NAMES: &str = "main";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Main,
}

impl Screen {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "main" => Ok(Self::Main),
            _ => Err(format!("unknown screen '{name}'; use {SCREEN_NAMES}")),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub screen: Screen,
    pub capture: Option<PathBuf>,
}

impl Cli {
    /// Reads `--screen=<name>` and `--capture=<path>`; other arguments are ignored. An unknown
    /// screen is an error.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut cli = Self::default();
        for arg in args {
            if let Some(name) = arg.strip_prefix("--screen=") {
                cli.screen = Screen::parse(name)?;
            } else if let Some(path) = arg.strip_prefix("--capture=") {
                cli.capture = Some(PathBuf::from(path));
            }
        }
        Ok(cli)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, String> {
        Cli::parse(args.iter().map(|a| (*a).to_owned()))
    }

    #[test]
    fn no_arguments_open_the_main_screen_without_a_capture() {
        assert_eq!(parse(&[]), Ok(Cli::default()));
    }

    #[test]
    fn screen_and_capture_are_read() {
        let cli = parse(&["--screen=main", "--capture=/tmp/a.png"]).unwrap();
        assert_eq!(cli.screen, Screen::Main);
        assert_eq!(cli.capture, Some(PathBuf::from("/tmp/a.png")));
    }

    #[test]
    fn an_unknown_screen_names_the_known_ones() {
        assert_eq!(
            parse(&["--screen=lobby"]),
            Err("unknown screen 'lobby'; use main".to_owned())
        );
    }
}
