#!/usr/bin/env bash
# Phase D driver: build both libraries, diff their exported symbols, and run the
# full differential suite under every feature combination.
#
# Usage: ./run_all.sh          (from the crate root)
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
RUST_SO="$HERE/target/release/libdriver.so"
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || fail "C build"
[ -f "$C_SO" ] || fail "missing $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# ---------------------------------------------------------------------------
# Every combination of the crate's declared features, plus the two baselines.
# The crate declares no [features], so this yields exactly: default and
# --no-default-features. The loop is written generically so it stays correct if
# features are ever added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/[ \t]*=.*/,"");gsub(/[ \t]/,"");if($0!="default"&&$0!="")print}' Cargo.toml
)
COMBOS=("--" "--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("--no-default-features --features $combo")
    COMBOS+=("--features $combo")
  done
fi
printf 'declared features: %s\n' "${FEATURES[*]:-<none>}"
printf 'combinations to verify: %s\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '  cargo test %s\n' "$c"; done

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  ARGS=$([ "$combo" = "--" ] && echo "" || echo "$combo")

  step "cargo check  [$combo]"
  # shellcheck disable=SC2086
  timeout 600 cargo check --all-targets $ARGS 2>&1 | tail -3 \
    || fail "cargo check [$combo]"

  step "Build Rust cdylib (release)  [$combo]"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $ARGS 2>&1 | tail -2 \
    || fail "cargo build [$combo]"
  [ -f "$RUST_SO" ] || fail "missing $RUST_SO [$combo]"

  step "Symbol parity  [$combo]"
  nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u > /tmp/sym_c.txt
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > /tmp/sym_rust.txt
  printf 'C exports   : %s symbol(s)\n' "$(wc -l < /tmp/sym_c.txt)"
  printf 'Rust exports: %s symbol(s)\n' "$(wc -l < /tmp/sym_rust.txt)"
  MISSING="$(comm -23 /tmp/sym_c.txt /tmp/sym_rust.txt)"
  if [ -n "$MISSING" ]; then
    printf 'missing from the Rust .so:\n%s\n' "$MISSING"
    fail "symbol diff not empty [$combo]"
  else
    echo "missing from the Rust .so: (none)"
  fi

  # Non-libc undefined symbols in the Rust .so. Everything the Rust runtime
  # imports is provided by glibc or libgcc; anything else would be a genuine
  # unresolved reference.
  UNDEF="$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
    | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^gettid$|^statx$' || true)"
  if [ -n "$UNDEF" ]; then
    printf 'unresolved non-libc symbols:\n%s\n' "$UNDEF"
    fail "undefined non-libc symbols [$combo]"
  else
    echo "unresolved non-libc symbols: (none)"
  fi

  step "Phase B + Phase C differential suite  [$combo]"
  # --test-threads=1 because the suite redirects the process-wide fd 1.
  # shellcheck disable=SC2086
  C_DRIVER_SO="$C_SO" RUST_DRIVER_SO="$RUST_SO" \
    timeout 600 cargo test --release $ARGS -- --test-threads=1 2>&1 \
    | grep -E '\.\.\. FAILED$|^test result|^error' || fail "cargo test [$combo]"
  # shellcheck disable=SC2086
  C_DRIVER_SO="$C_SO" RUST_DRIVER_SO="$RUST_SO" \
    timeout 600 cargo test --release $ARGS -- --test-threads=1 >/dev/null 2>&1 \
    || fail "differential suite [$combo]"
done

step "Result"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAILED"
