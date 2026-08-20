#!/usr/bin/env bash
# Phase A + D driver: enumerate every build-time configuration, check it, build
# both shared objects, diff their exported symbols, and run the whole
# differential test suite (Phase B + Phase C) in each configuration.
#
# Usage: ./run_all_configs.sh [extra cargo test args...]
set -uo pipefail

cd "$(dirname "$0")" || exit 1
LOG_DIR="${TMPDIR:-/tmp}/driver_verify_logs"
mkdir -p "$LOG_DIR"
fail=0

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
#    (There is no [features] table, so the only combination is the empty one;
#    the loop is written generically so it keeps working if features appear.)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=("")            # the empty (= default = --no-default-features) combination
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared object once (the ground truth).
# ---------------------------------------------------------------------------
echo
echo "=== building the C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1
if [[ $? -ne 0 ]]; then
  echo "FAIL: C build failed, see $LOG_DIR/cmake.log"; tail -20 "$LOG_DIR/cmake.log"; exit 1
fi
C_SO=c_src/build/libdriver.so
echo "built $C_SO"

c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | grep -v '^_' | sort -u)

# ---------------------------------------------------------------------------
# 3. For every combination × cargo profile: check, build, diff symbols, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    if [[ $profile == release ]]; then prof_flag=(--release); prof_dir=release
    else prof_flag=(); prof_dir=debug; fi

    if [[ -n "$combo" ]]; then feat=(--no-default-features --features "$combo")
    else feat=(--no-default-features); fi

    tag="${combo:-none}-$profile"
    tag=${tag//,/+}
    echo
    echo "=== configuration: features='${combo:-<none>}' profile=$profile ==="

    echo "--- cargo check ---"
    if ! timeout 600 cargo check --offline "${feat[@]}" "${prof_flag[@]}" \
         > "$LOG_DIR/check-$tag.log" 2>&1; then
      echo "FAIL: cargo check failed, see $LOG_DIR/check-$tag.log"
      tail -30 "$LOG_DIR/check-$tag.log"; fail=1; continue
    fi
    echo "ok"

    echo "--- cargo build (cdylib) ---"
    if ! timeout 600 cargo build --offline "${feat[@]}" "${prof_flag[@]}" \
         > "$LOG_DIR/build-$tag.log" 2>&1; then
      echo "FAIL: cargo build failed, see $LOG_DIR/build-$tag.log"
      tail -30 "$LOG_DIR/build-$tag.log"; fail=1; continue
    fi
    echo "ok"

    echo "--- symbol diff (C .so vs Rust .so) ---"
    r_so="target/$prof_dir/libdriver.so"
    r_syms=$(nm -D --defined-only "$r_so" | awk '{print $NF}' | grep -v '^_' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -n "$missing" ]]; then
      echo "FAIL: symbols exported by the C .so but missing from $r_so:"
      echo "$missing"; fail=1
    else
      echo "ok — 0 missing symbols ($(echo "$c_syms" | wc -l) checked)"
    fi
    echo "--- undefined non-libc symbols in $r_so ---"
    undef=$(nm -D -u "$r_so" | awk '{print $NF}' | grep -v '^_' \
              | grep -v '@GLIBC' | grep -v '@GCC' | sort -u)
    if [[ -n "$undef" ]]; then
      echo "NOTE: undefined symbols (must all be libc): $undef"
    else
      echo "ok — none beyond libc/runtime"
    fi

    echo "--- cargo test (Phase B + Phase C + symbol parity) ---"
    if ! timeout 600 cargo test --offline "${feat[@]}" "${prof_flag[@]}" "$@" \
         > "$LOG_DIR/test-$tag.log" 2>&1; then
      echo "FAIL: cargo test failed, see $LOG_DIR/test-$tag.log"
      grep -E "^(test result|failures:|test .* FAILED)" "$LOG_DIR/test-$tag.log" | head -40
      fail=1; continue
    fi
    grep -E "^test result" "$LOG_DIR/test-$tag.log"
  done
done

echo
if (( fail )); then
  echo "RESULT: FAILURES (see $LOG_DIR)"
  exit 1
fi
echo "RESULT: all configurations verified (logs in $LOG_DIR)"
