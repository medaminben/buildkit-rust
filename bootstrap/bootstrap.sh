#!/usr/bin/env bash
# Installs everything needed to work on Rust products on a fresh machine.
#
# Linux: Debian / Ubuntu (apt). Other distributions get the package list to install by hand.
# Windows: run bootstrap.ps1 in PowerShell. Started from Git Bash or MSYS2, this script hands
# over to it.
#
# Safe to run again: installed packages and tools are skipped. Rust and the packaging tool go
# into your home directory (~/.cargo, ~/.local/bin); only apt packages need sudo.
#
# cargo-bk is installed from the clone this script lives in, so the bootstrap and `cargo bk`
# always come from the same tag.
#
# Usage:
#   ./bootstrap.sh                  every group except the optional ones
#   ./bootstrap.sh --minimal        base, rust and cargo-tools only
#   ./bootstrap.sh --without gui    leave out a group (repeatable)
#   ./bootstrap.sh --with editor    add the optional group: editor (VS Code extensions)
#   ./bootstrap.sh --dry-run        print what would be installed
#   ./bootstrap.sh --yes            no questions
#   ./bootstrap.sh --list           show the groups and what they install

set -euo pipefail

# appimagetool is pinned. Rust code in buildkit-rust uses these same values: change them together.
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"

RUSTUP_DIST="https://static.rust-lang.org/rustup/dist"
BINSTALL_URL="https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz"
PYTHON_MIN="3.11"
EDITOR_EXTENSIONS=(rust-lang.rust-analyzer vadimcn.vscode-lldb tamasfe.even-better-toml)

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
        # -File passes arguments as literal strings, so translate each flag to the parameter
        # name bootstrap.ps1 defines.
        ps_args=()
        ps_without=()
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --list) ps_args+=(-List) ;;
                --dry-run) ps_args+=(-DryRun) ;;
                --yes | -y) ps_args+=(-Yes) ;;
                --minimal) ps_args+=(-Minimal) ;;
                --with)
                    [[ "${2:-}" == editor ]] || { echo "Unsupported group for --with: ${2:-}" >&2; exit 2; }
                    ps_args+=(-WithEditor)
                    shift
                    ;;
                --without)
                    [[ -n "${2:-}" ]] || { echo "--without needs a group" >&2; exit 2; }
                    ps_without+=("$2")
                    shift
                    ;;
                -h | --help)
                    sed -n '2,/^$/s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
                    exit 0
                    ;;
                *) echo "Unknown option: $1" >&2; exit 2 ;;
            esac
            shift
        done
        if [[ ${#ps_without[@]} -gt 0 ]]; then
            ps_list="$(IFS=,; echo "${ps_without[*]}")"
            ps_args+=(-Without "$ps_list")
        fi
        echo "Windows detected: running bootstrap.ps1"
        exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w "$script_dir/bootstrap.ps1")" "${ps_args[@]}"
        ;;
    Linux) ;;
    *)
        echo "Unsupported system: $(uname -s). Use Linux (Debian/Ubuntu) or Windows (bootstrap.ps1)." >&2
        exit 1
        ;;
esac

# apt packages per group. The groups without an entry are installed by special steps below.
declare -A GROUP
GROUP[base]="git curl ca-certificates build-essential pkgconf"
GROUP[rust]=""
GROUP[cargo-tools]=""
GROUP[gui]="libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libvulkan1 mesa-vulkan-drivers libgl1-mesa-dri libegl1"
GROUP[node]="nodejs npm"
GROUP[python]="python3 python3-venv python3-pip"
GROUP[github]="gh"
GROUP[packaging]="dpkg-dev"
GROUP[editor]=""

# What the special steps install, for --list.
declare -A SPECIAL
SPECIAL[rust]="rustup, stable with rustfmt clippy llvm-tools-preview, cargo-binstall"
SPECIAL[cargo-tools]="cargo-bk (from this clone), cargo-deny, cargo-llvm-cov, cargo-about"
SPECIAL[packaging]="appimagetool (pinned, to ~/.local/bin)"
SPECIAL[editor]="VS Code extensions: ${EDITOR_EXTENSIONS[*]}"

DEFAULT_GROUPS=(base rust cargo-tools gui node python github packaging)
MINIMAL_GROUPS=(base rust cargo-tools)
OPTIONAL_GROUPS=(editor)
ALL_GROUPS=("${DEFAULT_GROUPS[@]}" "${OPTIONAL_GROUPS[@]}")

dry_run=false
assume_yes=false
minimal=false
declare -A without=()
declare -A with=()

