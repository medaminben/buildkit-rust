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
    (cd "${RUN_CWD:-$PWD}" && HOME="$work/$name.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_MISSING="$missing" \
        bash "$script" "$@" >"$out" 2>&1)
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
assert_lacks "$calls" '^cargo bk doctor' "dry-run: doctor is not run"

# Nothing missing: nothing is installed. A copy without crates/ keeps the real cargo-bk
# crate out of it.
nocrates="$work/nocrates"
mkdir -p "$nocrates/bootstrap"
cp "$bootstrap" "$nocrates/bootstrap/bootstrap.sh"
run_case idem "" "$nocrates/bootstrap/bootstrap.sh" --yes --with editor
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

# Git Bash hands over to bootstrap.ps1 with the flags translated to its parameter names.
CALLS="$work/win.calls"
: >"$CALLS"
HOME="$work/win.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_UNAME="MINGW64_NT-10.0" \
    bash "$bootstrap" --dry-run --yes --without gui --with editor >"$work/win.out" 2>&1
rc=$?
assert_rc 0 "MINGW64_NT hand-over exits 0"
assert_has "$CALLS" '^powershell.exe .*bootstrap\.ps1' "MINGW64_NT execs powershell.exe ... bootstrap.ps1"
assert_has "$CALLS" ' -DryRun( |$)' "hand-over maps --dry-run to -DryRun"
assert_has "$CALLS" ' -Yes( |$)' "hand-over maps --yes to -Yes"
assert_has "$CALLS" ' -WithEditor( |$)' "hand-over maps --with editor to -WithEditor"
assert_has "$CALLS" ' -Without gui( |$)' "hand-over maps --without gui to -Without gui"
assert_lacks "$CALLS" ' --' "hand-over passes no -- flags"

# Several --without groups become one comma-separated -Without.
: >"$CALLS"
HOME="$work/win.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_UNAME="MINGW64_NT-10.0" \
    bash "$bootstrap" --without gui --without node >/dev/null 2>&1
assert_has "$CALLS" ' -Without gui,node( |$)' "hand-over joins several --without groups"

# Unknown flags and other --with groups exit 2 without handing over.
for bad_args in "--bogus" "--with gui"; do
    : >"$CALLS"
    # shellcheck disable=SC2086
    HOME="$work/win.home" PATH="$stubs:$PATH" CALLS="$CALLS" STUB_UNAME="MINGW64_NT-10.0" \
        bash "$bootstrap" $bad_args >/dev/null 2>&1
    rc=$?
    if [[ $rc -eq 2 ]]; then ok "hand-over rejects '$bad_args' with exit 2"; else bad "hand-over rejects '$bad_args' (rc=$rc)"; fi
    assert_lacks "$CALLS" '^powershell.exe' "no hand-over for '$bad_args'"
done

# A path with a space works, and cargo-bk installs from that path.
spaced="$work/dir with space"
mkdir -p "$spaced/bootstrap" "$spaced/crates/cargo-bk"
cp "$bootstrap" "$spaced/bootstrap/bootstrap.sh"
printf '[package]\nname = "cargo-bk"\nversion = "0.1.0"\n' >"$spaced/crates/cargo-bk/Cargo.toml"
run_case spaced cargo-bk "$spaced/bootstrap/bootstrap.sh" --yes
assert_rc 0 "path with a space: exits 0"
assert_has "$calls" '^cargo install --locked --path [^ ]*dir\\ with\\ space/bootstrap/\.\./crates/cargo-bk$' \
    "path with a space: --path arrives as one argument"

# cargo-bk is reinstalled unless its version equals the clone's exact tag.
tagged="$work/tagged"
mkdir -p "$tagged/bootstrap" "$tagged/crates/cargo-bk"
cp "$bootstrap" "$tagged/bootstrap/bootstrap.sh"
printf '[package]\nname = "cargo-bk"\nversion = "0.1.0-dev"\n' >"$tagged/crates/cargo-bk/Cargo.toml"
git_t() { git -C "$tagged" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
git_t init -q
git_t add -A
git_t commit -q -m one
git_t tag v1.2.3
STUB_BK_VERSION=1.2.3 run_case tagsame "" "$tagged/bootstrap/bootstrap.sh" --yes
assert_rc 0 "same tag: exits 0"
assert_lacks "$calls" '^cargo install --locked --path' "same tag: cargo-bk is not reinstalled"
assert_has "$out" 'cargo-bk already installed \(1\.2\.3\)' "same tag: says it is installed"
STUB_BK_VERSION=1.0.0 run_case tagdiff "" "$tagged/bootstrap/bootstrap.sh" --yes
assert_has "$calls" '^cargo install --locked --path' "different version: cargo-bk is installed"
git_t commit -q --allow-empty -m two
STUB_BK_VERSION=1.2.3 run_case notag "" "$tagged/bootstrap/bootstrap.sh" --yes
assert_has "$calls" '^cargo install --locked --path' "not exactly at a tag: cargo-bk is always installed"

# The last step is cargo bk doctor, with --machine outside a product.
product="$work/product/sub"
mkdir -p "$product"
printf '[workspace]\n[workspace.metadata.buildkit]\nversion = "0.1.0"\n' >"$work/product/Cargo.toml"
RUN_CWD="$product" run_case docin "" "$bootstrap" --yes
assert_has "$calls" '^cargo bk doctor$' "inside a product: cargo bk doctor"
RUN_CWD="$work" run_case docout "" "$bootstrap" --yes
assert_has "$calls" '^cargo bk doctor --machine$' "outside a product: cargo bk doctor --machine"
RUN_CWD="$work" run_case docnone cargo-bk "$bootstrap" --yes --without cargo-tools
assert_lacks "$calls" '^cargo bk doctor' "no cargo-bk: doctor is not run"
assert_has "$out" 'doctor: skipped \(cargo-bk is not installed\)' "no cargo-bk: says it skipped doctor"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[[ $fail -eq 0 ]]
