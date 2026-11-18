# buildkit-rust

<p align="center">
  <a href="https://github.com/medaminben/buildkit-rust/actions/workflows/self-test.yml"><img alt="Self-test" src="https://github.com/medaminben/buildkit-rust/actions/workflows/self-test.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/medaminben/buildkit-rust/tags"><img alt="version" src="https://img.shields.io/github/v/tag/medaminben/buildkit-rust?label=version"></a>
  <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-edition%202024-b7410e">
  <img alt="platform: Windows | Linux" src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-555">
  <a href="LICENSE"><img alt="license: MIT" src="https://img.shields.io/github/license/medaminben/buildkit-rust"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="docs/CI_CD.md">CI/CD</a> ·
  <a href="https://github.com/medaminben/buildkit-rust/tags">Versions</a>
</p>

A Rust and Cargo kit for products: a generator, versioning from git tags, installers, shared lint
configuration and reusable GitHub workflows. It is the Rust counterpart of the C++ and CMake
[buildkit](https://github.com/medaminben/buildkit). A product keeps its code; buildkit-rust
supplies the project layout, the version rules, the packaging, CI and releases.

It does not wrap `cargo build`, `cargo test` or dependency resolution. Cargo does those.

## Quick start

### A new product, from a bare machine

Clone buildkit-rust at a release tag, run the bootstrap once, then generate the product in a new,
empty folder outside the checkout. The bootstrap installs Rust, `cargo-bk` and the other tools.
It is safe to run again. It ends with `cargo bk doctor`, which names the fix for anything still
missing; the bootstrap exits 0 even when doctor reports a problem.

Linux (Debian or Ubuntu; run `sudo apt-get install git` first if git is missing):

```bash
git clone --branch v0.1.0 https://github.com/medaminben/buildkit-rust.git ~/dev/buildkit-rust
~/dev/buildkit-rust/bootstrap/bootstrap.sh
mkdir ~/dev/MyApp && cd ~/dev/MyApp
cargo bk new --repo owner/MyApp MyApp
```

Windows (PowerShell; run `winget install Git.Git` first if git is missing):

```powershell
git clone --branch v0.1.0 https://github.com/medaminben/buildkit-rust.git $HOME\dev\buildkit-rust
powershell -ExecutionPolicy Bypass -File $HOME\dev\buildkit-rust\bootstrap\bootstrap.ps1
mkdir $HOME\dev\MyApp; cd $HOME\dev\MyApp
cargo bk new --repo owner/MyApp MyApp
```

Open a new terminal after the bootstrap so `cargo` is on `PATH`. The bootstrap options and the
groups it installs are in [bootstrap/README.md](bootstrap/README.md).

`cargo bk new [NAME] [CRATE...]` takes these arguments, all optional:

| Argument | Meaning |
|---|---|
| `NAME` | Product name; default: the folder name |
| `CRATE...` | Library crates after the name; default: `core` |
| `--repo <owner/repo>` | Public product on GitHub: also writes `LICENSE` (MIT), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` and the issue and PR templates |
| `--flow direct\|beta` | Branch flow, default `direct`: feature branches merge into `main`. With `beta` they merge into `Beta`, which is promoted to `main` |
| `--no-build` | Skip the first build, test and run |

Existing files are never overwritten, so running the command again is safe. It writes the
workspace, one library per crate, a console app and an egui GUI app, the lint files, the
installer files, the workflow callers, the VS Code settings, the commit hooks, a README, an
architecture page and a user manual. It then builds, tests and runs the console app. Later
additions are in [docs/ADDING_CRATES.md](docs/ADDING_CRATES.md).

The generated workflows and the `buildkit` dependency name the version of the cargo-bk that
wrote them, and the first build fetches that tag from GitHub, so the tag must be published. To
try an unreleased buildkit-rust, see [Local checkout](#local-checkout).

### An existing product

A collaborator clones the product and runs its setup script. It installs `cargo bk` at the
version the product pins (by running the bootstrap from that tag when needed), the system
packages (it asks first unless `--yes`), the commit hooks and the dependencies. It then runs
`cargo bk doctor`, `cargo build` and `cargo test`.

```bash
git clone https://github.com/owner/MyApp.git && cd MyApp
scripts/setup.sh              # --yes skips the questions, --dry-run prints the steps
```

```powershell
git clone https://github.com/owner/MyApp.git; cd MyApp
powershell -ExecutionPolicy Bypass -File scripts\setup.ps1
```

### Only the command

```bash
cargo install --git https://github.com/medaminben/buildkit-rust --tag v0.1.0 cargo-bk --locked
```

This installs `cargo-bk` and nothing else; the bootstrap installs the rest.

## Commands

`cargo bk <command>` runs inside a product, except `new`, which runs in an empty folder, and
`doctor --machine`, which checks the machine only.

| Command | What it does |
|---|---|
| `new [NAME] [CRATE...]` | Creates a product in the current folder |
| `add-crate <name>` | Adds a library crate under `crates/` |
| `add-app <name> --console\|--gui` | Adds an app under `apps/` |
| `version` | Prints the product version from git tags |
| `package [--format inno\|appimage\|deb\|all]` | Builds the installers into `build/dist/` |
| `manual [--check]` | Builds the user manual into `build/manual/`, or checks it |
| `doctor [--machine]` | Checks the machine and the product against what they need |
| `check-pins` | Fails when the buildkit-rust version pins disagree |
| `upgrade <version>` | Moves every pin to the running cargo-bk version |
| `link [path] [--rewrite-dep] [--force]` | Points the product at a local buildkit-rust |
| `unlink [--rewrite-dep]` | Undoes `link` |
| `repo-setup [--dry-run] [--checks a,b]` | Creates the GitHub labels, the `Beta` branch and branch protection |

`cargo bk --version` prints the cargo-bk version. The [manual](docs/manual/content/en/commands.md)
lists every command and option.

## Versioning

The product version comes from git tags `vX.Y.Z` (stable) and `vX.Y.Z-beta.N` (Beta branch),
with the rules of buildkit. A product calls one function from its `build.rs`:

```rust
fn main() {
    buildkit::emit_version(env!("CARGO_PKG_VERSION"));
}
```

The code then reads `env!("BK_VERSION")` (`X.Y.Z`) and `env!("BK_VERSION_FULL")`. The full
version is `X.Y.Z` or `X.Y.Z-beta.N`, followed by `+<ahead>.g<sha7>` when HEAD is past the tag.
Without git, without a matching tag, or when the workspace root is not the top level of its own
repository, the version is the fallback and the full version is `<fallback>-dev`. The `version`
field in `Cargo.toml` is not the source of truth; the tag is.

## Local checkout

To work on buildkit-rust and a product at the same time, point the product at your checkout
without committing anything:

```bash
cargo bk link ../buildkit-rust
cargo install --path ../buildkit-rust/crates/cargo-bk --locked
```

`link` writes a marked `[patch]` block into `.cargo/config.toml` of the product's parent folder,
which Cargo reads for every project below it. It takes effect only once the tag the product
pins is published, because Cargo still fetches the git source it patches.

For a version that is not published, use `cargo bk link --rewrite-dep ../buildkit-rust`. It
rewrites the `buildkit` dependency in the product's root `Cargo.toml` to the local path. Do not
commit that change. CI does the same through the `buildkit-ref` input.

`cargo bk unlink` removes the first form and `cargo bk unlink --rewrite-dep` restores the git
dependency.

## Build output

All build output goes under `build/` of the repository: Cargo's target directory
(`build/target`, set in `.cargo/config.toml`), the stage folder, the installers in `build/dist`,
the manual in `build/manual` and downloaded tools in `build/tools`.

## Tests of buildkit-rust itself

```bash
cargo test --workspace
python3 -m unittest discover -s tools/release -p 'test_*.py'
python3 -m unittest discover -s tools/manual -p 'test_*.py'
```

`cargo test --workspace -- --include-ignored` also runs the tests that build a generated
product. [self-test.yml](.github/workflows/self-test.yml) runs all of this, and the whole path
from bootstrap to installers, on clean Ubuntu and Windows runners. It also runs
`cargo bk check-pins`, `cargo bk manual --check` and `cargo deny` on this repository.

## Documentation

| Doc | Topic |
|---|---|
| [Architecture](docs/ARCHITECTURE.md) | The two crates, the templates, the workflows and how they fit |
| [CI/CD](docs/CI_CD.md) | Reusable workflows with their inputs, labels, the Beta and main flow |
| [Adding crates](docs/ADDING_CRATES.md) | `add-crate`, `add-app` and the layout tables |
| [Packaging](docs/PACKAGING.md) | The three installer formats, the metadata and the file names |
| [Migration](docs/MIGRATION.md) | What replaces each feature of the C++ buildkit |
| [Manual](docs/manual/content/en/index.md) | Every command, option and setting, and the error messages |
| [Bootstrap](bootstrap/README.md) | The machine setup scripts |
| [Contributing](CONTRIBUTING.md) | Working on buildkit-rust itself |

## License

MIT, see [LICENSE](LICENSE).
