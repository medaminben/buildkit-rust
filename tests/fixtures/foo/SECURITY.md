# Security policy

## Supported versions

Only the latest release of Foo gets security fixes.

## Reporting a vulnerability

Please do not open a public issue for security problems. Report them privately through GitHub:
open the [Security tab](https://github.com/x/foo/security) and choose
**Report a vulnerability**
([direct link](https://github.com/x/foo/security/advisories/new)).

Include what is affected, how to reproduce it, and what an attacker could do with it. You should
get an answer within a week. Once a fix is released, the advisory is published and you are
credited unless you prefer not to be.

## Scope

In scope: the Foo apps and libraries in this repository, and the workflows in
`.github/workflows/`, which run with the repository's token.

Problems in a crate or tool Foo depends on belong with their maintainers. If Foo
pins a vulnerable version, do report that here.
