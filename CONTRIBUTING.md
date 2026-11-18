# Contributing to buildkit-rust

## Setup and tests

```bash
cargo test --workspace
```

Write the test first, see it fail, then implement. Build output goes only under `build/`
(`target-dir = "build/target"` in `.cargo/config.toml`).

Git hooks (format and commit lint) need Node: `npm install`.

## Style

- Rust stable (pinned in `rust-toolchain.toml`), edition 2024.
- Format with `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- Documentation is written in English and uses only names that exist in the code.

## Commits

Conventional Commits: `<type>(optional-scope): <description>` with the types
`feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`,
`chore`, `revert`.

## Pull requests

- [ ] `cargo test --workspace` passes
- [ ] New behavior has a test
- [ ] Docs updated when behavior changes
- [ ] The buildkit-rust version pins still agree

The default branch is `main` and the flow is direct: a feature branch goes to `main` by pull
request. A release is a tag `vX.Y.Z`, created by hand on `main` after the merge.
Push the tag to GitHub before any product pins that version: `cargo bk new`, the setup scripts
and the products' workflows fetch buildkit-rust and cargo-bk at that tag.

Do not commit secrets or machine-local paths.
