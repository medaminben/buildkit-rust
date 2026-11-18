use std::path::Path;
use std::process::{Command, Stdio};

/// Runs git for the version resolution. `None` on any failure.
pub trait Git {
    /// Runs `git -C <dir> <args>` and returns trimmed stdout.
    fn run(&self, dir: &Path, args: &[&str]) -> Option<String>;
}

/// Runs the `git` found on `PATH`.
pub struct SystemGit;

impl Git for SystemGit {
    fn run(&self, dir: &Path, args: &[&str]) -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}
