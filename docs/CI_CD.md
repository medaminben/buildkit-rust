# CI / CD

buildkit-rust ships three reusable workflows. A product calls them from short workflow files
and pins them to a buildkit-rust tag, for example
`medaminben/buildkit-rust/.github/workflows/ci.yml@v0.1.0`. `cargo bk new` writes these callers.
The header comment of each workflow in `.github/workflows/` is the reference for its inputs.

The tag in every `uses:` line must equal `[workspace.metadata.buildkit] version` in the product's
`Cargo.toml`. `cargo bk check-pins` checks it in the `lint` job, and `cargo bk upgrade` moves all
of them.

## `ci.yml`

Jobs:

| Job | Runs on | Content |
|---|---|---|
| `lint` | ubuntu-latest | `cargo bk check-pins`, `cargo bk manual --check`, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`, commitlint on the commits of a pull request. Skipped when nothing is enabled |
| `test` | ubuntu-latest, windows-latest | `cargo build --locked`, `cargo test --locked` and, with `release: true`, `cargo test --release --locked` |
| `coverage` | ubuntu-latest | Optional: `cargo llvm-cov`, the lcov report as the artifact `coverage-lcov` |
| `miri` | ubuntu-latest | Optional: `cargo +nightly miri test` on the crates in `miri-crates` |
| `package` | ubuntu-latest, windows-latest | Optional: `cargo bk package`, the installers as the artifact `installers-<os>`. Proves the installers still build |

The build dependencies of eframe (`libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`,
`libxcursor-dev`, `libxrandr-dev`, `libxi-dev`, `libgl1-mesa-dev`) are always installed on
Linux, then the packages from `apt-packages`. Caching uses `Swatinem/rust-cache`.

When `buildkit-ref` is empty, `cargo-bk` is installed from the tag `v<version>` of the product's
pin and the product builds against its own pinned git tag. When it is set, the workflow checks
buildkit-rust out at that ref, moves the checkout to `$RUNNER_TEMP/buildkit-rust` (outside the
product workspace), installs `cargo-bk` from it and runs `cargo bk link --rewrite-dep`. The
product then builds against that checkout, which is how a product uses a buildkit-rust version
that has no tag yet. It is the counterpart of exporting `BUILDKIT_DIR` in buildkit.

`format-exclude` may span several lines; newlines count as spaces.

| Input | Default | Meaning |
|---|---|---|
| `apt-packages` | empty | Extra apt packages for the Linux jobs, separated by spaces. Empty: the product's `system-deps.apt` |
| `lint` | `true` | Master switch: `false` turns off all four lint checks, `true` lets the next four decide |
| `lint-format` | `true` | `cargo fmt --check`. Only used when `lint` is `true` |
| `lint-clippy` | `true` | `cargo clippy --all-targets -- -D warnings`. Only used when `lint` is `true` |
| `lint-deny` | `true` | `cargo deny check`. Only used when `lint` is `true` |
| `lint-commits` | `true` | commitlint on pull request commits. Only used when `lint` is `true` |
| `format-exclude` | empty | Git pathspec globs that the format check skips, separated by spaces or newlines, for example `vendor/**`. With excludes the check runs `rustfmt --check` on the remaining `.rs` files, so a skipped file that a checked file declares as a module is still read |
| `xvfb` | `false` | Run `cargo test` and the coverage run under `xvfb-run -a` on Linux (GUI tests) |
| `timeout-minutes` | `45` | Time limit of each job; a caller job of a reusable workflow cannot set it |
| `release` | `true` | Also run `cargo test --release` in the `test` job |
| `coverage` | `false` | Run `cargo llvm-cov` and upload the lcov report |
| `miri-crates` | empty | Package names, separated by spaces or commas, that `cargo +nightly miri test` runs on. Empty: no `miri` job |
| `package` | `false` | Run `cargo bk package` on both systems and upload the installers |
| `buildkit-repository` | `medaminben/buildkit-rust` | Repository that provides `cargo-bk` |
| `buildkit-ref` | empty | Tag, branch or SHA of buildkit-rust. Empty: the product's own pin. Set: see above |
| `working-directory` | `.` | Folder of the product inside the repository. The self-test uses it for the committed fixture product |

Precedence of the lint switches: `lint: false` wins, whatever the four single inputs say. With
`lint: true` each single input decides, and the job is skipped when all four are `false`.
`cargo bk check-pins` and `cargo bk manual --check` run whenever the job runs.

There is no hook for extra steps. A check such as an install smoke test belongs into the
product as a Cargo test, so every system runs it.

```yaml
# .github/workflows/ci.yml in the product
name: CI
on:
  push:
    branches: [main, Beta]
  pull_request:
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
jobs:
  ci:
    uses: medaminben/buildkit-rust/.github/workflows/ci.yml@v0.1.0
    permissions:
      contents: read
    with:
      apt-packages: "libfoo-dev"
      coverage: true
      miri-crates: "myapp-core"
      package: true
      xvfb: true
      format-exclude: "vendor/**"
```

Concurrency is set by the caller, not by the reusable workflow. The generated caller lists
`main` and `Beta` as push branches for `--flow beta` and only `main` for `--flow direct`.

## `codeql.yml`

CodeQL for Rust, with `build-mode: none`, so no build is needed. It is kept apart from `ci.yml`
so that only this job needs `security-events: write`. Private repositories need code scanning
enabled. It takes no inputs.

```yaml
# .github/workflows/codeql.yml in the product
name: CodeQL
on:
  push:
    branches: [main, Beta]
  pull_request:
  schedule:
    - cron: "17 4 * * 1"
jobs:
  codeql:
    uses: medaminben/buildkit-rust/.github/workflows/codeql.yml@v0.1.0
    permissions:
      security-events: write
      actions: read
      contents: read
```

## `release.yml`

Runs when a pull request is closed and was merged into `main` or `Beta`. It tags the merge
commit, creates the GitHub release (a pre-release on `Beta`) with the commit subjects since the
previous tag as notes and, if given, starts the installer workflow on the new tag. It uses
`tools/release/plan_release.py` from buildkit-rust.

| Input | Default | Meaning |
|---|---|---|
| `installer-workflow` | empty | Workflow file of the caller, started with `gh workflow run <file> --ref <tag>` |
| `buildkit-repository` | `medaminben/buildkit-rust` | Repository that provides `tools/release` |
| `buildkit-ref` | empty | Tag, branch or SHA of `tools/release`. Empty: the tag `v<version>` of the caller's pin |

```yaml
# .github/workflows/release.yml in the product
name: Release
on:
  pull_request_target:
    types: [closed]
    branches: [main, Beta]
jobs:
  release:
    uses: medaminben/buildkit-rust/.github/workflows/release.yml@v0.1.0
    permissions:
      contents: write
      actions: write
    with:
      installer-workflow: installer.yml
```

The installer workflow must declare `on: workflow_dispatch` in the product's repository.

The generated caller triggers on `pull_request_target`, so merged pull requests from forks get
a write token. This is safe because `release.yml` never checks out or runs code of the pull
request. Its first checkout has no `ref`, so it is the pull request's base branch, used only for
history and tags. buildkit-rust comes from a fixed repository and ref, and the pull request
contributes only its labels, branch names and merge commit SHA, which reach the shell as
environment variables. A caller that triggers on `pull_request` instead gets a read-only token
for fork pull requests, and tagging fails.

Re-running a failed release job is safe. When the merge commit already carries a tag of the
planned kind (a stable tag on `main`, a beta tag on `Beta`), that tag is reused and not created
again, and the GitHub release is created only if it is missing.

### Labels

The bump comes from the label of the merged pull request: `semver:major`, `semver:minor`,
`semver:patch` (also the default without a label) or `semver:none` (no release).
`cargo bk repo-setup` creates these labels and `automerge`.

### Beta and main flow

With `flow = "beta"`, features merge into `Beta` through pull requests, and `Beta` is promoted to
`main` when the owner asks. With `flow = "direct"`, features merge into `main`.

- `Beta` gets `vX.Y.Z-beta.N` pre-releases. An open series (the highest beta base is greater
  than the latest stable) is continued, as long as its base is at least the one the labels ask
  for. Otherwise `-beta.1` of the new base starts.
- `main` gets `vX.Y.Z`. A merge of `Beta` into `main` (promotion) releases the base version of
  the open beta, so `v1.3.0-beta.4` is promoted as `v1.3.0`. Without an open beta the labels
  decide as usual, and `semver:none` still means no release.
- Promote with a merge commit. A squash merge hides the beta tags from the merge commit, so the
  release falls back to the label bump.
- Tags that are not strictly `vX.Y.Z` or `vX.Y.Z-beta.N` are ignored. The same tags are what
  `buildkit::emit_version` reads, so the binary version follows the release.

## The installer caller

`installer.yml` is generated into the product and is not reusable. It runs on
`workflow_dispatch`, which `release.yml` triggers with the new tag. A Windows job runs
`cargo bk package --format inno` and a Linux job runs `cargo bk package --format all`. Each
uploads `build/dist/*` as a workflow artifact and, when the ref is a tag, attaches the files to
the release with `gh release upload --clobber`. The fixed-name `<Name>_Setup.exe` is among
them, so `releases/latest/download/<Name>_Setup.exe` links keep working. Both jobs read the
pin from `Cargo.toml` and install `cargo-bk` at that tag. File names are in
[PACKAGING.md](PACKAGING.md).

Run it by hand on a tag to attach the installers to that release, or on a branch for artifacts
only.

## Repository setup

`cargo bk repo-setup` applies the GitHub side through `gh`. The owner runs it once, never
automatically. It creates the labels, creates `Beta` from `main` when the flow is `beta` and the
branch is missing, and protects `main` (and `Beta`) with required status checks. The default
checks are `ci / lint`, `ci / test (ubuntu-latest)` and `ci / test (windows-latest)`. Use
`--dry-run` to print the commands and `--checks` to replace the names.

## Local parity

Everything CI does to build, test, version and package runs locally:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo deny check
cargo bk check-pins
cargo bk manual --check
cargo test --locked
cargo bk package
```

Commit checks need Node: `npm install` installs the hooks, and
`npx commitlint --config lint/commitlint.config.cjs --from origin/main --to HEAD` checks a range.

## buildkit-rust's own CI

[`self-test.yml`](../.github/workflows/self-test.yml) runs on pushes to `main` and `Beta`, on
pull requests and by hand.

| Job | Content |
|---|---|
| `scripts` | `bash -n` and `shellcheck` on the shell scripts, the bootstrap tests, PSScriptAnalyzer and Pester on Windows |
| `crates` | Python unit tests, `cargo bk check-pins` and `cargo bk manual --check` on this repository, `cargo deny --config lint/deny.toml check`, then `cargo test --workspace --locked --no-fail-fast -- --include-ignored` on both systems |
| `bootstrap-clean` | The bootstrap on a clean runner, a second run that must install nothing, and `cargo bk doctor --machine` |
| `new-project` | `cargo bk new Foo core physics --repo x/foo`, the expected files, no unfilled placeholders, a second run that creates nothing, `.gitignore`, the manual, a fresh clone that tests and runs, and `cargo bk package` |
| `fixture-ci` | The reusable `ci.yml` on `tests/fixtures/foo` with `buildkit-ref` set to the commit and `package: true` |

A second bootstrap run reinstalls `cargo-bk` when the clone is not exactly at a tag. The
`bootstrap-clean` job allows that one line and fails on any other install.

### Releasing buildkit-rust

buildkit-rust does not use `release.yml` or the `semver:*` labels for itself. Its version is
`[workspace.metadata.buildkit] version` in the root `Cargo.toml`, which the docs, the workflow
examples and the version `cargo-bk` reports all follow. To release:

1. In a pull request, run `cargo bk upgrade X.Y.Z`. Inside buildkit-rust it accepts any `X.Y.Z`
   and moves the root version, the examples in `README.md`, `docs/` and the workflow comments,
   and `Cargo.lock`.
2. Merge it. [`self-release.yml`](../.github/workflows/self-release.yml) runs on the push to
   `main`, tags `vX.Y.Z` on that commit and creates the GitHub release, with the commit subjects
   since the previous tag as notes.

A push whose version is already tagged releases nothing, so ordinary pull requests need no
label. A version lower than the latest release fails the job.
