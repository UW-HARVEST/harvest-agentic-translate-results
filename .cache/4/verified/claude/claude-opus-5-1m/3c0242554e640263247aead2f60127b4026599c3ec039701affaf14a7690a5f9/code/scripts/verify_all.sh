#!/usr/bin/env bash
# Phase D driver: check + test every feature combination, in both profiles.
#
#   scripts/verify_all.sh
#
# `Cargo.toml` declares `default = []` and no other feature, so the complete set
# of valid feature combinations is {default} and {} (--no-default-features).
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
LOG=${TMPDIR:-/tmp}/verify_all.log
: > "$LOG"
fail=0

step() { printf '\n=== %s\n' "$*" | tee -a "$LOG"; }
runq() {
    printf '  $ %s\n' "$*" | tee -a "$LOG"
    if timeout 600 "$@" >>"$LOG" 2>&1; then
        echo "    OK"
    else
        echo "    FAILED (see $LOG)"
        fail=1
    fi
}

# --- feature combinations, derived from Cargo.toml -------------------------
mapfile -t FEATURES < <(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
step "declared non-default features: ${FEATURES[*]:-<none>}"
if [ "${#FEATURES[@]}" -ne 0 ]; then
    echo "ERROR: unexpected features declared; extend this script" | tee -a "$LOG"
    exit 1
fi

# --- C reference artifacts -------------------------------------------------
step "build C reference (executable + shared library)"
mkdir -p c_src/build build_c
runq cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON
runq cmake --build "$ROOT/c_src/build"
runq gcc -shared -fPIC -O2 -o "$ROOT/build_c/libcdriver.so" "$ROOT/c_src/src/main.c"

# --- per-combination check + test ------------------------------------------
for combo in "default" "no-default-features"; do
    if [ "$combo" = "default" ]; then
        FLAGS=()
    else
        FLAGS=(--no-default-features)
    fi
    for profile in dev release; do
        if [ "$profile" = "release" ]; then
            PROF=(--release)
        else
            PROF=()
        fi
        step "combo=$combo profile=$profile"
        runq cargo check --offline --all-targets "${FLAGS[@]}" "${PROF[@]}"
        runq cargo build --offline "${FLAGS[@]}" "${PROF[@]}"
        runq cargo test --offline "${FLAGS[@]}" "${PROF[@]}"
    done
done

# --- randomized end-to-end fuzz against the release binaries --------------
step "randomized stdin fuzz (release binaries, 2000 cases)"
if timeout 600 python3 "$ROOT/scripts/fuzz_diff.py" >>"$LOG" 2>&1; then
    echo "    OK"
else
    echo "    FAILED (see $LOG)"
    fail=1
fi

step "symbol diff (must be empty)"
diff <(nm -D --defined-only "$ROOT/build_c/libcdriver.so" | awk '{print $2, $3}' | sort) \
     <(nm -D --defined-only "$ROOT/target/release/libdriver.so" | awk '{print $2, $3}' | sort) \
     | tee -a "$LOG"

if [ "$fail" -eq 0 ]; then
    echo; echo "ALL CHECKS PASSED"
else
    echo; echo "FAILURES PRESENT — see $LOG"
fi
exit "$fail"
