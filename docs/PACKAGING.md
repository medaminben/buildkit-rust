# Packaging

`cargo bk package` builds one installer per product, not per app. The installer holds every app
listed in the product's packaging metadata, and the main app gets the Start menu, desktop and
`.desktop` entries. It runs the same way on a developer machine and in CI.

```bash
cargo bk package                       # every format of this system
cargo bk package --format deb          # inno, appimage, deb or all
```

| Format | Built on | Result in `build/dist/` |
|---|---|---|
| `inno` | Windows | `<Name>_Setup_<full>.exe` and a copy `<Name>_Setup.exe` |
| `appimage` | Linux | `<Name>-<full>-x86_64.AppImage` |
| `deb` | Linux | `<kebab>_<debver>_amd64.deb` |

`all` means every format of the current system, and so does leaving `--format` out. Asking for a
format that the system cannot build is an error. There is no macOS packaging and no
cross-building: Windows installers are built on Windows.

## Configuration

The settings live in the root `Cargo.toml`:

```toml
[workspace.metadata.bk.package]
display-name = "Bowlingbox"
main-app = "bowlingbox-gui"
apps = ["bowlingbox-gui", "bowlingbox-console"]
iss = "installer/bowlingbox.iss"
desktop = "packaging/bowlingbox.desktop"
icon = "packaging/icon.png"
extra-files = ["assets/**"]
system-deps.apt = ["libxkbcommon0", "libvulkan1"]
```

| Key | Required | Meaning |
|---|---|---|
| `display-name` | yes | The product name as people read it. Spaces become `-` in file names. Its kebab-case form (`My App` gives `my-app`) names the `.deb` package, the `.desktop` file and the icon |
| `main-app` | yes | The app the AppImage starts (the `AppRun` link). The generated `.iss` (`MyAppExe`) and `.desktop` file (`Exec`) name the same app: change all three together |
| `apps` | yes | The packages of the apps to build and ship |
| `iss` | yes | The Inno Setup script |
| `desktop` | yes | The `.desktop` file |
| `icon` | yes | The icon, a PNG, installed at 256x256 in the `.deb` |
| `extra-files` | no | Globs, relative to the workspace root, of files to add to the install tree |
| `system-deps.apt` | no | Apt packages: the `Depends` line of the `.deb`, and what the setup script and `cargo bk doctor` check on Linux |

`cargo bk new` fills this with the GUI app as `main-app` and both apps in `apps`. `add-app` does
not touch it.

## Steps

1. `cargo build --release --locked -p <app>` for each app in `apps`. A product needs a committed
   `Cargo.lock`, because the build uses `--locked`.
2. The install tree is staged into `build/stage/`, which is cleared first: each app's binary
   from `build/target/release`, `LICENSE` when it exists, and the files that match `extra-files`
   (globs cannot leave the workspace root; `build`, `target` and `.git` at the top level,
   `node_modules` at any depth and symlinked files are skipped).
3. `THIRD_PARTY_NOTICES.txt` is written into the stage.
4. Each requested format is built into `build/dist/`.

When the version has no release tag behind it, the command warns that it packages as
`<fallback>-dev`. A shallow clone is the usual cause: use `fetch-depth: 0` in CI, as the
generated workflows do.

### Version in the names

`<full>` is the version from `buildkit::resolve`: `X.Y.Z`, `X.Y.Z-beta.N`, or either one with
`+<ahead>.g<sha7>` after it. For the `.deb`, the first `-` becomes `~`, so `1.2.3-beta.4` becomes
`1.2.3~beta.4` and sorts before `1.2.3`.

### Third-party notices

`cargo about generate --config about.toml about.hbs` renders the licence of every crate in
`Cargo.lock`. `about.toml` lists the accepted licences, the same set as `deny.toml`, and
`about.hbs` is the output template. The text in `packaging/NOTICES-extra.txt`, for fonts, sounds
or vendored files that are not Rust crates, is appended when that file is not empty. Install the
tool with `cargo install cargo-about`; the bootstrap does it in the group `cargo-tools`.

## Inno Setup (Windows)

`ISCC` runs with these arguments, each one argument of its own so paths with spaces need no
quoting:

| Argument | Value |
|---|---|
| `/DAppVersion` | The full version |
| `/DAppVersionNumeric` | `X.Y.Z`, for `VersionInfoVersion` |
| `/DStageDir` | `build/stage` |
| `/O` | `build/dist` |
| `/F` | `<Name>_Setup_<full>` |

The generated `installer/<kebab>.iss` has a fixed AppId GUID that `cargo bk new` creates once,
with a comment not to change it: upgrades find the existing install by it. It starts
`<kebab>-gui.exe`, so edit the script when `main-app` is another app.

`ISCC.exe` is found through the environment variable `BK_ISCC` first (a path that must exist),
then `PATH`, then `%ProgramFiles(x86)%\Inno Setup 6` and `%LOCALAPPDATA%\Programs\Inno Setup 6`.
Without it the command fails with a message that names `BK_ISCC`. The copy `<Name>_Setup.exe`
keeps the `releases/latest/download/<Name>_Setup.exe` link stable.

## AppImage (Linux)

The command assembles `build/appdir/` with the whole stage in `usr/bin` (the apps, `LICENSE`,
`THIRD_PARTY_NOTICES.txt` and the `extra-files`), the `.desktop` file and the icon, and an
`AppRun` link to `main-app`. It then runs `appimagetool` with
`APPIMAGE_EXTRACT_AND_RUN=1` and `ARCH=x86_64`, so FUSE is not needed.

`appimagetool` comes from `PATH`. Otherwise the command downloads version 1.9.1 to
`build/tools/`, checks its SHA-256 and deletes the file when the hash differs. The URL and hash
are pinned in `crates/cargo-bk/src/package/appimage.rs` and in `bootstrap/bootstrap.sh`, which
installs the same file into `~/.local/bin`. Change them together.

## Debian package (Linux)

The command writes the package tree to `build/deb-root/` and runs
`dpkg-deb --root-owner-group --build`. It does not use `cargo-deb`.

- `/usr/lib/<kebab>/`: the whole stage, so the files sit next to the apps as in the Windows
  install.
- `/usr/bin/<app>` for each app in `apps`: a relative link to `../lib/<kebab>/<app>`. A program
  that finds its files through `std::env::current_exe` gets the path in `/usr/lib/<kebab>/`,
  because the link is resolved.
- `/usr/share/applications/<kebab>.desktop` and
  `/usr/share/icons/hicolor/256x256/apps/<kebab>.png`.
- `/usr/share/doc/<kebab>/copyright`: `LICENSE` followed by `THIRD_PARTY_NOTICES.txt`.
- `DEBIAN/control` with `Package` (the kebab name), `Version`, `Architecture: amd64`,
  `Maintainer`, `Installed-Size`, `Depends` (from `system-deps.apt`, left out when empty) and
  `Description` (the display name). The maintainer is the owner part of
  `[workspace.metadata.buildkit] repo`, or the kebab name when `repo` is empty.

All three formats ship the whole stage, with the same layout relative to the executables: the
Windows install folder, `usr/bin/` in the AppImage and `/usr/lib/<kebab>/` in the `.deb`.

## In CI

The `package` input of the reusable `ci.yml` runs `cargo bk package` on both systems on every
run. The generated `installer.yml` builds the installers for a release tag and attaches them. See
[CI_CD.md](CI_CD.md).
