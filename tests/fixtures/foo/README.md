# Foo

Foo is a Rust workspace made with buildkit-rust. Describe what Foo does in one or two sentences here.

## Build and run

```
cargo build
cargo test
cargo run -p foo-console
cargo run -p foo-gui
```

`cargo bk version` prints the product version. `cargo bk` lists the other commands.

## Layout

<!-- bk:layout -->
| Path | Purpose |
| --- | --- |
| [`crates/core`](crates/core) | `foo-core`: describe what this crate is for. |
| [`crates/physics`](crates/physics) | `foo-physics`: describe what this crate is for. |
| [`apps/foo-gui`](apps/foo-gui) | `foo-gui`: the desktop app. |
| [`apps/foo-console`](apps/foo-console) | `foo-console`: the command-line app. |

`cargo bk add-crate <name>` and `cargo bk add-app <name> --console|--gui` add a row to this table and to the one in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Your own text in either file is never rewritten.

## Documentation

- [Architecture](docs/ARCHITECTURE.md): how the crates and apps fit together.
- [User manual](docs/manual/content/en/index.md): the pages that ship with the app.
