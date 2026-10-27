#!/usr/bin/env bash
# The checks CI runs, one subcommand per job, so that a local run checks exactly what CI does.
#
#   ci/check.sh static                      formatting, and the source rules no compiler checks
#   ci/check.sh lint dev|release            clippy on every feature combination, then the tests
#   ci/check.sh doc                         the docs on every feature combination, the private
#                                           items, and the doctests
#   ci/check.sh test dev|release [ARGS...]  the test suite; ARGS go to nextest (`--test map`)
#   ci/check.sh msrv                        the library on the MSRV, on every feature combination
#   ci/check.sh all                         all of the above, in both profiles
#
# CI is the product of the tools (rustc, clippy, rustdoc, the tests) and the configurations the
# code compiles differently in (the features, the profile, the toolchain). Every configuration is
# checked with the strictest tool that applies: clippy wherever rustc alone would do, since clippy
# runs every rustc lint too. `ci/cfg_coverage.py` keeps that list of configurations complete.
#
# The warnings are denied here rather than in the workflow, so a local run is as strict as CI.
# The MSRV job is the exception: it only checks that the code builds there, and lints it does not
# know, or knows differently, are not its business.

set -euo pipefail
cd "$(dirname "$0")/.."

# `--optional-deps futures`: `futures` is the implicit feature of the optional dependency, which a
# user can enable on its own; cargo-hack leaves it out unless told. `--no-dev-deps`: the tests
# depend on the crate itself with every feature, which would otherwise enable them all. It rewrites
# Cargo.toml for the duration of the run, which is why these runs cannot be `--locked`, and why two
# of them must not run at once in one checkout: each restores the manifest the other rewrote.
POWERSET=(--feature-powerset --optional-deps futures --no-dev-deps)

strict() {
    RUSTFLAGS="-D warnings" RUSTDOCFLAGS="-D warnings" "$@"
}

# The cargo flag of a profile: `dev`, the default, or `release`, whose builds leave out the
# `cfg(debug_assertions)` code, and which only a release run therefore lints or tests.
profile_flag() {
    case "${1:-}" in
        dev) ;;
        release) echo --release ;;
        *)
            echo "expected a profile, dev or release, got '${1:-}'" >&2
            exit 2
            ;;
    esac
}

require() {
    if ! cargo "$1" --version >/dev/null 2>&1; then
        echo "cargo $1 is not installed: cargo install cargo-$1" >&2
        exit 2
    fi
}

run_static() {
    cargo fmt --all --check
    # Time comes from the scheduler's clock (`SchedulerTypes::clock`, or the `now` a task step
    # gets), so that a virtual-time scheduler drives every operator. Only `SystemClock::now` and
    # the origin of a virtual clock read it directly; comments and doc examples are skipped.
    if grep -rn 'Instant::now' src |
        grep -vE '^[^:]+:[0-9]+:\s*//' |
        grep -vE '^src/scheduler/(mod|virtual_time)\.rs:'; then
        echo "Read the time from the scheduler instead of Instant::now()" >&2
        exit 1
    fi
    python3 ci/cfg_coverage.py
}

run_lint() {
    local flag
    flag=$(profile_flag "${1:-}")
    require hack
    # The library alone, in every combination of its features: what a user compiles.
    strict cargo hack clippy "${POWERSET[@]}" --lib $flag
    # Everything, tests and unit tests included, which enable every feature.
    strict cargo clippy --locked --all-targets $flag
}

run_doc() {
    require hack
    # The crate documentation itself depends on the features (see `src/lib.rs`).
    strict cargo hack doc "${POWERSET[@]}" --no-deps --lib
    # Same features docs.rs builds with (see `[package.metadata.docs.rs]`), plus the private items,
    # whose links only this run resolves.
    strict cargo doc --locked --no-deps --all-features
    strict cargo doc --locked --no-deps --all-features --document-private-items
    # nextest does not run doctests, and the README examples are doctests. Every feature, so that
    # the examples of every scheduler run.
    strict cargo test --locked --doc --all-features
}

run_test() {
    local flag
    flag=$(profile_flag "${1:-}")
    shift
    require nextest
    # One build: `block_on` runs each async test on every scheduler in turn. CI sets
    # `NEXTEST_PROFILE=ci` (see `.config/nextest.toml`).
    strict cargo nextest run --locked $flag "$@"
}

run_msrv() {
    local msrv
    msrv=$(sed -n 's/^rust-version = "\(.*\)"$/\1/p' Cargo.toml)
    require hack
    # Every combination of the features, so that the code of every scheduler is checked against
    # the MSRV. No lint is denied: see the top of this file.
    cargo "+$msrv" hack check "${POWERSET[@]}" --lib
}

command="${1:-}"
shift || true
case "$command" in
    static) run_static ;;
    lint) run_lint "$@" ;;
    doc) run_doc ;;
    test) run_test "$@" ;;
    msrv) run_msrv ;;
    all)
        run_static
        run_lint dev
        run_lint release
        run_doc
        run_test dev
        run_test release
        run_msrv
        ;;
    *)
        sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//' >&2
        exit 2
        ;;
esac
