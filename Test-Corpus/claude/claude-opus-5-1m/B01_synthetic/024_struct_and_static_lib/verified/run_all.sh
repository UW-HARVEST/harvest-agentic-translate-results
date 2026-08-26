#!/usr/bin/env bash
# Full verification sweep: builds the C reference .so, enumerates every valid
# Cargo feature combination (the power set of [features]), and for each one runs
# `cargo check`, rebuilds the cdylib, diffs `nm -D` against the C .so and runs
# the differential test suite.
#
# NOTE: `cargo build` before `cargo test` is mandatory -- `cargo test` does NOT
# rebuild `crate-type = ["cdylib"]` artifacts, so the tests would otherwise load
# a stale .so. tests/differential.rs also guards against this itself.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
LOG_DIR="${TMPDIR:-/tmp}/driver_verify"
mkdir -p "$LOG_DIR"
FAILED=0

say() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- C reference
say "building C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1 \
  || { fail "C build (see $LOG_DIR/cmake.log)"; exit 1; }
echo "ok: c_src/build/libdriver.so"

# ------------------------------------------------------- feature enumeration
# All features declared in Cargo.toml's [features] section.
mapfile -t FEATURES < <(awk '
  /^\[features\]/       { in_f = 1; next }
  /^\[/                 { in_f = 0 }
  in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
  }' Cargo.toml)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS=("")   # no [features] section -> exactly one valid combination
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
say "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-(none)}'"; done

# ------------------------------------------------------------- verify a combo
verify() {                       # verify <label> <extra cargo args...>
  local label="$1"; shift
  local tag="${label//[^A-Za-z0-9]/_}"

  say "[$label] cargo check"
  timeout 600 cargo check --offline --all-targets "$@" \
      > "$LOG_DIR/check_$tag.log" 2>&1 || { fail "[$label] cargo check"; return; }

  say "[$label] cargo build (cdylib)"
  timeout 600 cargo build --offline "$@" \
      > "$LOG_DIR/build_$tag.log" 2>&1 || { fail "[$label] cargo build"; return; }

  local profile_dir=target/debug
  [[ " $* " == *" --release "* ]] && profile_dir=target/release

  say "[$label] nm -D symbol parity"
  local c_syms rust_syms missing
  c_syms=$(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort -u)
  rust_syms=$(nm -D --defined-only "$profile_dir/libdriver.so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -n "$missing" ]; then
    fail "[$label] symbols missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  else
    echo "ok: Rust .so exports all $(echo "$c_syms" | wc -l) C symbols: $(echo "$c_syms" | tr '\n' ' ')"
  fi

  say "[$label] cargo test (differential)"
  # The differential target has harness = false and runs its cases sequentially,
  # so no --test-threads plumbing is needed.
  if timeout 600 cargo test --offline "$@" \
        > "$LOG_DIR/test_$tag.log" 2>&1; then
    grep -E 'test result:' "$LOG_DIR/test_$tag.log"
  else
    fail "[$label] cargo test (see $LOG_DIR/test_$tag.log)"
    grep -E 'test result:|FAILED|divergence|panicked' "$LOG_DIR/test_$tag.log" | head -30
  fi
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    verify "no-default-features"  --no-default-features
    verify "default-features"
  else
    verify "features=$combo" --no-default-features --features "$combo"
  fi
done

# The release profile is a distinct build configuration (panic = "abort").
verify "release/no-default-features" --release --no-default-features
verify "release/default-features" --release

say "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "THERE WERE FAILURES (logs in $LOG_DIR)"
fi
exit "$FAILED"
