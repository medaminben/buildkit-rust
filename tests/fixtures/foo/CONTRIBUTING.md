# Contributing to Foo

## Setup

Clone the repository and run the setup script. It installs `cargo bk` at the pinned version,
the system packages, the commit hooks and the dependencies, then builds and tests:

```bash
scripts/setup.sh        # Linux, macOS Git Bash
scripts/setup.ps1       # Windows PowerShell
```

Run it before anything else: the commit hooks and the pinned tools come from it.

## Workflow

Branch from `main` and open a pull request into `main`. A release is a tag `vX.Y.Z` on `main`.

Name a branch `<type>/<short-topic>`, for example `feat/export-csv` or `fix/crash-on-start`.

## Commits

Conventional Commits: `<type>(optional-scope): <description>` with the types `feat`, `fix`,
`docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. The commit hook
checks the message.

## Pull requests

- Label the PR with one of `semver:major`, `semver:minor`, `semver:patch` or `semver:none`. The
  label decides the next version.
- [ ] `cargo test --workspace` passes
- [ ] New behavior has a test
- [ ] Docs updated when behavior changes
- Merge only when CI is green.

## Style

- Format with `cargo fmt`. `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- Build output goes only under `build/`.
- Documentation is written in English.

Do not commit secrets or machine-local paths.
