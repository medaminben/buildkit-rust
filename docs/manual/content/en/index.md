---
title: Welcome
order: 1
summary: What buildkit-rust is, how to install it and where to find each topic.
---

## buildkit-rust

buildkit-rust is a kit for Rust products. Its command `cargo bk` creates a product with the same layout, the same lint rules, the same installers and the same pipeline as every other product. It derives the product version from git tags and builds the installers.

It does not replace Cargo. You build, test and run with `cargo build`, `cargo test` and `cargo run`.

This manual is for people who use `cargo bk`. It has four pages:

- [Welcome](index.md), this page.
- [Commands](commands.md) lists every command and option.
- [Configuration](configuration.md) lists every setting, file and environment variable.
- [Troubleshooting](troubleshooting.md) lists every error and warning message and what to do about it.

## Installing the tools {#install}

On a new machine, clone buildkit-rust at a release tag and run the bootstrap script from the clone. Read the script first if you like: it is a plain file in the `bootstrap` folder.

1. On Linux (Debian or Ubuntu), run `bootstrap/bootstrap.sh`. On Windows, run `bootstrap/bootstrap.ps1` in PowerShell with `-ExecutionPolicy Bypass`.
2. Open a new terminal, so that `cargo` is on the path.
3. Run `cargo bk doctor --machine`. For everything that is missing it prints the group or command that installs it.

The bootstrap installs git, a C++ linker, Rust with `rustfmt` and `clippy`, `cargo-bk`, `cargo-deny`, `cargo-llvm-cov` and `cargo-about`, the libraries of the GUI toolkit, Node.js, Python, the GitHub command line, and the packaging tools of the system. It skips what is installed, so you can run it again. It runs `cargo bk doctor` at the end and still finishes with exit code 0 when doctor finds a problem.

## Starting a product {#start}

Create an empty folder outside the buildkit-rust clone, change into it and run `cargo bk new`. The command writes the workspace, a library, a console app, a GUI app, the installer files, the workflows and the documentation, then builds, tests and starts the console app. See [new](commands.md#new).

## Joining a product

When you clone a product that someone else started, run its setup script: `scripts/setup.sh` on Linux or `scripts/setup.ps1` on Windows. It installs `cargo bk` at the version the product pins, the system packages, the commit hooks and the dependencies, and then builds and tests. See [Setup scripts](configuration.md#setup).

## Building and releasing

- `cargo bk version` prints the version that the tags give.
- `cargo bk package` builds the installers into `build/dist`. See [package](commands.md#package).
- A pull request with a `semver:major`, `semver:minor`, `semver:patch` or `semver:none` label decides the next release tag when it is merged. See [Release labels](configuration.md#labels).
