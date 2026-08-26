#!/usr/bin/env bash
# Phase D driver: build the C reference .so, then run `cargo check` + the full
# differential suite (Phases B and C) for EVERY valid feature combination, in
# both the dev and release profiles.
#
# Feature combinations are enumerated mechanically from the [features] section of
# Cargo.toml (its full power set), so adding a feature automatically widens the
# matrix. `Cargo.toml` currently declares no [features], so the matrix is the
# single empty combination.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` library, so this
# script always runs `cargo build` for the same profile/features first; the tests
# additionally refuse to run against a stale .so.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$PWD"
CARGO_FLAGS=(--offline)
FAILED=0

log() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 1. Build the C reference shared library
# --------------------------------------------------------------------------
log "Building the C reference shared library"
mkdir -p c_src/build || exit 1
(
  cd c_src/build || exit 1
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || { echo "FATAL: C build failed"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { echo "FATAL: $C_SO not produced"; exit 1; }
echo "C .so:      $C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate the power set of [features] from Cargo.toml
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (( (mask >> i) & 1 )); then
      combo="${combo:+$combo,}${FEATURES[i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "features declared: ${n} (${FEATURES[*]:-none})"
echo "feature combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------------------
# 3. check + build + test every combination, in both profiles
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  feat_args=(--no-default-features)
  [ -n "$combo" ] && feat_args+=(--features "$combo")

  log "cargo check  [$label]"
  if ! timeout 600 cargo check "${CARGO_FLAGS[@]}" "${feat_args[@]}" --all-targets 2>&1 | tail -5; then
    echo "FAIL: cargo check [$label]"; FAILED=1; continue
  fi

  for profile in dev release; do
    prof_args=()
    [ "$profile" = release ] && prof_args=(--release)

    log "cargo build  [$label] [$profile]"
    if ! timeout 600 cargo build "${CARGO_FLAGS[@]}" "${feat_args[@]}" "${prof_args[@]}" 2>&1 | tail -3; then
      echo "FAIL: cargo build [$label] [$profile]"; FAILED=1; continue
    fi

    # The differential tests capture fd 1, which is process-wide, so they must
    # run one at a time or libtest's own progress lines land inside a capture.
    # `.cargo/config.toml` sets this too, but cargo discovers that file relative
    # to the *current directory*, so pass it explicitly here as well.
    log "cargo test   [$label] [$profile]"
    out=$(timeout 600 env HARVEST_C_SO="$C_SO" RUST_TEST_THREADS=1 \
      cargo test "${CARGO_FLAGS[@]}" "${feat_args[@]}" "${prof_args[@]}" \
      -- --test-threads=1 2>&1)
    echo "$out" | grep -E "^(test result|error|warning: unused)" | head -20
    if echo "$out" | grep -qE "^test result: FAILED|^error"; then
      echo "FAIL: cargo test [$label] [$profile]"
      echo "$out" | grep -E "^---- |panicked at" | head -20
      FAILED=1
    else
      echo "PASS: [$label] [$profile]"
    fi
  done
done

# --------------------------------------------------------------------------
# 4. Symbol parity summary (Phase D)
# --------------------------------------------------------------------------
log "nm -D symbol parity"
for so in "$C_SO" "$ROOT/target/debug/libdriver.so" "$ROOT/target/release/libdriver.so"; do
  [ -f "$so" ] || continue
  printf '%s:\n' "$so"
  nm -D --defined-only "$so" | sed 's/^/    /'
done
echo
echo "C-defined symbols missing from the Rust .so:"
diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$ROOT/target/debug/libdriver.so" | awk '{print $NF}' | sort) \
     | grep '^<' || echo "    (none)"

log "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAILED"
