//! `cargo bk repo-setup`: the GitHub side of the shared pipeline, applied through `gh`.

use std::io::Write;

use anyhow::{Result, bail};

use super::new::Flow;
use crate::exec::{Cmd, Output, Runner};
use crate::workspace::Workspace;

const LABELS: [(&str, &str); 5] = [
    ("semver:major", "b60205"),
    ("semver:minor", "fbca04"),
    ("semver:patch", "0e8a16"),
    ("semver:none", "cccccc"),
    ("automerge", "5319e7"),
];

/// The checks the generated caller workflow reports: the caller job id `ci`,
/// then the reusable workflow's job names (`lint`, and `test` per matrix OS).
#[must_use]
pub fn default_checks() -> Vec<String> {
    [
        "ci / lint",
        "ci / test (ubuntu-latest)",
        "ci / test (windows-latest)",
    ]
    .map(String::from)
    .to_vec()
}

fn gh(args: &[&str]) -> Cmd {
    Cmd {
        program: "gh".into(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        cwd: None,
        env: vec![],
    }
}

fn protection(repo: &str, branch: &str, checks: &[String]) -> Cmd {
    let mut c = gh(&[
        "api",
        "-X",
        "PUT",
        &format!("repos/{repo}/branches/{branch}/protection"),
        "-F",
        "required_status_checks[strict]=false",
    ]);
    for check in checks {
        c.args.push("-f".into());
        c.args
            .push(format!("required_status_checks[contexts][]={check}"));
    }
    for f in [
        "enforce_admins=false",
        "required_pull_request_reviews=null",
        "restrictions=null",
    ] {
        c.args.push("-F".into());
        c.args.push(f.into());
    }
    c
}

/// The mutating `gh` commands for `repo`, in order. Labels are created with
/// `--force`, so existing ones are only updated and `_existing_labels` needs no
/// filtering. `main_sha` is the commit the `Beta` branch starts from.
#[must_use]
pub fn plan_repo_setup(
    repo: &str,
    flow: Flow,
    _existing_labels: &[String],
    has_beta: bool,
    check_names: &[String],
    main_sha: &str,
) -> Vec<Cmd> {
    let mut plan: Vec<Cmd> = LABELS
        .iter()
        .map(|(n, c)| {
            gh(&[
                "label", "create", n, "--color", c, "--force", "--repo", repo,
            ])
        })
        .collect();
    if flow == Flow::Beta && !has_beta {
        plan.push(gh(&[
            "api",
            &format!("repos/{repo}/git/refs"),
            "-f",
            "ref=refs/heads/Beta",
            "-f",
            &format!("sha={main_sha}"),
        ]));
    }
    plan.push(protection(repo, "main", check_names));
    if flow == Flow::Beta {
        plan.push(protection(repo, "Beta", check_names));
    }
    plan
}

fn read(runner: &dyn Runner, args: &[&str]) -> Result<Output> {
    runner.run(&gh(args))
}

fn read_ok(runner: &dyn Runner, args: &[&str]) -> Result<String> {
    let o = read(runner, args)?;
    if o.status != 0 {
        bail!("`gh {}` failed: {}", args.join(" "), o.stderr.trim());
    }
    Ok(o.stdout.trim().to_string())
}

fn shell_word(a: &str) -> String {
    if a.chars().any(|c| c.is_whitespace() || "[]()'".contains(c)) {
        format!("'{}'", a.replace('\'', "'\\''"))
    } else {
        a.to_string()
    }
}

fn parse_flow(ws: &Workspace) -> Result<Flow> {
    match ws.bk_flow().as_deref() {
        None | Some("direct") => Ok(Flow::Direct),
        Some("beta") => Ok(Flow::Beta),
        Some(other) => {
            bail!("unknown flow `{other}` in [workspace.metadata.buildkit]; use direct or beta")
        }
    }
}

/// Reads the repository state, then runs the plan (or prints it with `dry_run`).
///
/// # Errors
/// When the flow is unknown, a `gh` read fails, or a planned command fails.
pub fn run(
    ws: &Workspace,
    runner: &dyn Runner,
    dry_run: bool,
    checks: Option<&[String]>,
    out: &mut dyn Write,
) -> Result<()> {
    let flow = parse_flow(ws)?;
    let repo = read_ok(
        runner,
        &[
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "--jq",
            ".nameWithOwner",
        ],
    )?;
    // Direct flow has no Beta branch: `true` plans no ref command, so the sha stays unused.
    let (mut has_beta, mut sha) = (true, String::new());
    if flow == Flow::Beta {
        let beta = read(runner, &["api", &format!("repos/{repo}/branches/Beta")])?;
        has_beta = match beta.status {
            0 => true,
            _ if beta.stderr.contains("HTTP 404") || beta.stderr.contains("Not Found") => false,
            _ => bail!("cannot read the Beta branch: {}", beta.stderr.trim()),
        };
        if !has_beta {
            sha = read_ok(
                runner,
                &[
                    "api",
                    &format!("repos/{repo}/git/ref/heads/main"),
                    "--jq",
                    ".object.sha",
                ],
            )?;
        }
    }
    let names = checks.map_or_else(default_checks, <[String]>::to_vec);
    let plan = plan_repo_setup(&repo, flow, &[], has_beta, &names, &sha);
    for cmd in &plan {
        let line = std::iter::once(cmd.program.as_str())
            .chain(cmd.args.iter().map(String::as_str))
            .map(shell_word)
            .collect::<Vec<_>>()
            .join(" ");
        if dry_run {
            writeln!(out, "{line}")?;
            continue;
        }
        let o = runner.run(cmd)?;
        if o.status != 0 {
            bail!("`{line}` failed: {}", o.stderr.trim());
        }
        writeln!(out, "ok: {line}")?;
    }
    Ok(())
}
