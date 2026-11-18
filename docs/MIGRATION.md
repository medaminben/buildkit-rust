# Migrating from buildkit

This guide is for a product that uses the C++ and CMake [buildkit](https://github.com/medaminben/buildkit)
and moves to Rust. Products built on it follow the same path. buildkit-rust does not convert C++ code or
CMake files: the product is rewritten in Rust, and the build, version, packaging and CI
conventions carry over. The table below lists every buildkit feature and what replaces it, or
why it was dropped on purpose.

## The steps

1. Install the tools on the machine: clone buildkit-rust at a release tag and run the
   bootstrap (see the [README](../README.md#quick-start)).
2. Generate the product with `cargo bk new <Name> <Crate...> --repo owner/repo`, with the same
   arguments `scripts/new-project.sh` took (`Name`, `Lib...`, `--repo`). Add `--flow beta` when
   the C++ product used the `Beta` branch flow.
3. Move the code, one library at a time, into `crates/<name>` and the executables into `apps/`.
   Use `cargo bk add-crate` and `cargo bk add-app` for more.
4. Fill `[workspace.metadata.bk.package]`: the display name, the main app, `extra-files` and
   `system-deps.apt`. Move the Inno Setup script to `installer/<kebab>.iss` with its AppId GUID
   unchanged, so upgrades still find the old installs. See [PACKAGING.md](PACKAGING.md).
5. Move the manual pages to `docs/manual/content/<lang>/` and run `cargo bk manual --check`.
   The page format is the same, because `tools/manual` is copied from buildkit.
6. Run `cargo bk repo-setup --dry-run`, then `cargo bk repo-setup`, to create the labels and the
   branch protection.
7. Release as before: the `semver:*` label of a merged pull request decides the next tag, and
   tags are `vX.Y.Z` and `vX.Y.Z-beta.N`.

## Feature by feature

### CMake modules

| buildkit | buildkit-rust |
|---|---|
| `Buildkit.cmake` (module entry, `BUILDKIT_VERSION`) | **Dropped**: there are no modules to load. `cargo bk` and the `buildkit` crate are versioned by the tag the product pins |
| `GitVersion` (`buildkit_git_version`) | The `buildkit` crate: `buildkit::emit_version("x.y.z")` in `build.rs` sets `BK_VERSION` and `BK_VERSION_FULL`, with the same rules. `cargo bk version` prints it |
| `Dependency` (`dev_dependency`: installed, fetched or without) | **Dropped**: Cargo resolves and fetches dependencies. Git dependencies are pinned to tags, `Cargo.lock` is committed, and `cargo build --offline` replaces the offline switches |
| `BuildLocation` (output folders in the binary dir) | `.cargo/config.toml` with `target-dir = "build/target"`, so all output stays under `build/` |
| `BuildUtils` (`create_library`, `create_application`) | **Dropped**: a crate is a folder with a `Cargo.toml`. `cargo bk add-crate` and `cargo bk add-app` create them |
| `LibraryGenerator` (`generate_libraries`, `generate_project`) | `cargo bk new`, `cargo bk add-crate`, `cargo bk add-app`. See [ADDING_CRATES.md](ADDING_CRATES.md) |
| `CompilerWarnings` (`buildkit_set_warnings`) | `[workspace.lints]` in the root `Cargo.toml`: `unsafe_code = "deny"` and `clippy::pedantic` as warnings. CI runs clippy with `-D warnings` |
| `Coverage` (`ENABLE_COVERAGE`) | The `coverage` input of `ci.yml`, which runs `cargo llvm-cov` |
| `TestUtils`, `GTestSupport` (`build_gtest_executable`, GoogleTest) | **Dropped**: `cargo test` has a test harness built in |
| `Packaging` (`buildkit_add_stage_target`, `buildkit_add_inno_installer`) | `cargo bk package`: Inno Setup, AppImage and `.deb`. The wine path on Linux is **dropped**, because Windows binaries are built on Windows. See [PACKAGING.md](PACKAGING.md) |
| `Manual` (`buildkit_add_manual`) | `cargo bk manual` and `cargo bk manual --check`, with the same renderer, embedded in `cargo-bk` |
| `DebugUtils` (`dump_cmake_vars`, `print`, `prompt`) | **Dropped**: they dump CMake variables, which do not exist in a Cargo workspace |
| `InstallConfig` and `PackageConfig.cmake.in` (`find_package` config) | **Dropped**: Rust consumers depend on a crate by git tag. There is no crates.io publishing for now |

### Loader

| buildkit | buildkit-rust |
|---|---|
| `loader/buildkit.cmake`, `BUILDKIT_TAG` | The pin: `[workspace.metadata.buildkit] version` and the `buildkit` git dependency. `cargo bk check-pins` and `cargo bk upgrade` keep every copy equal |
| `BUILDKIT_DIR` and the sibling checkout `../buildkit` | `cargo bk link [path]`, default `../buildkit-rust`, without a committed change. `cargo bk link --rewrite-dep` for an unpublished version |
| `BUILDKIT_REPOSITORY` | **Dropped** for products. The `buildkit-repository` input of the workflows remains |
| `BUILDKIT_OFFLINE`, `FETCHCONTENT_FULLY_DISCONNECTED` | **Dropped**: Cargo has `--offline` and `cargo fetch --locked` |

### Workflows and inputs

| buildkit | buildkit-rust |
|---|---|
| `ci.yml` job `lint` | `ci.yml` job `lint`: `cargo fmt`, clippy, `cargo deny`, `cargo bk manual --check`, `cargo bk check-pins`, commitlint |
| `ci.yml` job `build` | `ci.yml` job `test`: `cargo build --locked` and `cargo test --locked` on `ubuntu-latest` and `windows-latest` |
| `ci.yml` job `analysis` (clang-tidy, cppcheck) | Clippy, in the `lint` job. It fails the job instead of only reporting |
| `ci.yml` input `presets` | **Dropped**: there are no presets. `release` (default `true`) also runs `cargo test --release` |
| `ci.yml` input `compilers` | **Dropped**: rustc is the compiler. The system matrix is fixed |
| `ci.yml` input `apt-packages` | Same name. Empty uses the product's `system-deps.apt` |
| `ci.yml` inputs `lint`, `lint-format`, `lint-commits` | Same names and precedence |
| `ci.yml` input `lint-cmake` | **Dropped**: no CMake. `lint-clippy` and `lint-deny` are new |
| `ci.yml` input `format-exclude` | Same name, for `cargo fmt` |
| `ci.yml` input `xvfb` | Same name |
| `ci.yml` input `timeout-minutes` | Same name |
| `ci.yml` input `configure-args` | **Dropped**: there is no configure step |
| `ci.yml` input `static-analysis` | Replaced by `lint-clippy` |
| `ci.yml` input `coverage` | Same name; `cargo llvm-cov` |
| `ci.yml` input `sanitizers` (ASan, UBSan) | Replaced by `miri-crates`, which runs `cargo +nightly miri test` on the crates you list |
| `ci.yml` inputs `buildkit-repository`, `buildkit-ref` | Same names. With `buildkit-ref` set, `cargo bk link --rewrite-dep` replaces exporting `BUILDKIT_DIR` |
| (none) | New inputs: `release`, `package`, `working-directory`, `lint-clippy`, `lint-deny`, `miri-crates` |
| `codeql.yml`, inputs `preset`, `apt-packages`, `configure-args`, `buildkit-*` | `codeql.yml` without inputs. CodeQL analyses Rust without a build, so none of them applies |
| `release.yml`, input `installer-workflow` | Same behaviour and name |
| `release.yml`, inputs `buildkit-repository`, `buildkit-ref` | Same names. Empty takes `tools/release` from the tag `v<version>` of the caller's pin, where buildkit had a fixed default |
| `new-project.yml` | The `new-project` job of `self-test.yml` |
| `self-test.yml` | `self-test.yml`: scripts, crates, bootstrap, new project and the fixture product's CI |
| Product `windows-installer.yml` (user-written) | Generated `installer.yml`, which runs `cargo bk package` on Windows and Linux |

### Tools

| buildkit | buildkit-rust |
|---|---|
| `tools/release/next_version.py`, `plan_release.py` and their tests | Copied unchanged into `tools/release/`, used by `release.yml` |
| `tools/manual/build_manual.py`, `template.html`, `manual.css`, `manual.js` and tests | Copied into `tools/manual/` and embedded in `cargo-bk` |
| `tools/manual/capture.sh` | **Planned (deferred, required).** Not in the first release. See [Deferred](#deferred-planned-required) |
| `tests/release_pins/check_pins.py` | `cargo bk check-pins`, in the `lint` job. It also checks the lint file copies, and buildkit-rust runs it on its own docs |

### Lint

| buildkit | buildkit-rust |
|---|---|
| `.clang-format` | `rustfmt.toml`, checked with `cargo fmt --check` |
| clang-tidy and cppcheck | Clippy with `[workspace.lints]`, thresholds in `clippy.toml` |
| `scripts/check-cmake-style.mjs` | **Dropped**: no CMake |
| `.editorconfig` | Kept |
| `lint/commitlint.config.cjs`, husky, lint-staged | Same configuration. lint-staged runs `rustfmt` on staged `.rs` files |
| (none) | New: `deny.toml` for `cargo deny` (licences, advisories, duplicate versions, git sources limited to `medaminben`) |

The product's copies of `rustfmt.toml`, `clippy.toml` and `deny.toml` carry the line
`# buildkit-rust v<version>` first. `cargo bk upgrade` refreshes them, and a difference from the
pinned tag is a warning in CI.

### Dependencies and presets

| buildkit | buildkit-rust |
|---|---|
| `conan/profiles` (`linux-gcc`, `windows-mingw`) | **Dropped**: Cargo replaces Conan, and there is no cross-compilation to Windows |
| `CMakePresets.json`, `CMakeUserPresets.json` | **Dropped**. `.cargo/config.toml`, the toolchain file `rust-toolchain.toml` and `.vscode/` (Ctrl+Shift+B builds, F5 runs the console app) take their place |

### Scripts and docs

| buildkit | buildkit-rust |
|---|---|
| `scripts/new-project.sh`, `new-project.ps1` | `cargo bk new`, with the same arguments, after the bootstrap. The machine setup that the C++ scripts left to the reader is `bootstrap/bootstrap.sh` and `bootstrap.ps1` |
| `template/app`, `template/lib`, `template/project` | `templates/`, embedded in `cargo-bk` |
| `tests/` (one fixture project per module) | The tests of both crates, the golden fixture `tests/fixtures/foo` and the self-test |
| `docs/ADDING_LIBRARIES.md` | [ADDING_CRATES.md](ADDING_CRATES.md) |
| `docs/CI_CD.md` | [CI_CD.md](CI_CD.md) |
| `docs/MIGRATION.md` (unitpro to buildkit) | This file. The unitpro layout does not exist in buildkit-rust |
| `docs/CONTRIBUTING.md` | `CONTRIBUTING.md` of this repository, and a generated one for every product with `--repo` |

## Deferred (planned, required)

Manual screenshot capture, buildkit's `tools/manual/capture.sh`, is deferred, not dropped. It is
required and will ship before the first public release of the first product with an egui GUI,
so that product's manual has generated screenshots.

- `cargo bk manual capture` runs the product's capture command headless and writes
  `docs/manual/assets/shots/<lang>/<name>.png`, which `![caption](shot:name)` in a page refers to.
- The GUI template gets a capture hook that renders the window offscreen.
- CI runs the capture on Linux and Windows, and the self-test covers it.

Until it ships, put screenshots into `docs/manual/assets/` by hand.

Tracking issue: to be linked.
