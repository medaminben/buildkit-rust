//! Finding a usable Python interpreter.

use crate::exec::{Cmd, Runner};
use crate::os::Os;
use anyhow::{Result, bail};

const MIN_MINOR: u32 = 11;

fn candidate(program: &str, pre: &[&str]) -> Cmd {
    Cmd {
        program: program.to_string(),
        args: pre.iter().map(|s| (*s).to_string()).collect(),
        cwd: None,
        env: vec![],
    }
}

/// Parses `Python 3.x.y` and returns the minor version of a Python 3.
fn minor_version(text: &str) -> Option<u32> {
    let rest = text.split("Python 3.").nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The first of `python3`, `python` and (Windows) `py -3` that is Python 3.11+.
///
/// # Errors
/// When none qualifies.
pub fn find_python(r: &dyn Runner, os: Os) -> Result<Cmd> {
    let mut candidates = vec![candidate("python3", &[]), candidate("python", &[])];
    if os == Os::Windows {
        candidates.push(candidate("py", &["-3"]));
    }
    for cmd in candidates {
        let mut probe = cmd.clone();
        probe.args.push("--version".to_string());
        let Ok(out) = r.run(&probe) else { continue };
        if out.status != 0 {
            continue;
        }
        let text = format!("{}\n{}", out.stdout, out.stderr);
        if minor_version(&text).is_some_and(|m| m >= MIN_MINOR) {
            return Ok(cmd);
        }
    }
    bail!("Python 3.11+ not found; run bootstrap (group python)")
}

#[cfg(test)]
mod tests {
    use super::minor_version;

    #[test]
    fn parses_the_minor_version() {
        assert_eq!(minor_version("Python 3.12.1\n"), Some(12));
        assert_eq!(minor_version("\nPython 3.9.0"), Some(9));
        assert_eq!(minor_version("Python 2.7.18"), None);
        assert_eq!(minor_version("nope"), None);
    }
}
