use cargo_bk::commands::new::Flow;
use cargo_bk::commands::repo_setup::{default_checks, plan_repo_setup, run};
use cargo_bk::exec::{Cmd, FakeRunner};
use cargo_bk::workspace::Workspace;
use std::fs;

fn checks(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

fn joined(c: &Cmd) -> String {
    format!("{} {}", c.program, c.args.join(" "))
}

fn is_label(c: &Cmd) -> bool {
    c.args.starts_with(&["label".into(), "create".into()])
}

fn is_protection(c: &Cmd) -> bool {
    c.args.iter().any(|a| a.ends_with("/protection"))
}

#[test]
fn default_checks_match_the_caller_job_names() {
    assert_eq!(
        default_checks(),
        checks(&[
            "ci / lint",
            "ci / test (ubuntu-latest)",
            "ci / test (windows-latest)"
        ])
    );
}

#[test]
fn direct_plan_has_five_labels_and_one_protection() {
    let plan = plan_repo_setup("o/r", Flow::Direct, &[], false, &default_checks(), "abc");
    assert_eq!(plan.iter().filter(|c| is_label(c)).count(), 5);
    assert_eq!(plan.iter().filter(|c| is_protection(c)).count(), 1);
    assert_eq!(plan.len(), 6);
    assert!(plan.iter().all(|c| c.program == "gh"));
    assert!(!plan.iter().any(|c| joined(c).contains("git/refs")));
}

#[test]
fn labels_use_the_exact_names_and_colours() {
    let plan = plan_repo_setup("o/r", Flow::Direct, &[], false, &default_checks(), "abc");
    let text: Vec<String> = plan.iter().filter(|c| is_label(c)).map(joined).collect();
    for (n, c) in [
        ("semver:major", "b60205"),
        ("semver:minor", "fbca04"),
        ("semver:patch", "0e8a16"),
        ("semver:none", "cccccc"),
        ("automerge", "5319e7"),
    ] {
        assert!(
            text.contains(&format!(
                "gh label create {n} --color {c} --force --repo o/r"
            )),
            "missing {n}: {text:?}"
        );
    }
}

#[test]
fn beta_without_beta_adds_ref_and_second_protection() {
    let plan = plan_repo_setup("o/r", Flow::Beta, &[], false, &default_checks(), "abc123");
    assert_eq!(plan.iter().filter(|c| is_protection(c)).count(), 2);
    let ref_cmd = plan
        .iter()
        .find(|c| joined(c).contains("repos/o/r/git/refs"))
        .expect("ref command");
    assert!(joined(ref_cmd).contains("ref=refs/heads/Beta"));
    assert!(joined(ref_cmd).contains("sha=abc123"));
    let ref_at = plan
        .iter()
        .position(|c| joined(c).contains("git/refs"))
        .unwrap();
    let beta_prot = plan
        .iter()
        .position(|c| joined(c).contains("branches/Beta/protection"))
        .unwrap();
    assert!(ref_at < beta_prot);
}

#[test]
fn beta_with_beta_has_no_ref_command() {
    let plan = plan_repo_setup("o/r", Flow::Beta, &[], true, &default_checks(), "abc");
    assert!(!plan.iter().any(|c| joined(c).contains("git/refs")));
    assert_eq!(plan.iter().filter(|c| is_protection(c)).count(), 2);
}

#[test]
fn protection_payload_is_exact() {
    let plan = plan_repo_setup("o/r", Flow::Direct, &[], false, &default_checks(), "abc");
    let p = plan.iter().find(|c| is_protection(c)).unwrap();
    let text = joined(p);
    assert!(text.starts_with("gh api -X PUT repos/o/r/branches/main/protection"));
    for a in [
        "required_status_checks[strict]=false",
        "required_status_checks[contexts][]=ci / lint",
        "required_status_checks[contexts][]=ci / test (ubuntu-latest)",
        "required_status_checks[contexts][]=ci / test (windows-latest)",
        "enforce_admins=false",
        "required_pull_request_reviews=null",
        "restrictions=null",
    ] {
        assert!(p.args.iter().any(|x| x == a), "missing {a}: {text}");
    }
}

#[test]
fn custom_checks_replace_the_defaults() {
    let plan = plan_repo_setup(
        "o/r",
        Flow::Direct,
        &[],
        false,
        &checks(&["self-test"]),
        "abc",
    );
    let text = joined(plan.iter().find(|c| is_protection(c)).unwrap());
    assert!(text.contains("required_status_checks[contexts][]=self-test"));
    assert!(!text.contains("ci / lint"));
    assert!(!text.contains("windows-latest"));
}

fn product(flow: Option<&str>) -> (tempfile::TempDir, Workspace) {
    let t = tempfile::tempdir().unwrap();
    let meta = flow.map_or(String::new(), |f| format!("flow = \"{f}\"\n"));
    fs::write(
        t.path().join("Cargo.toml"),
        format!("[workspace]\n[workspace.metadata.buildkit]\nversion = \"0.1.0\"\n{meta}"),
    )
    .unwrap();
    let ws = Workspace::find(t.path()).unwrap();
    (t, ws)
}

fn gh_calls(r: &FakeRunner) -> Vec<Cmd> {
    r.calls()
}

#[test]
fn dry_run_runs_only_reads_and_prints_the_commands() {
    let (_t, ws) = product(None);
    let r = FakeRunner::new().with_output("gh", 0, "o/r\n");
    let mut out = Vec::new();
    run(&ws, &r, true, None, &mut out).unwrap();
    let calls = gh_calls(&r);
    assert_eq!(calls.len(), 1, "direct flow reads only the repo name");
    assert!(calls.iter().all(|c| {
        let t = joined(c);
        t.starts_with("gh repo view")
            || t.starts_with("gh api repos/o/r/git/ref/heads/")
            || t.starts_with("gh api repos/o/r/branches/Beta")
    }));
    assert!(!calls.iter().any(|c| is_label(c) || is_protection(c)));
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("gh label create semver:major --color b60205 --force --repo o/r"));
    assert!(text.contains("branches/main/protection"));
    assert!(text.contains("'required_status_checks[contexts][]=ci / lint'"));
}

#[test]
fn real_run_executes_the_plan() {
    let (_t, ws) = product(Some("direct"));
    let r = FakeRunner::new().with_output("gh", 0, "o/r\n");
    let mut out = Vec::new();
    run(&ws, &r, false, Some(&checks(&["self-test"])), &mut out).unwrap();
    let calls = r.calls();
    assert_eq!(calls.iter().filter(|c| is_label(c)).count(), 5);
    assert_eq!(calls.iter().filter(|c| is_protection(c)).count(), 1);
}

#[test]
fn unknown_flow_is_an_error() {
    let (_t, ws) = product(Some("sideways"));
    let r = FakeRunner::new().with_output("gh", 0, "o/r\n");
    let err = run(&ws, &r, true, None, &mut Vec::new()).unwrap_err();
    assert!(err.to_string().contains("sideways"));
}

#[test]
fn gh_failure_is_an_error() {
    let (_t, ws) = product(None);
    let r = FakeRunner::new().with_failure("gh", 1, "not logged in");
    let err = run(&ws, &r, true, None, &mut Vec::new()).unwrap_err();
    assert!(format!("{err:#}").contains("not logged in"));
}
