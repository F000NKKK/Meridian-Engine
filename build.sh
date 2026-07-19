#!/usr/bin/env bash
# Meridian-Engine build — a dotnet-style task runner for the Cargo workspace.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CONFIG="Release"   # Debug | Release

usage() {
    cat <<'EOF'
Meridian-Engine build.

Usage: ./build.sh <command> [args] [options]

Commands:
  build [crate]         Compile the workspace, or a single crate (-p <crate>)
  test [crate]          Run tests (cargo test), or a single crate
  check                 cargo check --workspace
  check-deps            Verify the crate graph matches docs/dependency-rules.md
  clippy                cargo clippy --workspace --all-targets
  fmt                   cargo fmt --all
  run <example> [-- args...]
                        Build and run examples/examples/<example>.rs, forwarding args
  list-examples         List available examples
  clean                 cargo clean

Options:
  -c, --configuration <Debug|Release>   default: Release
  -h, --help

Examples:
  ./build.sh build
  ./build.sh build -p meridian-gac-core
  ./build.sh test
  ./build.sh run hello_engine -- --foo bar
  ./build.sh clean
EOF
}

die() { echo "error: $*" >&2; exit 1; }

cargo_flag() { [ "$CONFIG" = "Release" ] && echo "--release" || echo ""; }

cmd_build() {
    local pkg="${1:-}"
    if [ -n "$pkg" ] && [ "$pkg" != "-p" ]; then
        cargo build $(cargo_flag) --manifest-path "$ROOT/Cargo.toml" -p "$pkg"
    else
        cargo build $(cargo_flag) --manifest-path "$ROOT/Cargo.toml" --workspace
    fi
}

cmd_test() {
    local pkg="${1:-}"
    if [ -n "$pkg" ]; then
        cargo test $(cargo_flag) --manifest-path "$ROOT/Cargo.toml" -p "$pkg"
    else
        cargo test $(cargo_flag) --manifest-path "$ROOT/Cargo.toml" --workspace
    fi
}

cmd_check() {
    cargo check --manifest-path "$ROOT/Cargo.toml" --workspace
}

cmd_check_deps() {
    python3 "$ROOT/scripts/check_dependency_rules.py"
}

cmd_clippy() {
    cargo clippy --manifest-path "$ROOT/Cargo.toml" --workspace --all-targets
}

cmd_fmt() {
    cargo fmt --manifest-path "$ROOT/Cargo.toml" --all
}

cmd_list_examples() {
    find "$ROOT/examples/examples" -maxdepth 1 -name '*.rs' -exec basename {} .rs \; | sort
}

# ./build.sh run <example> [-- args...]
cmd_run() {
    local example="${1:-}"
    [ -n "$example" ] || die "run needs an example name (see: ./build.sh list-examples)"
    shift || true
    if [ "${1:-}" = "--" ]; then shift; fi
    cargo run $(cargo_flag) --manifest-path "$ROOT/Cargo.toml" -p meridian-examples --example "$example" -- "$@"
}

cmd_clean() {
    cargo clean --manifest-path "$ROOT/Cargo.toml"
}

[ $# -eq 0 ] && { usage; exit 0; }
cmd="$1"; shift

args=()
while [ $# -gt 0 ]; do
    case "$1" in
        -c|--configuration)
            case "${2:-}" in
                [Dd]ebug)   CONFIG="Debug" ;;
                [Rr]elease) CONFIG="Release" ;;
                *) die "configuration must be Debug or Release" ;;
            esac
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) args+=("$1"); shift ;;
    esac
done

case "$cmd" in
    build)          cmd_build "${args[@]:-}" ;;
    test)           cmd_test "${args[@]:-}" ;;
    check)          cmd_check ;;
    check-deps)     cmd_check_deps ;;
    clippy)         cmd_clippy ;;
    fmt)            cmd_fmt ;;
    run)            cmd_run "${args[@]}" ;;
    list-examples)  cmd_list_examples ;;
    clean)          cmd_clean ;;
    -h|--help|help) usage ;;
    *) echo "unknown command: $cmd" >&2; usage; exit 2 ;;
esac
