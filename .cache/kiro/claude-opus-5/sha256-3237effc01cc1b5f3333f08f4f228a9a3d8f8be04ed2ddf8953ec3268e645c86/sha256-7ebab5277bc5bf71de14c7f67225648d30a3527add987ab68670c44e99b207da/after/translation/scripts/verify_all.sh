#!/usr/bin/env bash
# Verify the Rust translation against the C build for every valid
# build-time configuration.
#
# Cargo.toml declares no [features] and c_src/CMakeLists.txt exposes no options
# or -D switches, so the set of valid feature combinations is exactly one: the
# empty set (identical to the default). The loop below still enumerates it
# programmatically so that adding a feature later is picked up automatically.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# --- 1. build the C shared library -----------------------------------------
step "building the C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1
) || { echo "C build FAILED (see /tmp/cmake.log)"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C  .so: $C_SO"

# --- 2. enumerate the feature combinations ---------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /=/   {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
  ' "$CRATE/Cargo.toml"
)
N=${#FEATURES[@]}
echo "features declared: $N ${FEATURES[*]-}"

COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
  done
  COMBOS+=("$combo")
done
echo "combinations to verify: ${#COMBOS[@]}"

symbol_diff() { # $1 = rust .so
  comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | grep -v '^_' | sort -u) \
    <(nm -D --defined-only "$1"    | awk '{print $NF}' | grep -v '^_' | sort -u)
}

# --- 3. check / build / test each combination ------------------------------
for combo in "${COMBOS[@]}"; do
  label=${combo:-'<none>'}
  step "combo: $label"

  ( cd "$CRATE" && timeout 600 cargo check --no-default-features --features "$combo" >/tmp/rcheck.log 2>&1 )
  if [ $? -ne 0 ]; then echo "cargo check FAILED for $label"; tail -20 /tmp/rcheck.log; FAIL=1; continue; fi
  echo "cargo check: ok"

  ( cd "$CRATE" && timeout 600 cargo build --no-default-features --features "$combo" >/tmp/rbuild-dbg.log 2>&1 )
  if [ $? -ne 0 ]; then echo "debug build FAILED for $label"; tail -20 /tmp/rbuild-dbg.log; FAIL=1; continue; fi

  ( cd "$CRATE" && timeout 600 cargo build --release --no-default-features --features "$combo" >/tmp/rbuild-rel.log 2>&1 )
  if [ $? -ne 0 ]; then echo "release build FAILED for $label"; tail -20 /tmp/rbuild-rel.log; FAIL=1; continue; fi

  for profile in debug release; do
    so="$CRATE/target/$profile/libhm_geti_lib.so"
    [ -f "$so" ] || { echo "no $profile .so, skipping"; FAIL=1; continue; }

    missing=$(symbol_diff "$so")
    if [ -n "$missing" ]; then
      echo "  symbols MISSING from the $profile Rust .so:"; echo "$missing"; FAIL=1
    else
      echo "  symbols ($profile): every C export present"
    fi

    ( cd "$CRATE" && RUST_SO="$so" timeout 600 cargo test --no-default-features --features "$combo" \
        >"/tmp/rtest-$profile.log" 2>&1 )
    rc=$?
    grep -E '^test result' "/tmp/rtest-$profile.log" | sed "s/^/  [$profile] /"
    if [ $rc -ne 0 ]; then
      echo "  cargo test FAILED ($profile .so, combo $label) -- see /tmp/rtest-$profile.log"
      grep -E 'FAILED|panicked|differ' "/tmp/rtest-$profile.log" | head -20
      FAIL=1
    fi
  done
done

step "result"
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASS"; else echo "FAILURES DETECTED"; fi
exit "$FAIL"
