#!/usr/bin/env bash
# Full verification driver: builds the C reference .so, then for EVERY cargo
# feature combination and BOTH cargo profiles it
#   1. builds the Rust cdylib (cargo test does NOT re-link a cdylib),
#   2. diffs `nm -D` between the two shared objects (must be empty),
#   3. runs the Phase A/B/C differential test suites.
#
# Usage: ./run_all.sh          (from the `translation` directory)
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
CSRC="$ROOT/c_src"
CARGO_FLAGS="--offline"
FAILED=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. Build the C reference shared object
# ---------------------------------------------------------------------------
say "Building the C reference shared object"
mkdir -p "$CSRC/build"
(
  cd "$CSRC/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { fail "C build"; exit 1; }

C_SO="$(find "$CSRC/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$HERE/Cargo.toml"
)

declare -a COMBOS=("--no-default-features" "")   # "" == default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--all-features")
  n="${#FEATURES[@]}"
  # Full power set of the individual features (with --no-default-features).
  for ((m = 1; m < (1 << n); m++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( m & (1 << i) )); then sel="$sel,${FEATURES[i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
else
  # No [features] table: `--all-features` is identical to the default build,
  # but run it anyway so the matrix is explicit.
  COMBOS+=("--all-features")
fi

say "Feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  cargo test ${c:-<default features>}"; done

# ---------------------------------------------------------------------------
# 3. Build + symbol-diff + test every combination x profile
# ---------------------------------------------------------------------------
cd "$HERE"
C_SYMS="$(mktemp)"
R_SYMS="$(mktemp)"
trap 'rm -f "$C_SYMS" "$R_SYMS"' EXIT
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$C_SYMS"
echo "C exports $(wc -l < "$C_SYMS") symbols"

for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features='${combo:-default}' profile=$profile"
    say "$label"
    if [ "$profile" = release ]; then
      relflag="--release"
      outdir="target/release"
    else
      relflag=""
      outdir="target/debug"
    fi

    # shellcheck disable=SC2086
    if ! cargo build $CARGO_FLAGS $relflag $combo >/dev/null 2>&1; then
      fail "cargo build ($label)"
      continue
    fi

    nm -D --defined-only "$outdir/libaabb_lib.so" | awk '{print $3}' | sort -u > "$R_SYMS"
    missing="$(comm -23 "$C_SYMS" "$R_SYMS")"
    if [ -n "$missing" ]; then
      fail "symbols missing from the Rust .so ($label):"
      echo "$missing"
    else
      echo "symbol diff: EMPTY (all $(wc -l < "$C_SYMS") C symbols exported)"
    fi

    undef="$(nm -D -u "$outdir/libaabb_lib.so" \
      | awk '{print $2}' \
      | grep -v -E '@GLIBC|@GCC|^_ITM_|^__cxa_|^__gmon_start__$|^_Unwind_' || true)"
    if [ -n "$undef" ]; then
      fail "non-libc undefined symbols ($label):"
      echo "$undef"
    fi

    # shellcheck disable=SC2086
    log="$(mktemp)"
    if timeout 600 cargo test $CARGO_FLAGS $relflag $combo >"$log" 2>&1; then
      grep -E '^(     Running|test result:)' "$log" || true
    else
      fail "cargo test ($label)"
      tail -60 "$log"
    fi
    rm -f "$log"

    # Property-test soak: re-run every randomized row with fresh inputs.
    for s in $(seq 1 "${SOAK_SEEDS:-3}"); do
      log="$(mktemp)"
      # shellcheck disable=SC2086
      if C2_DIFF_SEED="$s" timeout 600 cargo test $CARGO_FLAGS $relflag $combo >"$log" 2>&1; then
        printf 'soak C2_DIFF_SEED=%s: %s\n' "$s" \
          "$(grep -cE '^test result: ok' "$log") suites ok"
      else
        fail "soak C2_DIFF_SEED=$s ($label)"
        tail -60 "$log"
      fi
      rm -f "$log"
    done
  done
done

say "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAILED"
