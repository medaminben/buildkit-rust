# Security policy

## Supported versions

Only the latest `vX.Y.Z` tag on `main` gets security fixes. Products pin a tag (the
`[workspace.metadata.buildkit]` version in their `Cargo.toml`, `@vX.Y.Z` on the reusable
workflows), so a fix reaches them when they move their pin.

## Reporting a vulnerability

Please do not open a public issue for security problems. Report them privately through GitHub:
open the [Security tab](https://github.com/medaminben/buildkit-rust/security) and choose
**Report a vulnerability**
([direct link](https://github.com/medaminben/buildkit-rust/security/advisories/new)).

Include what is affected, how to reproduce it, and what an attacker could do with it. You should get
an answer within a week. Once a fix is released, the advisory is published and you are credited
unless you prefer not to be.

## Scope

In scope:
- the `cargo bk` tool and the bootstrap scripts, which run commands and install packages on the
  developer's machine;
- the reusable workflows in `.github/workflows/`, which run with the caller's token
  (`release.yml` with `contents: write`);
- the project templates that `cargo bk` generates.

Problems in a crate or tool a product depends on belong with their maintainers. If buildkit-rust
pins a vulnerable version, do report that here.