usage() { sed -n '2,/^$/s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"; }

is_group() { local g; for g in "${ALL_GROUPS[@]}"; do [[ "$g" == "$1" ]] && return 0; done; return 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --minimal) minimal=true ;;
        --without)
            is_group "${2:?--without needs a group}" || { echo "Unknown group: $2" >&2; exit 1; }
            without["$2"]=1
            shift
            ;;
        --with)
            is_group "${2:?--with needs a group}" || { echo "Unknown group: $2" >&2; exit 1; }
            with["$2"]=1
            shift
            ;;
        --dry-run) dry_run=true ;;
        --yes | -y) assume_yes=true ;;
        --list)
            for g in "${ALL_GROUPS[@]}"; do
                printf '%-12s %s\n' "$g" "${GROUP[$g]:-}${SPECIAL[$g]:+ ${SPECIAL[$g]}}"
            done
            exit 0
            ;;
        -h | --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
    shift
done

if $minimal; then groups=("${MINIMAL_GROUPS[@]}"); else groups=("${DEFAULT_GROUPS[@]}"); fi
for g in "${OPTIONAL_GROUPS[@]}"; do [[ -n "${with[$g]:-}" ]] && groups+=("$g"); done
selected=()
for g in "${groups[@]}"; do [[ -z "${without[$g]:-}" ]] && selected+=("$g"); done
has_group() { local g; for g in "${selected[@]}"; do [[ "$g" == "$1" ]] && return 0; done; return 1; }

