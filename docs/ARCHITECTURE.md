# Architecture

buildkit-rust is a Cargo workspace with two crates, a tree of templates that `cargo-bk` embeds,
three reusable workflows, two bootstrap scripts and two Python tools. Cargo builds, tests and
resolves dependencies; buildkit-rust adds what Cargo does not: a generator, the version rules,
installers, shared lint configuration and the CI and release flow.

## Layout

| Path | Purpose |
|---|---|
| [`crates/buildkit`](../crates/buildkit) | Library. Derives the product version from git tags. A product uses it from `build.rs` |
| [`crates/cargo-bk`](../crates/cargo-bk) | Binary `cargo-bk`, so `cargo bk ...` works. Every command in the [manual](manual/content/en/commands.md) |
| [`templates/`](../templates) | The files `cargo bk new`, `add-crate` and `add-app` write, embedded into `cargo-bk` at build time |
| [`lint/`](../lint) | `rustfmt.toml`, `clippy.toml`, `deny.toml` and the commitlint config, for buildkit-rust itself |
| [`bootstrap/`](../bootstrap) | `bootstrap.sh` and `bootstrap.ps1`: machine setup |
| [`tools/release/`](../tools/release) | `next_version.py` and `plan_release.py`: tag planning for `release.yml` |
| [`tools/manual/`](../tools/manual) | `build_manual.py` and its assets: renders the manual, embedded into `cargo-bk` |
| [`.github/workflows/`](../.github/workflows) | `ci.yml`, `codeql.yml` and `release.yml` (reusable), `self-test.yml` and `self-release.yml` (this repository's own CI and release) |
| [`tests/fixtures/`](../tests/fixtures) | `foo`: a committed generated product, kept equal to a fresh `cargo bk new` |

## The `buildkit` crate

`buildkit::emit_version("x.y.z")` runs in a product's `build.rs`. It finds the
workspace root above `CARGO_MANIFEST_DIR`, asks `git` for the version and prints
`cargo:rustc-env=BK_VERSION=...` and `BK_VERSION_FULL=...`, plus `cargo:rerun-if-changed` lines
for `.git/HEAD`, the current branch ref, `packed-refs` and the tags, so a new tag or commit
changes the version without `cargo clean`.

The rules are those of buildkit's `buildkit_git_version`:

- The nearest tag `vX.Y.Z` or `vX.Y.Z-beta.N` reachable from HEAD. When one commit has several
  tags, a stable tag wins over a beta tag, then the highest version wins.
- `full` is `X.Y.Z` or `X.Y.Z-beta.N`, followed by `+<ahead>.g<sha7>` when HEAD is past the tag.
- Without git, without a matching tag, or when the workspace root is not the top level of its
  own repository, `numeric` is the fallback and `full` is `<fallback>-dev`.

The crate calls the `git` command line through the `Git` trait, not a git library, so tests can
fake it and the build dependency stays small. `cargo-bk` uses the same `resolve` function for
`cargo bk version` and `cargo bk package`, so a binary, a release tag and an installer name
agree.

## The `cargo-bk` crate

`cli.rs` declares the commands with clap and `lib.rs` dispatches them to one module each under
`src/commands/`. Three pieces carry the rest:

- **Workspace** (`workspace.rs`) finds the nearest `Cargo.toml` with a `[workspace]` and reads
  `[workspace.metadata.buildkit]` and `[workspace.metadata.bk.package]` with `toml_edit`, so
  writing a manifest back keeps its formatting and comments.
- **Runner** (`exec.rs`) is the trait behind every external program: cargo, git, `gh`,
  `dpkg-deb`, `appimagetool`, `ISCC`, Python. Commands take a `&dyn Runner`; unit tests pass a
  `FakeRunner` that records the calls and returns scripted output.
- **Templates** (`templates.rs`) embed `templates/` with `include_dir`. A file ending in
  `.tmpl` is rendered: `{{name}}` in the content and `__name__` in a path segment are replaced,
  and an unknown placeholder is an error. Other files are copied as they are. `write_tree` never
  overwrites an existing file; it reports what it created and what it kept.

### `new`, `add-crate`, `add-app`

`new` writes the `project` tree once, the `crate` tree once per library, the console and GUI app
trees, and (with `--repo`) the `community` tree. `add-crate` and `add-app` write one more crate
or app and call `docs_rows.rs`, which appends a row to the table after `<!-- bk:layout -->` in
`README.md` and `docs/ARCHITECTURE.md` of the product and never rewrites other text. See
[ADDING_CRATES.md](ADDING_CRATES.md).

### `package`

`commands/package.rs` checks that `Cargo.lock` exists, resolves the version, builds each app
with `cargo build --release --locked -p <app>`, then stages and builds. `package/stage.rs` plans
and applies the install tree in `<root>/build/stage` (the binaries, `LICENSE` and the
`extra-files`). `package/notices.rs` runs `cargo about` to write `THIRD_PARTY_NOTICES.txt`. The
three builders, `inno.rs`, `appimage.rs` and `deb.rs`, share a `PackageCtx` (root, stage, output
folder, metadata, product name, maintainer, version) and write into `build/dist/`. Details are
in [PACKAGING.md](PACKAGING.md).

### Pins

A product records the buildkit-rust version it expects once, in
`[workspace.metadata.buildkit] version`, and repeats it in the `buildkit` git dependency, the
`uses: medaminben/buildkit-rust/...@vX.Y.Z` lines of its workflows and the header of its
copied lint files. `pins.rs` finds every site. `cargo bk check-pins` fails when one differs;
`cargo bk upgrade <version>` rewrites them all. The lint files are also compared with the copies
embedded in the running cargo-bk, and a difference is a warning, never a failure.

buildkit-rust applies the same check to itself. Its root `Cargo.toml` has
`[workspace.metadata.buildkit] version`, and when a repository has `crates/cargo-bk`,
`find_pins` also reads `README.md`, every `.md` file under `docs/` and its own workflows for
`buildkit-rust/.github/workflows/<file>@v<version>`, `--tag v<version>` and
`--branch v<version>`. A placeholder such
as `vX.Y.Z` is not a pin. The lint-file comparison is skipped there, because those files are the
originals. [self-test.yml](../.github/workflows/self-test.yml) runs
`cargo bk check-pins`. Inside buildkit-rust, `cargo bk upgrade X.Y.Z` accepts any release
version, moves the root version and every example together and leaves the lint files alone; see
[Releasing buildkit-rust](CI_CD.md#releasing-buildkit-rust).

### `link`

`link` has two forms because Cargo has no way to override a git dependency without either a
config or a manifest change. Both are described in the [README](../README.md#local-checkout).
The first writes a block between the markers `# >>> cargo bk link` and `# <<< cargo bk link`
into `<product's parent>/.cargo/config.toml`. The second edits the inline table
`buildkit = { ... }` in `[workspace.dependencies]`.

## Templates

`templates/project` holds what every product shares: the root `Cargo.toml` with
`[workspace.metadata.buildkit]`, `[workspace.metadata.bk.package]` and the lint levels;
`rust-toolchain.toml`; `.cargo/config.toml` with `target-dir = "build/target"`; the lint files;
`.gitignore`; the Inno Setup script; the `.desktop` file and icon; `scripts/setup.sh` and
`scripts/setup.ps1`; the four workflow callers (`ci.yml`, `codeql.yml`, `release.yml`,
`installer.yml`); `.vscode/`; the commit hooks; and a README, architecture page and manual.
`templates/crate`, `app-console` and `app-gui` hold one crate or app each. `templates/community`
holds the files written with `--repo`.

The generated callers are the only copy of workflow logic a product owns, and they only call
the reusable workflows at a pinned tag. The exception is `installer.yml`, which is not
reusable: it runs `cargo bk package` on Windows and Linux for a tag.

## Workflows

The three reusable workflows and `self-test.yml` are described in [CI_CD.md](CI_CD.md). Every
step that builds, versions or packages calls `cargo bk`, so the same command runs on a
developer machine and in CI. YAML only installs tools and chooses inputs.

With `buildkit-ref` set, `ci.yml` checks buildkit-rust out at that ref, moves it to
`$RUNNER_TEMP/buildkit-rust` (outside the product workspace), installs `cargo-bk` from it and
runs `cargo bk link --rewrite-dep`. This is how the self-test runs a product's CI before the tag
the product pins exists.

## Tests

| Layer | Where | What it proves |
|---|---|---|
| Unit | `src/` of both crates | Version rules, names, template rendering, staging, package control files, with fake git and fake runners |
| Integration | `crates/cargo-bk/tests/` | The binary on temporary products: `new`, `add`, `link`, pins, `repo-setup`, `doctor`, `manual`, setup scripts, the packages |
| Golden | `crates/cargo-bk/tests/golden.rs`, `tests/fixtures/foo` | A fresh `cargo bk new` equals the committed fixture product |
| Real build | `cargo test -- --include-ignored` | A generated product builds and runs; tools such as `dpkg-deb` run for real |
| Python | `tools/*/test_*.py` | Tag planning and the manual renderer |
| Scripts | `bootstrap/tests/` | `bootstrap.sh` (stubs) and `bootstrap.ps1` (Pester) |
| End to end | `self-test.yml` | Bootstrap, `cargo bk new`, build, test, manual, installers on clean runners; the fixture's own CI |

## Decisions

- **One shared tool, pinned to a tag.** The alternatives were copying an `xtask/` crate into each
  product, which drifts, and a `cargo-generate` template with packaging in YAML, which gives no
  local installers. A bumped tag updates a product in one step.
- **The tag is the version.** `version` fields in `Cargo.toml` stay at the fallback.
- **No crates.io.** Products use git dependencies pinned to tags.
- **Windows binaries are built on Windows.** There is no cross-compile or wine path.
- **One installer per product**, not per app, holding every app listed in the metadata.
