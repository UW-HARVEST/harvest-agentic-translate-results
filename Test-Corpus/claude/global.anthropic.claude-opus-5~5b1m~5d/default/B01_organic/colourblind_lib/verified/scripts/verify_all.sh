#!/usr/bin/env bash
# One-shot driver for the whole verification: Phases A-D plus the anti-vacuity
# gates. Exits non-zero if anything fails.
set -uo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname -- "$here")"
root="$(dirname -- "$crate")"
cd "$crate" || exit 1

status=0
step() {
    local name="$1"
    shift
    echo
    echo "################################################################"
    echo "# $name"
    echo "################################################################"
    if "$@"; then
        echo "--> OK: $name"
    else
        echo "--> FAILED: $name"
        status=1
    fi
}

build_c() {
    mkdir -p "$root/c_src/build"
    ( cd "$root/c_src/build" \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null )
}

step "Phase A: build the C shared library" build_c
step "compile cleanly (clippy, all targets, denying warnings)" \
    timeout 600 cargo clippy --release --all-targets -- -D warnings
step "Phase D: symbol parity (release)" "$here/symbol_parity.sh"
step "Phases B+C+D: full suite across every feature combo and both profiles" \
    "$here/feature_matrix.sh"
step "artifact gate: every CONFIGS.md / ERRORS.md row maps to a passing test" \
    timeout 600 python3 "$here/check_artifacts.py"
step "anti-vacuity gate: mutation testing" \
    timeout 600 python3 "$here/mutation_check.py"

echo
if [ "$status" -eq 0 ]; then
    echo "================================================================"
    echo "ALL VERIFICATION GATES PASSED"
    echo "================================================================"
else
    echo "================================================================"
    echo "VERIFICATION FAILED — see the FAILED lines above"
    echo "================================================================"
fi
exit "$status"