say() { printf '\n==> %s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
run() {
    if $dry_run; then printf '[dry-run] %s\n' "$*"; else "$@"; fi
}
# have <tool>: the tool answers --version.
have() { "$1" --version >/dev/null 2>&1; }
download() { # <url> <file>
    run curl --proto '=https' --tlsv1.2 -fsSL --retry 3 -o "$2" "$1"
}

SUDO=()
if [[ $EUID -ne 0 ]]; then
    command -v sudo >/dev/null || { echo "Run as root or install sudo." >&2; exit 1; }
    SUDO=(sudo)
fi

if ! command -v apt-get >/dev/null; then
    echo "No apt-get found. Install these packages with your distribution's package manager:" >&2
    for g in "${selected[@]}"; do
        [[ -n "${GROUP[$g]:-}" ]] && echo "  $g: ${GROUP[$g]}" >&2
    done
    echo "Also needed: rustup (https://rustup.rs), appimagetool ${APPIMAGETOOL_URL}" >&2
    exit 1
fi

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

# shellcheck disable=SC1091
. /etc/os-release
say "System: ${PRETTY_NAME:-Linux}. Groups: ${selected[*]}"
if ! $assume_yes && ! $dry_run; then
    read -r -p "Continue? [Y/n] " answer
    [[ "${answer:-y}" =~ ^[Yy]$ ]] || exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# --- apt packages ---------------------------------------------------------------------------
wanted=()
for g in "${selected[@]}"; do
    # shellcheck disable=SC2206
    [[ -n "${GROUP[$g]:-}" ]] && wanted+=(${GROUP[$g]})
done

missing_pkgs=()
for p in "${wanted[@]}"; do
    dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing_pkgs+=("$p")
done

if [[ ${#missing_pkgs[@]} -gt 0 ]]; then
    say "Updating the package index"
    run "${SUDO[@]}" apt-get update -qq

    to_install=()
    unavailable=()
    for p in "${missing_pkgs[@]}"; do
        if apt-cache show "$p" >/dev/null 2>&1; then to_install+=("$p"); else unavailable+=("$p"); fi
    done
    [[ ${#unavailable[@]} -gt 0 ]] && warn "not available on ${ID:-this system}, skipped: ${unavailable[*]}"

    if [[ ${#to_install[@]} -gt 0 ]]; then
        say "Installing ${#to_install[@]} packages: ${to_install[*]}"
        run "${SUDO[@]}" env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${to_install[@]}"
    fi
else
    say "All apt packages already installed"
fi

# --- Rust -----------------------------------------------------------------------------------
RUST_COMPONENTS=(rustfmt clippy llvm-tools-preview)

install_rust() {
    local triple="x86_64-unknown-linux-gnu"
    case "$(uname -m)" in aarch64 | arm64) triple="aarch64-unknown-linux-gnu" ;; esac
    say "Installing rustup and Rust stable into ~/.cargo"
    download "$RUSTUP_DIST/$triple/rustup-init" "$tmp/rustup-init"
    run chmod +x "$tmp/rustup-init"
    run "$tmp/rustup-init" -y --default-toolchain stable --profile minimal -c rustfmt -c clippy -c llvm-tools-preview
}

ensure_rust_components() {
    if ! rustup toolchain list 2>/dev/null | grep -q '^stable'; then
        say "Installing the stable toolchain"
        run rustup toolchain install stable --profile minimal -c rustfmt -c clippy -c llvm-tools-preview
        run rustup default stable
        return
    fi
    local installed c
    installed="$(rustup component list --installed 2>/dev/null || true)"
    for c in "${RUST_COMPONENTS[@]}"; do
        # The listing says "llvm-tools-<triple>" for the component added as llvm-tools-preview.
        if ! grep -q "^${c%-preview}" <<<"$installed"; then
            say "Adding the rustup component $c"
            run rustup component add "$c"
        fi
    done
}

ensure_binstall() {
    cargo binstall -V >/dev/null 2>&1 && return 0
    say "Installing cargo-binstall (prebuilt)"
    download "$BINSTALL_URL" "$tmp/cargo-binstall.tgz"
    run mkdir -p "$HOME/.cargo/bin"
    run tar -xzf "$tmp/cargo-binstall.tgz" -C "$HOME/.cargo/bin" cargo-binstall
}

if has_group rust; then
    if have rustup; then
        say "rustup already installed"
        ensure_rust_components
    else
        install_rust
    fi
    if $dry_run || have cargo; then ensure_binstall; fi
fi

# --- cargo tools ----------------------------------------------------------------------------
# cargo_tool <crate> <subcommand>: install through binstall, falling back to a source build.
cargo_tool() {
    local crate="$1" sub="$2"
    if cargo "$sub" --version >/dev/null 2>&1; then
        say "$crate already installed"
        return
    fi
    say "Installing $crate"
    if $dry_run; then
        echo "[dry-run] cargo binstall --no-confirm --locked $crate || cargo install --locked $crate"
    else
        cargo binstall --no-confirm --locked "$crate" || cargo install --locked "$crate"
    fi
}

install_cargo_bk() {
    local src="$script_dir/../crates/cargo-bk" want have_version
    if [[ ! -d "$src" ]]; then
        echo "cargo-bk: skipped (not in this checkout)"
        return
    fi
    want="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$src/Cargo.toml" 2>/dev/null | head -1)"
    have_version="$(cargo bk --version 2>/dev/null || true)"
    if [[ -n "$have_version" && ( -z "$want" || "$have_version" == *"$want"* ) ]]; then
        say "cargo-bk already installed ($have_version)"
        return
    fi
    say "Installing cargo-bk from this clone"
    run cargo install --locked --path "$src"
}

if has_group cargo-tools; then
    if ! $dry_run && ! have cargo; then
        warn "cargo not found: skipping cargo-tools (install the rust group first)"
    else
        install_cargo_bk
        cargo_tool cargo-deny deny
        cargo_tool cargo-llvm-cov llvm-cov
        cargo_tool cargo-about about
    fi
fi

# --- python version -------------------------------------------------------------------------
if has_group python && have python3; then
    py_version="$(python3 --version 2>/dev/null | awk '{print $2}')"
    if [[ -n "$py_version" && "$(printf '%s\n' "$PYTHON_MIN" "$py_version" | sort -V | head -1)" != "$PYTHON_MIN" ]]; then
        warn "python3 $py_version is older than $PYTHON_MIN; the release and manual tools need $PYTHON_MIN or newer"
    fi
fi

# --- appimagetool ---------------------------------------------------------------------------
if has_group packaging; then
    if [[ -x "$HOME/.local/bin/appimagetool" ]]; then
        say "appimagetool already installed"
    else
        say "Installing appimagetool into ~/.local/bin"
        download "$APPIMAGETOOL_URL" "$tmp/appimagetool"
        if $dry_run; then
            echo "[dry-run] verify SHA-256 $APPIMAGETOOL_SHA256"
        else
            echo "$APPIMAGETOOL_SHA256  $tmp/appimagetool" | sha256sum -c --quiet - \
                || { echo "appimagetool: SHA-256 mismatch, not installing" >&2; exit 1; }
        fi
        run mkdir -p "$HOME/.local/bin"
        run install -m 755 "$tmp/appimagetool" "$HOME/.local/bin/appimagetool"
    fi
fi

# --- editor ---------------------------------------------------------------------------------
if has_group editor; then
    if ! $dry_run && ! have code; then
        warn "VS Code (code) not found: install it from https://code.visualstudio.com, then run this again"
    else
        installed_ext="$(code --list-extensions 2>/dev/null || true)"
        for ext in "${EDITOR_EXTENSIONS[@]}"; do
            if grep -qix "$ext" <<<"$installed_ext"; then
                say "$ext already installed"
            else
                run code --install-extension "$ext"
            fi
        done
    fi
fi

# --- Summary --------------------------------------------------------------------------------
say "Installed versions"
check() {
    local name="$1" out
    shift
    if out="$("$@" 2>/dev/null | head -1)" && [[ -n "$out" ]]; then printf '  %-14s %s\n' "$name" "$out"
    else printf '  %-14s %s\n' "$name" "-"; fi
}
check git          git --version
check gcc          gcc --version
check rustup       rustup --version
check rustc        rustc --version
check cargo        cargo --version
check cargo-bk     cargo bk --version
check cargo-deny   cargo deny --version
check llvm-cov     cargo llvm-cov --version
check cargo-about  cargo about --version
check node         node --version
check python       python3 --version
check gh           gh --version
check appimagetool "$HOME/.local/bin/appimagetool" --version
check code         code --version

say "Done. Open a new shell so PATH changes (~/.cargo/bin, ~/.local/bin) apply."
# Task 14 adds the last step here: `cargo bk doctor`.
