#!/usr/bin/env bash
# Tests for bootstrap.sh. The script runs with stubs first on PATH; each stub appends its
# argv to $CALLS and acts "installed" or "missing" according to $STUB_MISSING.
#
# Usage: bash bootstrap/tests/test_bootstrap.sh

set -uo pipefail

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bootstrap="$tests_dir/../bootstrap.sh"
stubs="$tests_dir/stubs"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

pass=0
fail=0
ok() { pass=$((pass + 1)); printf '  ok   %s\n' "$1"; }
bad() { fail=$((fail + 1)); printf '  FAIL %s\n' "$1"; }
assert_rc() { # <expected: 0|nonzero> <message>
    if [[ "$1" == 0 && $rc -eq 0 ]] || [[ "$1" != 0 && $rc -ne 0 ]]; then ok "$2"; else bad "$2 (rc=$rc)"; fi
}
assert_has() { if grep -Eq -- "$2" "$1"; then ok "$3"; else bad "$3 (no match for: $2)"; fi; }
assert_lacks() { if grep -Eq -- "$2" "$1"; then bad "$3 (unexpected: $(grep -E -- "$2" "$1" | head -1))"; else ok "$3"; fi; }

# run_case <name> <STUB_MISSING> <script> [args...]: runs into $work/<name>.{calls,out}, sets $rc.
run_case() {
    local name="$1" missing="$2" script="$3"
    shift 3
    CALLS="$work/$name.calls"
    out="$work/$name.out"
    : >"$CALLS"
    mkdir -p "$work/$name.home/.local/bin"
    # A previously installed appimagetool, unless the case says it is missing.
    if [[ ",$missing," != *,appimagetool,* && ",$missing," != *,all,* ]]; then
        printf '#!/bin/sh\n' >"$work/$name.home/.local/bin/appimagetool"
        chmod +x "$work/$name.home/.local/bin/appimagetool"
    fi
    HOME="$work/$name.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_MISSING="$missing" \
        bash "$script" "$@" >"$out" 2>&1
    rc=$?
    calls="$CALLS"
}

echo "bootstrap.sh"

if [[ ! -f "$bootstrap" ]]; then
    bad "bootstrap.sh exists at $bootstrap"
    printf '\n%d passed, %d failed\n' "$pass" "$fail"
    exit 1
fi

# --list names every group.
run_case list "" "$bootstrap" --list
assert_rc 0 "--list exits 0"
for g in base rust cargo-tools gui node python github packaging editor; do
    assert_has "$out" "^$g( |$)" "--list shows $g"
done

# Dry run with everything missing: no installs, [dry-run] lines instead.
run_case dry all "$bootstrap" --dry-run --yes --with editor
assert_rc 0 "--dry-run exits 0"
assert_lacks "$calls" '^apt-get install' "dry-run: no apt-get install"
assert_lacks "$calls" '^rustup-init' "dry-run: no rustup-init"
assert_lacks "$calls" '^cargo (install|binstall --no-confirm)' "dry-run: no cargo install"
assert_lacks "$calls" '^curl .*-o ' "dry-run: no curl -o"
assert_lacks "$calls" '^code --install-extension' "dry-run: no editor extension install"
assert_has "$out" '^\[dry-run\] .*apt-get install' "dry-run prints the apt-get install"
assert_has "$out" '^\[dry-run\] .*curl .*-o .*rustup-init' "dry-run prints the rustup-init download"
assert_has "$out" '^\[dry-run\] .*rustup-init -y --default-toolchain stable' "dry-run prints the rustup-init call"
assert_has "$out" '^\[dry-run\] .*cargo install' "dry-run prints a cargo install"
assert_has "$out" '^\[dry-run\] .*curl .*-o .*appimagetool' "dry-run prints the appimagetool download"
assert_has "$out" '^\[dry-run\] .*code --install-extension' "dry-run prints the editor extension install"

# Nothing missing: nothing is installed.
run_case idem "" "$bootstrap" --yes --with editor
assert_rc 0 "idempotent run exits 0"
assert_lacks "$calls" '^apt-get (install|update)' "idempotent: no apt-get install"
assert_lacks "$calls" '^rustup-init' "idempotent: no rustup-init"
assert_lacks "$calls" '^rustup (component add|toolchain install|default)' "idempotent: no rustup changes"
assert_lacks "$calls" '^cargo (install|binstall --no-confirm)' "idempotent: no cargo install"
assert_lacks "$calls" '^curl .*-o ' "idempotent: no downloads"
assert_lacks "$calls" '^code --install-extension' "idempotent: no editor install"
assert_has "$out" 'cargo-bk: skipped \(not in this checkout\)' "cargo-bk skipped when crates/cargo-bk is absent"

# rustup missing: download rustup-init and call it with the exact arguments.
run_case rustup rustup "$bootstrap" --yes
assert_rc 0 "rustup install exits 0"
assert_has "$calls" '^curl .*-o .*rustup-init' "rustup missing: downloads rustup-init"
assert_has "$calls" '^rustup-init -y --default-toolchain stable --profile minimal -c rustfmt -c clippy -c llvm-tools-preview$' \
    "rustup missing: calls rustup-init with the pinned arguments"

# Missing components on an existing rustup are added.
run_case comp llvm-tools "$bootstrap" --yes
assert_rc 0 "run with a missing component exits 0"
assert_has "$calls" '^rustup component add llvm-tools-preview$' "missing component: only that component is added"
assert_lacks "$calls" '^rustup component add .*rustfmt' "missing component: installed ones are left alone"

# A missing package triggers update and install.
run_case apt git "$bootstrap" --yes
assert_has "$calls" '^apt-get install -y .*git' "missing package: apt-get install runs"

# --without drops a group.
run_case without apt "$bootstrap" --yes --without gui --without base --without node --without python \
    --without github --without packaging --without rust --without cargo-tools
assert_lacks "$calls" '^apt-get install' "--without removes every apt group"

# Unknown group is an error.
run_case badgroup "" "$bootstrap" --without nonsense
assert_rc nonzero "unknown group exits non-zero"

# Git Bash hands over to bootstrap.ps1.
CALLS="$work/win.calls"
: >"$CALLS"
HOME="$work/win.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_UNAME="MINGW64_NT-10.0" \
    bash "$bootstrap" --yes >"$work/win.out" 2>&1
assert_has "$CALLS" '^powershell.exe .*bootstrap\.ps1' "MINGW64_NT execs powershell.exe ... bootstrap.ps1"

# A path with a space works, and cargo-bk installs from that path.
spaced="$work/dir with space"
mkdir -p "$spaced/bootstrap" "$spaced/crates/cargo-bk"
cp "$bootstrap" "$spaced/bootstrap/bootstrap.sh"
printf '[package]\nname = "cargo-bk"\nversion = "0.1.0"\n' >"$spaced/crates/cargo-bk/Cargo.toml"
run_case spaced cargo-bk "$spaced/bootstrap/bootstrap.sh" --yes
assert_rc 0 "path with a space: exits 0"
assert_has "$calls" '^cargo install --locked --path .*dir with space/bootstrap/\.\./crates/cargo-bk$' \
    "path with a space: cargo-bk installs from the clone"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[[ $fail -eq 0 ]]
