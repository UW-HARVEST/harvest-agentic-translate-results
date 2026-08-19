#!/usr/bin/env bash
# Phase D driver: builds the C reference library, enumerates every valid Cargo
# feature combination, and runs `cargo check` + the full differential suite for
# each one, in both the dev and release profiles. Also diffs the exported
# symbols of the two shared libraries.
#
# Usage: ./verify_all.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Building C reference library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/libdriver.so" || bad "C build"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations (power set of declared features)
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '
  /^\[features\]/ { inside = 1; next }
  /^\[/           { inside = 0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  no [features] declared -> the power set is a single empty combination"
  COMBOS=("")
else
  FEAT_ARR=($FEATURES)
  N=${#FEAT_ARR[@]}
  echo "  declared features: ${FEAT_ARR[*]}"
  COMBOS=()
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEAT_ARR[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
printf '  %d combination(s) to verify\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. check + build + test each combination, in both profiles
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty>}"
  for profile in dev release; do
    if [ "$profile" = release ]; then REL=(--release); else REL=(); fi
    step "combo=$label profile=$profile"

    timeout 600 cargo check --offline --no-default-features \
      --features "$combo" --all-targets "${REL[@]}" >/dev/null 2>&1 \
      && ok "cargo check" || bad "cargo check (combo=$label profile=$profile)"

    timeout 600 cargo build --offline --no-default-features \
      --features "$combo" "${REL[@]}" >/dev/null 2>&1 \
      && ok "cargo build" || bad "cargo build (combo=$label profile=$profile)"

    log="logs/test-${label//[^A-Za-z0-9]/_}-$profile.log"
    mkdir -p logs
    if timeout 600 cargo test --offline --no-default-features \
         --features "$combo" "${REL[@]}" >"$log" 2>&1; then
      ok "cargo test ($(grep -c '^test .* ok$' "$log") tests passed; see $log)"
    else
      bad "cargo test (combo=$label profile=$profile); see $log"
      tail -n 25 "$log"
    fi
  done
done

# Also verify the default invocation (default features, which here is the empty
# set) works without --no-default-features.
step "Default invocation"
timeout 600 cargo check --offline --all-targets >/dev/null 2>&1 \
  && ok "cargo check (default features)" || bad "cargo check (default features)"

# ---------------------------------------------------------------------------
# 4. Symbol parity
# ---------------------------------------------------------------------------
step "Symbol parity (nm -D)"
for profile in debug release; do
  rust_so="target/$profile/libdriver.so"
  [ -f "$rust_so" ] || { echo "  (skipping $profile: not built)"; continue; }
  missing=$(comm -23 \
    <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$rust_so"              | awk '{print $3}' | sort -u))
  if [ -z "$missing" ]; then
    ok "$profile: 0 C symbols missing from the Rust .so"
  else
    bad "$profile: missing symbols:"; echo "$missing" | sed 's/^/         /'
  fi
done

step "Unresolved symbols in the Rust .so"
if ldd -r target/debug/libdriver.so 2>&1 | grep -q "undefined symbol"; then
  bad "unresolved symbols:"; ldd -r target/debug/libdriver.so 2>&1 | grep "undefined symbol"
else
  ok "none (all imports resolve to libc/libgcc)"
fi

# ---------------------------------------------------------------------------
step "Summary"
if [ "$FAIL" -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  SOME CHECKS FAILED"
fi
exit "$FAIL"
