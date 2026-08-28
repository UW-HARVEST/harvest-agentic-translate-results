#!/usr/bin/env bash
# Build the C reference .so, then build + test the Rust cdylib against it for
# every valid Cargo feature combination and both profiles.
#
#   ./verify.sh            # everything
#   ./verify.sh --check    # cargo check for every feature combination only
#
# Note: `cargo test` does NOT build a cdylib artifact, so every test run must be
# preceded by a `cargo build` for the same profile. The test harness asserts the
# .so is not stale, so a missing build fails loudly rather than silently testing
# old code.

set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
LOG_DIR="${TMPDIR:-/tmp}/c2-verify"
mkdir -p "$LOG_DIR"

TIMEOUT=600
rc=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s (log: %s)\n' "$1" "$2"; rc=1; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
# Reads the [features] table; every subset of the non-`default` features is a
# candidate configuration. With no [features] table there is exactly one
# configuration (the empty set), which is the case for this crate.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=")
      gsub(/[[:space:]]/, "", kv[1])
      if (kv[1] != "default") print kv[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((bit = 0; bit < n; bit++)); do
    if (((mask >> bit) & 1)); then
      combo="${combo:+$combo,}${FEATURES[bit]}"
    fi
  done
  COMBOS+=("$combo")
done

step "feature combinations (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do
  printf '  --no-default-features --features "%s"\n' "$c"
done

# ---------------------------------------------------------------------------
# cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check"
for c in "${COMBOS[@]}"; do
  slug="${c//,/_}"; slug="${slug:-none}"
  log="$LOG_DIR/check-$slug.log"
  if timeout "$TIMEOUT" cargo check --manifest-path "$CRATE_DIR/Cargo.toml" \
      --all-targets --no-default-features --features "$c" >"$log" 2>&1; then
    echo "  ok      features=[$c]"
  else
    fail "cargo check features=[$c]" "$log"
    tail -n 20 "$log"
  fi
done

[[ "${1:-}" == "--check" ]] && exit $rc

# ---------------------------------------------------------------------------
# C reference library
# ---------------------------------------------------------------------------
step "build C reference"
log="$LOG_DIR/cmake.log"
if (cd "$ROOT/c_src" && mkdir -p build && cd build \
      && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && timeout "$TIMEOUT" cmake --build .) >"$log" 2>&1; then
  echo "  ok      $(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' -printf '%f')"
else
  fail "C build" "$log"
  tail -n 20 "$log"
  exit $rc
fi

# ---------------------------------------------------------------------------
# Build + test each combination, in both profiles (codegen differs, and NaN
# payload propagation is codegen-sensitive, so both must be checked).
# ---------------------------------------------------------------------------
for profile in debug release; do
  relflag=""
  [[ $profile == release ]] && relflag="--release"
  for c in "${COMBOS[@]}"; do
    slug="${c//,/_}"; slug="${slug:-none}"
    step "$profile / features=[$c]"

    log="$LOG_DIR/build-$profile-$slug.log"
    if ! timeout "$TIMEOUT" cargo build --manifest-path "$CRATE_DIR/Cargo.toml" \
        $relflag --no-default-features --features "$c" >"$log" 2>&1; then
      fail "cargo build $profile features=[$c]" "$log"
      tail -n 20 "$log"
      continue
    fi

    log="$LOG_DIR/test-$profile-$slug.log"
    if timeout "$TIMEOUT" cargo test --manifest-path "$CRATE_DIR/Cargo.toml" \
        $relflag --no-default-features --features "$c" >"$log" 2>&1; then
      grep -E '^test result' "$log" | sed 's/^/  /'
    else
      fail "cargo test $profile features=[$c]" "$log"
      grep -E 'FAILED|mismatch|panicked|^error' "$log" | head -n 40
    fi
  done
done

step "symbol parity"
c_so="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -n1)"
for profile in debug release; do
  rs_so="$CRATE_DIR/target/$profile/libcircle_collide_lib.so"
  [[ -f $rs_so ]] || continue
  missing="$(comm -23 \
    <(nm -D --defined-only --format=posix "$c_so"  | awk '{print $1}' | sort -u) \
    <(nm -D --defined-only --format=posix "$rs_so" | awk '{print $1}' | sort -u))"
  if [[ -z $missing ]]; then
    echo "  ok      $profile: Rust .so exports every C symbol"
  else
    fail "missing symbols in $profile: $(tr '\n' ' ' <<<"$missing")" "-"
  fi
done

step "result"
if ((rc == 0)); then echo "ALL CONFIGURATIONS MATCH"; else echo "FAILURES PRESENT"; fi
exit $rc
