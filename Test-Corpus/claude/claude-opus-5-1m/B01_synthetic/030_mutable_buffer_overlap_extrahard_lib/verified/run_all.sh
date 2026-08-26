#!/usr/bin/env bash
# Full verification run: every build-time configuration x every phase.
#
#   ./run_all.sh              # everything
#   ./run_all.sh quick        # skip the slow fork-heavy suites
#
# Phase A artifacts: SYMBOLS.md, ERRORS.md, CONFIGS.md
# Phase B:           tests/phase_b_configs.rs        (one test per CONFIGS.md row)
# Phase C:           tests/phase_c_errors.rs         (one test per ERRORS.md row)
#                    tests/negative_len_analysis.rs  (ERRORS.md rows 14-16 rationale)
#                    tests/oob_read_analysis.rs      (ERRORS.md rows 26-27 rationale)
# Phase D:           tests/phase_d_symbols.rs + ./check_symbols.sh
set -uo pipefail
cd "$(dirname "$0")"

QUICK=${1:-}
CARGO="cargo"
OFFLINE=""
if [[ -n ${CARGO_NET_OFFLINE:-} || -d ${CARGO_HOME:-$HOME/.cargo}/registry/cache ]]; then
    OFFLINE="--offline"
fi

fail=0
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
check() { if [[ $1 -ne 0 ]]; then echo "!! FAILED: $2"; fail=1; else echo "   ok: $2"; fi; }

# ---------------------------------------------------------------------------
# 0. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Building the C reference library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
check $? "c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 1. Phase A: enumerate every valid feature combination and `cargo check` it.
#
#    Cargo.toml has no [features] section, so the powerset of the feature set is
#    the single empty combination. Both spellings are still exercised, and the
#    list is derived mechanically so that adding a feature later cannot be
#    silently skipped.
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [[ -z ${FEATURES// /} ]]; then
    echo "   no [features] in Cargo.toml -> exactly one valid combination (empty)"
    COMBOS=("")
else
    # Full powerset of the declared features.
    mapfile -t FEATS <<<"$FEATURES"
    n=${#FEATS[@]}
    COMBOS=()
    for ((m = 0; m < (1 << n); m++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( m & (1 << i) )); then combo="${combo:+$combo,}${FEATS[i]}"; fi
        done
        COMBOS+=("$combo")
    done
    printf '   %d features -> %d combinations\n' "$n" "${#COMBOS[@]}"
fi

step "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
    if [[ -z $combo ]]; then
        $CARGO check $OFFLINE --no-default-features --all-targets >/dev/null 2>&1
        check $? "cargo check --no-default-features --all-targets"
        $CARGO check $OFFLINE --all-targets >/dev/null 2>&1
        check $? "cargo check --all-targets (default features)"
    else
        $CARGO check $OFFLINE --no-default-features --features "$combo" --all-targets >/dev/null 2>&1
        check $? "cargo check --no-default-features --features $combo"
    fi
done

# ---------------------------------------------------------------------------
# 2. Build the Rust shared library in both profiles, for every combination
# ---------------------------------------------------------------------------
step "Building the Rust shared library (dev + release)"
for combo in "${COMBOS[@]}"; do
    args=(--no-default-features)
    [[ -n $combo ]] && args+=(--features "$combo")
    $CARGO build $OFFLINE "${args[@]}" >/dev/null 2>&1
    check $? "cargo build ${args[*]}"
    $CARGO build $OFFLINE --release "${args[@]}" >/dev/null 2>&1
    check $? "cargo build --release ${args[*]}"
done

# ---------------------------------------------------------------------------
# 3. Phase D: symbol parity (script form)
# ---------------------------------------------------------------------------
step "Phase D: symbol parity (nm -D diff)"
./check_symbols.sh >/dev/null 2>&1
check $? "check_symbols.sh (C exports == Rust exports)"

# ---------------------------------------------------------------------------
# 4. Phases B, C, D against BOTH the dev and the release shared library, for
#    every feature combination.
# ---------------------------------------------------------------------------
SUITES=(phase_d_symbols phase_b_configs oob_read_analysis)
if [[ $QUICK != quick ]]; then
    # Fork-heavy: every case runs the call in a subprocess so a fault or a spin
    # is a comparable outcome rather than a dead test run.
    SUITES+=(phase_c_errors negative_len_analysis)
fi

for combo in "${COMBOS[@]}"; do
    args=(--no-default-features)
    [[ -n $combo ]] && args+=(--features "$combo")
    label=${combo:-<no features>}
    for profile in debug release; do
        so="target/$profile/libdriver.so"
        [[ -f $so ]] || continue
        export DRIVER_RUST_SO="$PWD/$so"
        for suite in "${SUITES[@]}"; do
            step "features=[$label] profile=$profile suite=$suite"
            timeout 580 $CARGO test $OFFLINE "${args[@]}" --test "$suite" -- --test-threads=1 \
                >"${TMPDIR:-/tmp}/driver-$suite-$profile.log" 2>&1
            rc=$?
            check $rc "$suite ($profile)"
            [[ $rc -ne 0 ]] && tail -30 "${TMPDIR:-/tmp}/driver-$suite-$profile.log"
        done
    done
done
unset DRIVER_RUST_SO

# ---------------------------------------------------------------------------
step "Result"
if [[ $fail -eq 0 ]]; then
    echo "ALL CHECKS PASSED"
else
    echo "SOME CHECKS FAILED"
fi
exit $fail
