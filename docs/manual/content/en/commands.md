---
title: Commands
order: 2
summary: Every cargo bk command with its arguments and options.
---

## Using the commands

Run `cargo bk <command>`. The program is also installed as `cargo-bk`, and `cargo-bk <command>` does the same. Run a command inside the product, in its root or any folder below it. The exceptions are `new`, which runs in an empty folder, and `doctor --machine`, which does not need a product.

`cargo bk --version` (or `-V`) prints the version of `cargo-bk`. A command that works on a product warns when the product pins another version. See [Troubleshooting](troubleshooting.md#any).

Every command exits with 0 on success and 1 on failure. The failure message goes to the error output.

| Command | Purpose |
| --- | --- |
| [new](#new) | Create a product |
| [add-crate](#add-crate) | Add a library crate |
| [add-app](#add-app) | Add an app |
| [version](#version) | Print the product version |
| [package](#package) | Build the installers |
| [manual](#manual) | Build or check the user manual |
| [doctor](#doctor) | Check the machine and the product |
| [check-pins](#check-pins) | Check that the version pins agree |
| [upgrade](#upgrade) | Move the pins to the running version |
| [link](#link) | Use a local buildkit-rust checkout |
| [unlink](#unlink) | Stop using it |
| [repo-setup](#repo-setup) | Set up the GitHub repository |

## new {#new}

`cargo bk new [NAME] [CRATE...] [--repo owner/repo] [--flow direct or beta] [--no-build]`

Creates a product in the current folder. The folder must be empty, hold only a `.git` folder, or already be a product that `new` made. Existing files are never overwritten, so running the command again is safe. It prints `created <n> files, kept <m>` at the end.

| Argument or option | Meaning |
| --- | --- |
| `NAME` | The product name. Default: the name of the folder. ASCII letters, digits, spaces, `-` and `_`; it must start with a letter. |
| `CRATE...` | Names of the library crates. Default: one crate named `core`. |
| `--repo owner/repo` | The GitHub repository. It also writes `LICENSE` (MIT, with the owner as holder), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` and the issue and pull request templates, and it is stored as `repo` in the settings. |
| `--flow direct` or `--flow beta` | How changes reach `main`. `direct` (default): feature branches merge into `main`. `beta`: they merge into `Beta`, which is promoted to `main`. Stored as `flow` in the settings. |
| `--no-build` | Skip the first `cargo build`, `cargo test` and `cargo run` of the console app. |

It writes the workspace `Cargo.toml` with the settings and the lint levels, `rust-toolchain.toml`, `.cargo/config.toml`, the lint files, `.gitignore`, one library per crate, a console app and a GUI app, the Inno Setup script, the `.desktop` file and the icon, the setup scripts, the four workflow callers, the VS Code files, the commit hooks, a README, an architecture page and a user manual.

## add-crate {#add-crate}

`cargo bk add-crate <name>`

Adds the library `crates/<name>` with the package name `<product>-<name>`, and a row for it in the layout tables of `README.md` and `docs/ARCHITECTURE.md`. It stops when the crate exists.

## add-app {#add-app}

`cargo bk add-app <name> --console` or `cargo bk add-app <name> --gui`

Adds the app `apps/<product>-<name>` and its layout rows. Exactly one of `--console` and `--gui` is required. The app depends on the `core` crate, or on the first crate when there is no `core`. The list `apps` of the packaging settings does not change; add the app there if the installer should hold it.

## version {#version}

`cargo bk version`

Prints the full version of the product, from the git tags, with the rules the apps use: the nearest tag `vX.Y.Z` or `vX.Y.Z-beta.N`, followed by `+<ahead>.g<sha7>` when HEAD is past the tag. Without a tag, outside the top level of the repository, or without git, it prints the fallback and `-dev`. The fallback is the `version` of a `[package]` in the root `Cargo.toml`, or `0.0.0` when the root has none.

## package {#package}

`cargo bk package [--format inno, appimage, deb or all]`

Builds the installers into `build/dist` and prints the path of each file. The default and `all` mean every format of the current system.

| Format | System | File |
| --- | --- | --- |
| `inno` | Windows | `<Name>_Setup_<version>.exe` and a copy `<Name>_Setup.exe` |
| `appimage` | Linux | `<Name>-<version>-x86_64.AppImage` |
| `deb` | Linux | `<product>_<debversion>_amd64.deb` |

`<Name>` is the display name with spaces written as `-`. In the `.deb` file name the first `-` of the version becomes `~`. The command needs a committed `Cargo.lock`, because it builds with `--locked`. It builds each app in release mode, stages the files in `build/stage`, writes `THIRD_PARTY_NOTICES.txt` with `cargo-about` and then builds the formats. It warns when the version ends in `-dev`, which usually means a shallow clone. See [Packaging settings](configuration.md#package-keys).

## manual {#manual}

`cargo bk manual [--check]`

Renders `docs/manual` into `build/manual`, with one folder per language and a language chooser. It needs Python 3.11 or newer. With `--check` it writes nothing and fails when a page has a problem: a missing front matter key, a page that exists in one language only, a broken link, anchor or image, or an external web address. This command does not need the packaging settings. See [Manual pages](configuration.md#manual-pages).

## doctor {#doctor}

`cargo bk doctor [--machine]`

Checks the machine against what the product needs, and prints one line for each check, with the fix below each problem. It exits with 1 when a required check fails. Without `--machine`, inside a product, it also checks the product; outside a product it checks the machine only. `--machine` checks the machine only, even inside a product.

Machine checks: `rustc` and `cargo`, `rustfmt`, `clippy`, `cargo-bk`, `cargo-deny`, `cargo-llvm-cov`, `cargo-about`, `node`, `python` (3.11 or newer) and `gh` (optional). On Linux also `dpkg-deb` and `appimagetool`. On Windows also `ISCC` and `MSVC`.

Product checks: `rustc` must be the version in `rust-toolchain.toml`, `cargo-bk` must be the version the product pins, `husky` (the commit hooks) must be installed, every package in `system-deps.apt` must be installed on Linux, and the optional line `cargo bk link` tells whether a link is active.

## check-pins {#check-pins}

`cargo bk check-pins`

Compares every place that names a buildkit-rust version with `[workspace.metadata.buildkit] version`: the `tag` of the `buildkit` dependency, each `uses: medaminben/buildkit-rust/.github/workflows/...@vX.Y.Z` line in `.github/workflows`, and the first line `# buildkit-rust vX.Y.Z` of `rustfmt.toml`, `clippy.toml` and `deny.toml`. It prints each place that differs, with file and line, and exits with 1. It also warns, without failing, when a copied lint file differs from the copy inside `cargo-bk`.

In the buildkit-rust repository itself it also reads the examples `buildkit-rust/.github/workflows/<file>@v<version>` and `--tag v<version>` in `README.md` and every Markdown file under `docs`. There it does not compare the lint files, because they are the originals.

## upgrade {#upgrade}

`cargo bk upgrade <version>`

Moves every pin to `<version>`, with or without a leading `v`: the metadata version, the dependency tag, the workflow lines, and the lint files, which are replaced with the copies of this `cargo-bk`. It prints `updated <file>` for each file that changed, or `already at v<version>`. The version must be the one of the running `cargo-bk`; install it first.

## link {#link}

`cargo bk link [path] [--rewrite-dep] [--force]`

Points the product at a local checkout of buildkit-rust. The default path is `../buildkit-rust`, next to the product.

| Option | Meaning |
| --- | --- |
| none | Writes a block into `.cargo/config.toml` in the parent folder of the product, which patches the `buildkit` git source to the checkout. Cargo applies it to every project below that folder. It works only when the tag the product pins is published, because Cargo still fetches the git source. A second run replaces the block. |
| `--rewrite-dep` | Rewrites the `buildkit` dependency in the root `Cargo.toml` to the local path. Do not commit this. It works for a version without a tag, and the CI workflows use it through `buildkit-ref`. |
| `--force` | Writes the block even when the parent folder is your home folder, where it would become the global Cargo configuration. A filesystem root is always refused. |

After it succeeds, the command prints the `cargo install --path` line that installs the `cargo-bk` of the checkout.

## unlink {#unlink}

`cargo bk unlink [--rewrite-dep]`

Without the option it removes the block that `link` wrote, and deletes the config file when nothing else is left. It prints `unlinked` or `no link to remove`. With `--rewrite-dep` it restores the git dependency, using the tag `v<version>` from the settings.

## repo-setup {#repo-setup}

`cargo bk repo-setup [--dry-run] [--checks NAME,NAME]`

Applies the GitHub side of the pipeline through the `gh` command, for the repository of the current clone. You run it once, by hand. It is safe to run again.

1. It creates or updates the labels `semver:major`, `semver:minor`, `semver:patch`, `semver:none` and `automerge`.
2. With `flow = "beta"` it creates the `Beta` branch from `main` when it is missing.
3. It protects `main`, and `Beta` with the beta flow, so that the required checks must pass.

| Option | Meaning |
| --- | --- |
| `--dry-run` | Prints the commands that change something, and runs none of them. |
| `--checks NAME,NAME` | The names of the required checks, separated by commas. They replace the defaults `ci / lint`, `ci / test (ubuntu-latest)` and `ci / test (windows-latest)`. |

The flow comes from `flow` in `[workspace.metadata.buildkit]`. The protection requires the checks, does not require a review and does not apply to administrators.
