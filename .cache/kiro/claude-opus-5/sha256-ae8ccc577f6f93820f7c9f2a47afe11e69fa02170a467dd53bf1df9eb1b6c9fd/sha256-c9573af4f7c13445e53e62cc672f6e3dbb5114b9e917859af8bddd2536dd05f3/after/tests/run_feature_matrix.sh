#!/bin/bash
# Phase D: symbol parity + the whole differential suite under EVERY feature
# combination declared by translation/Cargo.toml.
#
# Usage: tests/run_feature_matrix.sh        (from the repository root)
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/tests/out"
mkdir -p "$OUT"
LOG="$OUT/feature_matrix.txt"
: > "$LOG"

say() { echo "$@" | tee -a "$LOG"; }

# ---------------------------------------------------------------- C reference
if [ ! -f "$ROOT/c_src/build/libjansson.so" ]; then
    say "building the C reference library"
    (mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null) || { say "C build FAILED"; exit 1; }
fi

# ------------------------------------------------------- enumerate feature sets
# Features are read out of Cargo.toml's [features] table; if there is none, the
# only configuration is the default (empty) feature set.
FEATURES=$(awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {gsub(/[[:space:]]*=.*/,""); print}
' "$ROOT/translation/Cargo.toml" | grep -v '^default$' | sort -u)

say "features declared in Cargo.toml: ${FEATURES:-<none>}"

COMBOS=()
COMBOS+=("DEFAULT|")                      # default feature set
COMBOS+=("NO_DEFAULT|--no-default-features")
COMBOS+=("ALL|--all-features")
if [ -n "$FEATURES" ]; then
    # every individual feature, and the powerset if it is small enough
    for f in $FEATURES; do
        COMBOS+=("$f|--no-default-features --features $f")
    done
    n=$(echo "$FEATURES" | wc -l)
    if [ "$n" -le 8 ]; then
        arr=($FEATURES)
        total=$((1 << n))
        for ((m = 0; m < total; m++)); do
            sel=""
            for ((b = 0; b < n; b++)); do
                if (((m >> b) & 1)); then sel="$sel,${arr[$b]}"; fi
            done
            sel="${sel#,}"
            [ -z "$sel" ] && continue
            COMBOS+=("SET[$sel]|--no-default-features --features $sel")
        done
    fi
fi

# --------------------------------------------------------------------- run them
FAIL=0
cd "$ROOT/translation"
for combo in "${COMBOS[@]}"; do
    name="${combo%%|*}"
    flags="${combo#*|}"
    say ""
    say "================ $name  (cargo flags: ${flags:-<default>}) ================"

    if ! timeout 600 cargo check $flags >/tmp/fm_check.log 2>&1; then
        say "  cargo check FAILED"; tail -20 /tmp/fm_check.log | tee -a "$LOG"; FAIL=1; continue
    fi
    say "  cargo check: ok"

    if ! timeout 600 cargo build --release $flags >/tmp/fm_build.log 2>&1; then
        say "  cargo build --release FAILED"; tail -20 /tmp/fm_build.log | tee -a "$LOG"; FAIL=1; continue
    fi

    # symbol parity for THIS configuration
    nm -D --defined-only "$ROOT/c_src/build/libjansson.so" |
        awk '{print $2" "$3}' | sort -k2 > /tmp/fm_c.txt
    nm -D --defined-only "$ROOT/translation/target/release/libjansson.so" |
        awk '{print $2" "$3}' | sort -k2 > /tmp/fm_r.txt
    nc=$(wc -l < /tmp/fm_c.txt); nr=$(wc -l < /tmp/fm_r.txt)
    missing=$(comm -23 <(awk '{print $2}' /tmp/fm_c.txt) <(awk '{print $2}' /tmp/fm_r.txt))
    say "  exported symbols: C=$nc Rust=$nr"
    if [ -n "$missing" ]; then
        say "  MISSING FROM RUST: $missing"; FAIL=1
    else
        say "  symbol diff: EMPTY"
    fi
    if ! diff -q /tmp/fm_c.txt /tmp/fm_r.txt >/dev/null; then
        say "  nm (type,name) pairs DIFFER:"; diff /tmp/fm_c.txt /tmp/fm_r.txt | tee -a "$LOG"; FAIL=1
    else
        say "  nm (type,name) pairs: identical"
    fi

    if ! timeout 600 cargo test --release $flags >/tmp/fm_test.log 2>&1; then
        say "  cargo test FAILED"; grep -E "^(test |error|assert)" /tmp/fm_test.log | tail -40 | tee -a "$LOG"; FAIL=1; continue
    fi
    passed=$(grep -oE '[0-9]+ passed' /tmp/fm_test.log | awk '{s+=$1} END{print s}')
    failed=$(grep -oE '[0-9]+ failed' /tmp/fm_test.log | awk '{s+=$1} END{print s}')
    say "  cargo test --release: $passed passed, $failed failed"
    [ "${failed:-0}" != "0" ] && FAIL=1
done

say ""
if [ "$FAIL" = 0 ]; then
    say "RESULT: all feature combinations pass with an EMPTY symbol diff"
else
    say "RESULT: FAILURES — see above"
fi
exit $FAIL
