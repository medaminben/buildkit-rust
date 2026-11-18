use std::path::Path;
use std::process::{Command, Stdio};

/// Runs git for the version resolution. `None` on any failure.
pub trait Git {
    /// Runs `git -C <dir> <args>` and returns trimmed stdout.
    fn run(&self, dir: &Path, args: &[&str]) -> Option<String>;
}

/// Runs the `git` found on `PATH`.
pub struct SystemGit;

/// Like [`SystemGit`], with extra environment variables for each git call
/// (for example `GIT_CONFIG_GLOBAL` in tests).
pub struct SystemGitWith {
    pub env: Vec<(String, String)>,
}

fn run_git(dir: &Path, args: &[&str], env: &[(String, String)]) -> Option<String> {
    let out = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .envs(env.iter().map(|(k, v)| (k, v)))
        .args(["-c", "log.showSignature=false"])
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

impl Git for SystemGit {
    fn run(&self, dir: &Path, args: &[&str]) -> Option<String> {
        run_git(dir, args, &[])
    }
}

impl Git for SystemGitWith {
    fn run(&self, dir: &Path, args: &[&str]) -> Option<String> {
        run_git(dir, args, &self.env)
    }
}
