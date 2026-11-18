---
title: Troubleshooting
order: 4
summary: Every error and warning message of cargo bk, the bootstrap and the setup scripts, with the cause and the fix.
---

## How errors look

`cargo bk` prints a failure as `error: <message>` on the error output and exits with code 1. A warning starts with `warning:` and does not change the exit code. Where a message below shows a name in angle brackets, `cargo bk` prints the real name there. Some real messages put a name or a command between backticks; they are shown here without the backticks, because this manual uses backticks for labels.

The messages are grouped by the command that prints them.

## Any command {#any}

- `no command given; see cargo bk --help`: you ran `cargo bk` without a command. Pick one from [Commands](commands.md).
- `no Cargo workspace found from <folder>; run cargo bk inside a product`: no `Cargo.toml` with a `[workspace]` table exists in the folder or above it. Change into the product.
- `cannot read <file>` or `cannot parse <file>`: the manifest is unreadable or is not valid TOML. Fix the file.
- `warning: this product pins buildkit-rust v<pin>, cargo-bk is v<own>; run cargo bk doctor`: the installed `cargo-bk` is not the version the product pins. Install the pinned one. Doctor prints the command.
- `[workspace.metadata.bk.package] is missing`: the root `Cargo.toml` has no packaging table. See [Packaging settings](configuration.md#package-keys).
- `[workspace.metadata.bk.package] <key> is missing`: a required packaging key is missing. The keys are `display-name`, `main-app`, `apps`, `iss`, `desktop` and `icon`.

## new, add-crate and add-app {#new-errors}

- `folder is not empty and is not a buildkit-rust product`: `new` runs only in an empty folder, a folder that holds only `.git`, or a product that `new` made before. Use another folder.
- `cannot derive a name from the folder; pass one`: the folder name is not usable. Pass the product name as the first argument.
- `invalid name "<name>": use ASCII letters, digits, space, - or _; must start with a letter`: a product, crate or app name breaks the rule. This includes names with accents.
- `cargo <args> failed:` followed by the cargo error output: the first build, test or run of `new` failed. Fix the cause, or run `new` with `--no-build` and build yourself. If cargo cannot fetch the `buildkit` dependency, the tag the product pins is not published yet; see [link](commands.md#link).
- `unknown placeholder <name>`, `template path <path>`, `rendering template <path>`, `template <path> is not valid UTF-8`, `creating <path>`, `writing <path>`: a template of `cargo-bk` itself is broken, or the target cannot be written. Check the folder permissions. If they are right, report a bug with the version from `cargo bk --version`.
- `crate <name> already exists`: `crates/<name>` is there. Pick another name.
- `app <name> already exists`: `apps/<name>` is there. Pick another name.
- `no crate in crates/ for the app to build on; run cargo bk add-crate first`: `add-app` needs one library to depend on.
- `[workspace.metadata.buildkit] flow "<value>" is not direct or beta`: fix the `flow` key. See [Settings](configuration.md#keys).
- `<file> not found; add the row by hand: <row>` and `<file> has no <!-- bk:layout --> table; add the row by hand: <row>`: this is a note, not an error. `add-crate` or `add-app` could not append its row to the layout table of that file. Paste the row yourself.

## package {#package-errors}

- `Cargo.lock is missing; run cargo generate-lockfile and commit it (packages build with --locked)`: create and commit the lock file.
- `warning: no release tag found (shallow clone? use fetch-depth: 0); packaging as <version>`: git found no tag `vX.Y.Z` reachable from HEAD. Fetch the tags, or in CI check out with `fetch-depth: 0`. The installer is named with the `-dev` version.
- `cargo build of <app> failed: <output>`: the release build of an app failed. Fix the build error shown.
- `app <app> listed in [workspace.metadata.bk.package] apps has no binary`: the name in `apps` is not an app of the workspace, or it builds no binary. Fix the list.
- `extra-files glob <glob> leaves the workspace root` and `invalid extra-files glob <glob>`: fix the pattern; it must be relative and stay inside the product.
- `stage destination <path> leaves the stage`, `cannot clear <folder>`, `cannot create <folder>`, `cannot copy <a> to <b>`, `cannot set permissions on <file>`, `cannot write <file>`: a file operation failed in `build/`. Check the permissions, and close programs that hold files there.
- `cannot run cargo; install cargo-about with cargo install cargo-about`, `cargo about failed: <output>`: the notices step needs `cargo-about`. Install it, or run the bootstrap group `cargo-tools`. The output names the cause. A product that `new` made has `about.toml` and `about.hbs` at its root; restore them from a new product when they are gone.
- `inno installers are built on Windows`: you asked for `--format inno` on Linux.
- `appimage packages are built on Linux` and `deb packages are built on Linux`: you asked for that format on Windows.
- `unknown package format <name>; use inno, appimage, deb or all`: fix the `--format` value.
- `BK_ISCC points to <path>, which does not exist`: fix or unset the environment variable `BK_ISCC`.
- `ISCC.exe not found; install Inno Setup (bootstrap group packaging) or set BK_ISCC`: install Inno Setup, or set `BK_ISCC` to the full path of `ISCC.exe`.
- `ISCC failed: <output>`: the Inno Setup script has an error. The output names the line.
- `ISCC did not write <file>`: ISCC finished without writing the installer under the expected name. Run ISCC by hand on the script to see why.
- `dpkg-deb failed: <output>`: the Debian package could not be built. Install `dpkg-dev`, and read the output.
- `appimagetool failed: <output>`: the AppImage could not be built. Read the output.
- `download of <url> failed: <output>`: the download of `appimagetool` failed. Check the network, or install `appimagetool` yourself so that it is on the path.
- `SHA-256 of <url> does not match the pinned value; file deleted`: the downloaded `appimagetool` is not the pinned file. Do not use it. Try again, or install the pinned version with the bootstrap.
- `AppImages are built on Linux`: the AppImage builder ran on another system.

## manual {#manual-errors}

- `Python 3.11+ not found; run bootstrap (group python)`: install Python 3.11 or newer. The command tries `python3`, `python` and on Windows `py -3`.
- `manual check failed` and `manual build failed`: the renderer found problems. It prints each one before this line; they are listed next.

### Messages of the renderer

- `<lang>/<page>.md: front matter lacks title`, `lacks order` or `lacks summary`: add the key between the `---` lines at the top of the page.
- `<lang>/<page>.md: order <n> also used by <page>`: give every page of a language its own `order`.
- `missing page: <lang>/<page>.md`: a page exists in one language and not in another. Add it or remove the other.
- `<page>: external URL`: a page contains an external web address. The manual works offline; remove the address.
- `<page>: broken link <target>`, `missing anchor <target>` and `missing image <target>`: the link, the heading anchor or the image file does not exist.

## doctor {#doctor-errors}

Doctor prints one line per check: `ok`, `missing` or `wrong version`, then the name, and under each problem a line that starts with `fix:`. A check marked `(optional)` never fails the command.

- `doctor: required checks failed`: at least one required check is `missing` or `wrong version`. Run the fix that is printed under it.
- `wrong version rustc`: the installed Rust is not the channel in `rust-toolchain.toml`. Run the printed `rustup toolchain install` command.
- `wrong version cargo-bk`: the installed `cargo-bk` is not the version the product pins. Run the printed `cargo install` command.
- `missing husky`: the commit hooks are not installed. Run `npm install`.
- `missing <package>` for an apt package: a package from `system-deps.apt` is not installed. Run the printed `sudo apt-get install` command.
- `missing ISCC`, `missing MSVC`, `missing dpkg-deb`, `missing appimagetool`: install the packaging group of the bootstrap. MSVC is the Visual Studio C++ build tools, in the group `base`.

## link and unlink {#link-errors}

- `<path> is not a buildkit-rust checkout (crates/buildkit/Cargo.toml missing)`: the path is not a clone of buildkit-rust. The default is `../buildkit-rust` next to the product.
- `cannot resolve <path>`: the path does not exist.
- `<folder> is a root folder; put the product in a project folder first`: the parent of the product is a drive or filesystem root. Cargo would read a global config there.
- `<folder> is your home folder, so <file> is cargo's global config; use --force to write it anyway`: the product sits directly in your home folder, so the patch would apply to every project you have. Move the product into a project folder, or pass `--force`.
- `<file> already has [patch."<url>"]; remove or merge it by hand`: the config already patches buildkit-rust outside the block that `link` manages.
- `<file>:<line>: # >>> cargo bk link has no matching # <<< cargo bk link; fix or remove the block by hand`: the managed block lost its end marker. Edit the file.
- `refusing to write an invalid <file>`: the existing config is not valid TOML. Fix it first.
- `buildkit dependency is missing from [workspace.dependencies]` and `buildkit dependency is not an inline table; edit it by hand`: `--rewrite-dep` needs the dependency written as `buildkit = { git = "...", tag = "..." }`.
- `[workspace.metadata.buildkit] version is missing`: `unlink --rewrite-dep` restores the tag from this key.
- `Cargo.toml now uses a local buildkit path; do not commit this change (cargo bk unlink --rewrite-dep restores it)`: a warning after `link --rewrite-dep`. Restore the file before you commit.
- `patch written to <file>; it applies to every Cargo project under <folder>`: a note after `link`. It takes effect only when the pinned tag is published.

## check-pins and upgrade {#pins-errors}

- `<file>:<line>: v<found>, expected v<pin>`: that place pins another buildkit-rust version than `[workspace.metadata.buildkit] version`. The command lists one line per place and exits 1. Run `cargo bk upgrade` or fix the line.
- `[workspace.metadata.buildkit] version is missing`: the product has no pin to compare with.
- `warning: <file> differs from buildkit-rust v<version>`: a copied lint file differs from the copy in `cargo-bk`. It is a warning only. In GitHub Actions it appears as an annotation. Run `cargo bk upgrade` to refresh the files, or keep your change on purpose.
- `install cargo-bk v<version> first: cargo install --git <repository URL> --tag v<version> cargo-bk`: `upgrade` moves the pins to the version of the running `cargo-bk` only. Install that version, then run `upgrade` again. The URL is the address of the buildkit-rust repository.
- `cargo update -p buildkit failed: <output>`: `upgrade` moved the pins but could not refresh `Cargo.lock`, often because the tag is not on GitHub yet or the machine is offline. Fix the cause, then run `cargo update -p buildkit` yourself.

## repo-setup {#repo-errors}

- `unknown flow <value> in [workspace.metadata.buildkit]; use direct or beta`: fix the `flow` key.
- `gh <arguments> failed: <output>`: a read from GitHub failed. Run `gh auth login`, and run the command inside a clone of the repository.
- `cannot read the Beta branch: <output>`: GitHub answered with an error other than not found.
- `<command> failed: <output>`: one of the planned changes failed, for example the branch protection. You need admin rights on the repository. Branch protection on private repositories needs a GitHub plan that supports it. Run `--dry-run` to see every command.

## Bootstrap and setup scripts {#script-errors}

- `Unknown option: <option>`: the script does not know the option. `bootstrap.sh --help` lists them.
- `Unknown group: <group>`: use one of the names from `bootstrap.sh --list`.
- `--without needs a group`, `Unsupported group for --with: <group>`: only the group `editor` can be added.
- `Unsupported system: <system>. Use Linux (Debian/Ubuntu) or Windows (bootstrap.ps1).`: there is no macOS support.
- `Run as root or install sudo.`: the apt step needs `sudo`.
- `No apt-get found. Install these packages with your distribution's package manager:`: the script then prints the package list per group. Install them yourself.
- `appimagetool: SHA-256 mismatch, not installing`: the download is not the pinned file. Run the bootstrap again.
- `WARNING: cargo not found: skipping cargo-tools (install the rust group first)`: run the bootstrap again after the rust group.
- `WARNING: python3 <version> is older than 3.11`: install a newer Python. The manual and release tools need it.
- `WARNING: cargo bk doctor found problems; the fix is named next to each one`: the bootstrap finished, but doctor reported a required check. The exit code is still 0.
- `winget is missing. Install 'App Installer' from the Microsoft Store, then run this script again.`: Windows only. The bootstrap installs everything through winget.
- `winget could not install <id> (exit <code>)`: install that package by hand and run the bootstrap again.
- `No version in [workspace.metadata.buildkit] of Cargo.toml.`: the setup script of a product cannot read the pin. Check the table in the root `Cargo.toml`.
- `<program> <arguments> failed with exit code <n>`: a step of the Windows setup script failed. Run the step by hand to see the output.

## Reporting a problem

Send the output of `cargo bk --version`, the command you ran and the full error text.
