#!/usr/bin/env bash
# Shared helpers for the stubs. Sourced, never run on its own.
# Each stub appends its argv to $CALLS and acts "missing" when its name (or "all")
# appears in the comma-separated $STUB_MISSING.

stub_log() { printf '%s\n' "$*" >>"${CALLS:?CALLS must be set}"; }

stub_missing() {
    local item
    local -a items
    IFS=, read -ra items <<<"${STUB_MISSING:-}"
    for item in "${items[@]}"; do
        [[ "$item" == "$1" || "$item" == all ]] && return 0
    done
    return 1
}

# A tool that only answers --version: exit 127 when missing.
stub_version_tool() {
    local name="$1" text="$2"
    shift 2
    stub_log "$name $*"
    stub_missing "$name" && exit 127
    [[ "${1:-}" == "--version" ]] && echo "$text"
    exit 0
}
