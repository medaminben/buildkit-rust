---
title: Configuration
order: 3
summary: Every setting, file, label and environment variable that cargo bk reads.
---

## Settings in Cargo.toml {#keys}

`cargo bk` reads two tables from the `Cargo.toml` of the workspace root.

### `[workspace.metadata.buildkit]`

| Key | Meaning |
| --- | --- |
| `version` | The buildkit-rust version the product expects, as `X.Y.Z` without a `v`. It is the single source of the pin. The workflows install exactly this tag, and `cargo bk check-pins` compares every other mention with it. |
| `flow` | `direct` or `beta`. Default: `direct`. `repo-setup` and `add-crate` read it. |
| `repo` | The GitHub repository as `owner/repo`. Empty when `new` ran without `--repo`. The owner is the maintainer of the `.deb`. |

### `[workspace.metadata.bk.package]` {#package-keys}

| Key | Required | Meaning |
| --- | --- | --- |
| `display-name` | yes | The product name. Spaces become `-` in the installer file names. The lower-case form with `-` names the `.deb`, the `.desktop` file and the icon. |
| `main-app` | yes | The app that the AppImage starts. |
| `apps` | yes | The packages of the apps that the installers hold. |
| `iss` | yes | The Inno Setup script, relative to the root. |
| `desktop` | yes | The `.desktop` file. |
| `icon` | yes | The PNG icon. |
| `extra-files` | no | Globs, relative to the root, of files to add to the install tree. A glob cannot leave the root. |
| `system-deps.apt` | no | Apt packages. They become the `Depends` line of the `.deb`, and the setup script and doctor check them. |

### `[workspace.dependencies]`

The line `buildkit = { git = "...", tag = "vX.Y.Z" }` holds the build helper that the apps use from `build.rs`. Keep it an inline table, so that `link --rewrite-dep` can change it.

## Version in the program {#version}

The `build.rs` of every app calls `buildkit::emit_version` with a fallback. The program reads the result with `env!("BK_VERSION")`, which is `X.Y.Z`, and `env!("BK_VERSION_FULL")`. A tag `vX.Y.Z` or `vX.Y.Z-beta.N` on or before the commit gives the version. The `version` fields in `Cargo.toml` are not the source of the version.

## Release labels {#labels}

A merged pull request decides the next release tag by its label.

| Label | Effect |
| --- | --- |
| `semver:major` | Next major version |
| `semver:minor` | Next minor version |
| `semver:patch` | Next patch version. Also the default without a label. |
| `semver:none` | No release |
| `automerge` | Created by `repo-setup` for merge automation |

On `Beta` the tag is `vX.Y.Z-beta.N`; on `main` it is `vX.Y.Z`. A merge of `Beta` into `main` releases the base version of the open beta series. Use a merge commit for it, not a squash.

## Files that cargo bk manages

| File | Role |
| --- | --- |
| `.cargo/config.toml` | Sets `target-dir = "build/target"`, so build output stays under `build/`. |
| `rust-toolchain.toml` | The Rust version of the product, with `rustfmt` and `clippy`. |
| `rustfmt.toml`, `clippy.toml`, `deny.toml` | Lint files copied from buildkit-rust. Their first line names the version. `upgrade` refreshes them. |
| `about.toml`, `about.hbs` | The accepted licences and the layout of `THIRD_PARTY_NOTICES.txt`. |
| `packaging/NOTICES-extra.txt` | Notices that do not come from Rust crates. It is appended to `THIRD_PARTY_NOTICES.txt` when it is not empty. |
| `installer/<name>.iss` | The Inno Setup script. Its AppId GUID must never change. |
| `README.md`, `docs/ARCHITECTURE.md` | Each has a table after the line `<!-- bk:layout -->`. `add-crate` and `add-app` append rows to it and never change your text. |
| `build/` | All generated output: `build/target`, `build/stage`, `build/dist`, `build/manual`, `build/tools`. It is ignored by git. |

## Manual pages {#manual-pages}

The manual lives in `docs/manual`. The pages are `docs/manual/content/<lang>/<page>.md`, with one folder per language, and images and other files are in `docs/manual/assets`. Every page starts with front matter between two `---` lines:

| Key | Meaning |
| --- | --- |
| `title` | The page title. |
| `order` | A number. It sets the order of the pages, and no two pages of a language share one. |
| `summary` | One sentence about the page. |

The body uses `##` and `###` headings with an optional `{#anchor}`, paragraphs, lists that start with `-` or `1.`, tables, the blocks `::: note`, `::: warning`, `::: formula` and `::: step` closed by `:::`, images written as `![caption](assets/file.png)`, bold, italic and links to other pages such as `[text](page.md#anchor)`. A word between backticks is rendered as a label of the application. A page may not contain an external web address, because the manual has to work offline. Every language needs the same set of pages.

## Setup scripts {#setup}

`scripts/setup.sh` and `scripts/setup.ps1` of a product run these steps.

1. When `cargo bk` is missing or is not the pinned version, clone the buildkit-rust bootstrap at the pinned tag into `build/tools/buildkit-rust` and run it.
2. On Linux, install the packages of `system-deps.apt` that are missing. The script asks first, unless you pass `--yes`.
3. Run `npm ci` (or `npm install` without a lock file) and `cargo fetch`.
4. Run `cargo bk doctor`, `cargo build` and `cargo test`.

| Option | Script | Meaning |
| --- | --- | --- |
| `--yes` or `-y` | `setup.sh` | No questions |
| `--dry-run` | `setup.sh` | Print the steps and run none of them |
| `-Yes` | `setup.ps1` | Run the bootstrap without questions |
| `-DryRun` | `setup.ps1` | Print the steps and run none of them |

## Bootstrap options {#bootstrap}

`bootstrap.sh` and `bootstrap.ps1` install the tools. The groups are `base`, `rust`, `cargo-tools`, `gui`, `node`, `python`, `github`, `packaging` and the optional `editor`.

| `bootstrap.sh` | `bootstrap.ps1` | Meaning |
| --- | --- | --- |
| `--list` | `-List` | Show the groups and what they install |
| `--dry-run` | `-DryRun` | Print what would be installed |
| `--yes` | `-Yes` | No questions |
| `--minimal` | `-Minimal` | Only `base`, `rust` and `cargo-tools` |
| `--with editor` | `-WithEditor` | Add the optional group |
| `--without <group>` | `-Without <group>` | Leave out a group; it can be repeated |

## Environment variables

| Variable | Used by | Meaning |
| --- | --- | --- |
| `BK_ISCC` | `package`, `doctor` | Full path of `ISCC.exe`. It is looked up before the path and the default folders, and it must exist. |
| `GITHUB_ACTIONS` | `check-pins` | When it is `true`, a lint file warning is written as a GitHub annotation. |
| `HOME` or `USERPROFILE` | `link` | The home folder that `link` refuses without `--force`. |
| `ProgramFiles(x86)` and `LOCALAPPDATA` | `package`, `doctor` | Where `ISCC.exe` and `vswhere.exe` are looked for on Windows. |
| `APPIMAGE_EXTRACT_AND_RUN` and `ARCH` | `package` | Set by `cargo bk package` for `appimagetool`, so that it runs without FUSE. |
| `BK_VERSION` and `BK_VERSION_FULL` | build | Set by `buildkit::emit_version` for the compiler. |
