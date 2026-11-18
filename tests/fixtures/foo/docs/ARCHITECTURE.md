# Foo architecture

Describe in a few sentences how Foo is put together and why.

## Layout

<!-- bk:layout -->
| Path | Purpose |
| --- | --- |
| [`crates/core`](crates/core) | `foo-core`: describe what this crate is for. |
| [`crates/physics`](crates/physics) | `foo-physics`: describe what this crate is for. |
| [`apps/foo-gui`](apps/foo-gui) | `foo-gui`: the desktop app. |
| [`apps/foo-console`](apps/foo-console) | `foo-console`: the command-line app. |

Libraries live in `crates/` and hold the logic. Apps live in `apps/` and only wire a library to an interface: a command line or a window. An app depends on a library, never the other way round, so each library can be tested without an interface.

## Dependencies

Write down which crate depends on which, and why, when a dependency is not obvious from the table.

## Decisions

Record a decision here when you make one: what you chose, what you rejected, and what would make you revisit it.
