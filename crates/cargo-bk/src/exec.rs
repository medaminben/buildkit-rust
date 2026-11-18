//! Process execution behind a trait, so commands can be tested without
//! spawning anything.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl Cmd {
    #[must_use]
    pub fn new(program: &str, args: &[&str]) -> Self {
        Cmd {
            program: program.to_string(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            cwd: None,
            env: Vec::new(),
        }
    }

    #[must_use]
    pub fn cwd(mut self, dir: &Path) -> Self {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    #[must_use]
    pub fn env(mut self, vars: &[(&str, &str)]) -> Self {
        self.env = vars
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait Runner {
    /// Runs `cmd` to completion. A failure to spawn is an `Err`; a non-zero
    /// exit is an `Ok` with that status.
    ///
    /// # Errors
    /// When the process cannot be spawned.
    fn run(&self, cmd: &Cmd) -> Result<Output>;

    /// Finds `program` on `PATH`.
    fn which(&self, program: &str) -> Option<PathBuf>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, cmd: &Cmd) -> Result<Output> {
        let mut c = Command::new(&cmd.program);
        c.args(&cmd.args).envs(cmd.env.iter().map(|(k, v)| (k, v)));
        if let Some(dir) = &cmd.cwd {
            c.current_dir(dir);
        }
        let out = c
            .output()
            .with_context(|| format!("cannot run `{}`", cmd.program))?;
        Ok(Output {
            status: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn which(&self, program: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).find_map(|dir| find_in(&dir, program))
    }
}

fn find_in(dir: &Path, program: &str) -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    names
        .iter()
        .map(|ext| dir.join(format!("{program}{ext}")))
        .find(|p| p.is_file())
}

pub use fake::FakeRunner;

mod fake {
    use super::{Cmd, Output, Runner};
    use anyhow::{Result, bail};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Scripted responses by program name; records every `Cmd` it receives.
    #[derive(Default)]
    pub struct FakeRunner {
        responses: RefCell<HashMap<String, Output>>,
        scoped: RefCell<Vec<(String, String, Output)>>,
        on_path: RefCell<HashMap<String, PathBuf>>,
        calls: RefCell<Vec<Cmd>>,
    }

    impl FakeRunner {
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Answers every run of `program` with `status`, `stdout` and no stderr.
        #[must_use]
        pub fn with_output(self, program: &str, status: i32, stdout: &str) -> Self {
            self.responses.borrow_mut().insert(
                program.to_string(),
                Output {
                    status,
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                },
            );
            self
        }

        /// Answers every run of `program` with `status`, no stdout and `stderr`.
        #[must_use]
        pub fn with_failure(self, program: &str, status: i32, stderr: &str) -> Self {
            self.responses.borrow_mut().insert(
                program.to_string(),
                Output {
                    status,
                    stdout: String::new(),
                    stderr: stderr.to_string(),
                },
            );
            self
        }

        /// Like `with_output`, but only for runs of `program` with an argument
        /// containing `arg_part`; checked before the program-only responses.
        #[must_use]
        pub fn with_output_for(
            self,
            program: &str,
            arg_part: &str,
            status: i32,
            stdout: &str,
        ) -> Self {
            self.scoped.borrow_mut().push((
                program.to_string(),
                arg_part.to_string(),
                Output {
                    status,
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                },
            ));
            self
        }

        /// Like `with_failure`, but scoped by an argument substring.
        #[must_use]
        pub fn with_failure_for(
            self,
            program: &str,
            arg_part: &str,
            status: i32,
            stderr: &str,
        ) -> Self {
            self.scoped.borrow_mut().push((
                program.to_string(),
                arg_part.to_string(),
                Output {
                    status,
                    stdout: String::new(),
                    stderr: stderr.to_string(),
                },
            ));
            self
        }

        /// Makes `which(program)` return `path`.
        #[must_use]
        pub fn with_tool(self, program: &str, path: &str) -> Self {
            self.on_path
                .borrow_mut()
                .insert(program.to_string(), PathBuf::from(path));
            self
        }

        /// Every command run so far, in order.
        #[must_use]
        pub fn calls(&self) -> Vec<Cmd> {
            self.calls.borrow().clone()
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, cmd: &Cmd) -> Result<Output> {
            self.calls.borrow_mut().push(cmd.clone());
            let hit = self
                .scoped
                .borrow()
                .iter()
                .find(|(p, part, _)| {
                    *p == cmd.program && cmd.args.iter().any(|a| a.contains(part.as_str()))
                })
                .map(|(_, _, o)| o.clone());
            if let Some(o) = hit {
                return Ok(o);
            }
            match self.responses.borrow().get(&cmd.program) {
                Some(o) => Ok(o.clone()),
                None => bail!("cannot run `{}`: not scripted", cmd.program),
            }
        }

        fn which(&self, program: &str) -> Option<PathBuf> {
            self.on_path.borrow().get(program).cloned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_runner_records_and_answers() {
        let r = FakeRunner::new()
            .with_output("git", 0, "ok")
            .with_tool("git", "/usr/bin/git");
        assert_eq!(r.run(&Cmd::new("git", &[])).unwrap().stdout, "ok");
        assert!(r.run(&Cmd::new("nope", &[])).is_err());
        assert_eq!(r.calls().len(), 2);
        assert_eq!(r.which("git"), Some(PathBuf::from("/usr/bin/git")));
        assert_eq!(r.which("nope"), None);
    }

    #[test]
    fn system_runner_spawn_failure_is_err() {
        assert!(
            SystemRunner
                .run(&Cmd::new("definitely-not-a-program-xyz", &[]))
                .is_err()
        );
    }
}
