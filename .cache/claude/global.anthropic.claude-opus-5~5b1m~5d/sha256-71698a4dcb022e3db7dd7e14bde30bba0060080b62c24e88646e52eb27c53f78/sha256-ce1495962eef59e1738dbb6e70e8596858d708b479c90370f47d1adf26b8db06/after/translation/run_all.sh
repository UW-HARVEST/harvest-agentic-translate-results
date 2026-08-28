#!/usr/bin/env bash
# Full verification matrix: build both libraries, check symbol parity, then run
# every test under every feature combination and against both Rust build
# profiles. Automated so no step has to be repeated by hand.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAILED=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
fail() { echo "!!! FAILED: $*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "1. Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || fail "C build"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
step "2. cargo check (must be clean before anything else)"
# ---------------------------------------------------------------------------
cargo check --all-targets 2>&1 | tail -5 || fail "cargo check"

# ---------------------------------------------------------------------------
step "3. Build the Rust cdylib (release + debug)"
# ---------------------------------------------------------------------------
cargo build --release 2>&1 | tail -3 || fail "cargo build --release"
cargo build          2>&1 | tail -3 || fail "cargo build (debug)"
ls -l target/release/libbuffapp_lib.so target/debug/libbuffapp_lib.so

# ---------------------------------------------------------------------------
step "4. Phase A/D symbol parity (nm -D diff must be empty)"
# ---------------------------------------------------------------------------
bash ./check_symbols.sh || fail "symbol parity"

# ---------------------------------------------------------------------------
step "5. Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Every feature name declared under [features], excluding "default".
FEATURES="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)"

if [ -z "$FEATURES" ]; then
    echo "No [features] declared -> the only build configuration is the default."
    COMBOS=("DEFAULT" "NO_DEFAULT")
else
    echo "Declared features: $FEATURES"
    COMBOS=("DEFAULT" "NO_DEFAULT")
    # Full power set of the declared features.
    feats=($FEATURES)
    n=${#feats[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then combo="$combo,${feats[$i]}"; fi
        done
        COMBOS+=("FEAT:${combo#,}")
    done
fi
printf 'combinations to test: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
step "6. Run the full suite for every feature combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        DEFAULT)    args=() ;;
        NO_DEFAULT) args=(--no-default-features) ;;
        FEAT:*)     args=(--no-default-features --features "${combo#FEAT:}") ;;
    esac
    echo
    echo "--- cargo test ${args[*]:-<default>} ---"
    # Rebuild the cdylib with the same flags so the .so under test matches.
    cargo build --release "${args[@]}" >/dev/null 2>&1 || fail "build $combo"
    log="${TMPDIR:-/tmp}/combo_$$.log"
    if timeout 600 cargo test "${args[@]}" >"$log" 2>&1; then
        grep -E '^(test result|running)' "$log"
        echo "combination $combo: OK"
    else
        grep -E '^(test result|error|test .* FAILED|failures:)|panicked|assertion' "$log" | head -30
        fail "test $combo"
    fi
    rm -f "$log"
done

# ---------------------------------------------------------------------------
step "7. Re-run the suite against the DEBUG cdylib"
# ---------------------------------------------------------------------------
# The release profile uses panic=abort and full optimisation; the debug profile
# has overflow checks and no optimisation. Both must match the C byte-for-byte,
# so the differential suite is run against each .so.
cargo build --release >/dev/null 2>&1
cargo build >/dev/null 2>&1
log="${TMPDIR:-/tmp}/dbg_$$.log"
if RUST_SO="$(pwd)/target/debug/libbuffapp_lib.so" timeout 600 cargo test >"$log" 2>&1; then
    grep -E '^(test result|running)' "$log"
    echo "debug cdylib: OK"
else
    grep -E '^(test result|error|test .* FAILED|failures:)|panicked|assertion' "$log" | head -30
    fail "debug-profile suite"
fi
rm -f "$log"

# ---------------------------------------------------------------------------
step "8. Re-run the suite against the C compiled at every optimisation level"
# ---------------------------------------------------------------------------
# CMakeLists.txt sets no CMAKE_BUILD_TYPE, so the ground-truth build is -O0.
# The translation must also match gcc at -O1/-O2/-O3/-Os, which proves its
# undefined-behaviour choices (idiv trap, wrapping overflow) are not tied to one
# optimisation level. Compiled into target/ so nothing in c_src/ is touched.
mkdir -p target/copt
for opt in O0 O1 O2 O3 Os; do
    gcc "-$opt" -fPIC -shared -I"$ROOT/c_src/include" "$ROOT/c_src/src/lib.c" \
        -o "target/copt/libc_$opt.so" || { fail "gcc -$opt"; continue; }
    log="${TMPDIR:-/tmp}/copt_$$.log"
    if C_SO="$(pwd)/target/copt/libc_$opt.so" timeout 600 cargo test >"$log" 2>&1; then
        printf 'C -%-3s : %s\n' "$opt" \
            "$(grep -c '^test result: ok' "$log") test binaries OK"
    else
        grep -E '^(test result|test .* FAILED)|panicked' "$log" | head -20
        fail "C -$opt differential"
    fi
    rm -f "$log"
done

# ---------------------------------------------------------------------------
step "SUMMARY"
# ---------------------------------------------------------------------------
if [ "$FAILED" -eq 0 ]; then
    echo "ALL CHECKS PASSED"
else
    echo "THERE WERE FAILURES"
fi
exit "$FAILED"
