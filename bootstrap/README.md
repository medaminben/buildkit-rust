# Bootstrap

`bootstrap.sh` is the first thing to run on a new Linux machine. It installs what you need to work on Rust products. Run it from a clone, so you can read it before it runs:

```bash
git clone --branch vX.Y.Z https://github.com/medaminben/buildkit-rust.git ~/dev/buildkit-rust
~/dev/buildkit-rust/bootstrap/bootstrap.sh
```

Debian and Ubuntu (apt) are supported. On other distributions the script prints the package list to install by hand. Started from Git Bash or MSYS2 on Windows, it hands over to `bootstrap.ps1`.

It is safe to run again: installed packages and tools are skipped.

## Groups

| Group | Installs |
|---|---|
| base | git, curl, ca-certificates, build-essential, pkgconf |
| rust | rustup, stable with `rustfmt`, `clippy`, `llvm-tools-preview`, `cargo-binstall` |
| cargo-tools | `cargo-bk` (from this clone), `cargo-deny`, `cargo-llvm-cov`, `cargo-about` |
| gui | the libraries eframe and wgpu need to build and run |
| node | Node.js and npm (commitlint, husky) |
| python | python3 3.11 or newer, venv, pip |
| github | gh |
| packaging | `appimagetool` (pinned, SHA-256 checked, into `~/.local/bin`), dpkg-dev |
| editor (optional) | the VS Code extensions rust-analyzer, CodeLLDB and Even Better TOML |

`cargo-bk` is installed from the clone the script lives in, so the bootstrap and `cargo bk` always come from the same tag. cargo-binstall is tried first for the other cargo tools, with a `cargo install` fallback.

Only the apt packages need `sudo`. Rust, the cargo tools and `appimagetool` go into your home directory.

## Options

```text
--list            show the groups and what they install
--dry-run         print what would be installed, change nothing
--yes             no questions
--minimal         base, rust and cargo-tools only
--with editor     add the optional editor group
--without <group> leave out a group (repeatable)
```

## Tests

```bash
bash bootstrap/tests/test_bootstrap.sh
```

The tests run the script against stubs in `bootstrap/tests/stubs/`. Each stub appends its arguments to `$CALLS` and acts as installed or missing according to `$STUB_MISSING`.
