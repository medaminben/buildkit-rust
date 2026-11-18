# Adding crates and apps

A product made by `cargo bk new` is a Cargo workspace with `members = ["crates/*", "apps/*"]`.
Libraries live in `crates/` and hold the logic. Apps live in `apps/` and only wire a library to
an interface: a command line or a window. An app depends on a library, never the other way
round, so each library can be tested without an interface.

Run both commands inside the product. Cargo picks up the new member without a change to the
root `Cargo.toml`.

## A library crate

```bash
cargo bk add-crate physics
```

For a product named `Pong` this writes:

| File | Content |
|---|---|
| `crates/physics/Cargo.toml` | Package `pong-physics`, `publish = false`, edition and `rust-version` from the workspace, `[lints] workspace = true` |
| `crates/physics/src/lib.rs` | A small public type, `Greeting`, with a doc test and a unit test |
| `crates/physics/tests/physics.rs` | An integration test that uses the crate through its public API |

The library name in code is `pong_physics`. Replace `Greeting` with your own API; it exists so
the crate builds and tests from the first minute.

Names use ASCII letters, digits, space, `-` or `_` and must start with a letter. The directory is
the kebab-case form (`my_math` and `My Math` both give `crates/my-math`), the package is
`<product>-<directory>`, and the code name is the snake-case form. `core`, `std`, `alloc` and
`test` are accepted because the package name carries the product prefix.

The command stops with `crate <dir> already exists` when `crates/<dir>` exists, and it never
overwrites a file.

## An app

```bash
cargo bk add-app tool --console
cargo bk add-app viewer --gui
```

Exactly one of `--console` and `--gui` is required. The app is `apps/<product>-<name>`, for
example `apps/pong-tool`. It depends on the product's `core` crate, or on the first crate in
`crates/` by name when there is no `core`. With no crate at all the command stops and tells you
to run `cargo bk add-crate` first.

| App | Files | Content |
|---|---|---|
| `--console` | `Cargo.toml`, `build.rs`, `src/main.rs` | Prints a greeting for its first argument; `--version` prints `<app> <full version>` |
| `--gui` | `Cargo.toml`, `build.rs`, `src/main.rs`, `src/app.rs` | An eframe window with the version in its title |

The generated code of both apps uses the `Greeting` type of that crate. When you have replaced
`Greeting` with your own API, edit the new app's `src/main.rs` (and `src/app.rs` for `--gui`)
before it builds.

The `build.rs` of each app calls `buildkit::emit_version`, so `env!("BK_VERSION_FULL")` works.

`add-app` does not change `apps` in `[workspace.metadata.bk.package]`. An app that should ship
in the installer has to be added to that list by hand. See [PACKAGING.md](PACKAGING.md).

## The layout tables

`README.md` and `docs/ARCHITECTURE.md` of a product each have a table that starts after the
line `<!-- bk:layout -->`. Both commands append one row to each table:

```text
| [`crates/physics`](crates/physics) | `pong-physics`: describe what this crate is for. |
```

Replace the sentence with your own. The commands only append a row, and only when no row starts
with the same first cell, so text you wrote is never rewritten. A file that has no marker table,
or that does not exist, is left alone and the command prints the row to add by hand.

## What a new crate needs from you

1. Write the real purpose in the row of both tables.
2. Add the crate as a dependency of the apps or crates that use it, with a path dependency
   such as `pong-physics = { path = "../../crates/physics" }`.
3. Run `cargo test` and `cargo clippy --all-targets -- -D warnings`.
4. Mention the crate in `docs/ARCHITECTURE.md` under dependencies when the dependency is not
   obvious from the table.
