#!/usr/bin/env bash
# Full verification driver: builds both shared objects, checks symbol parity
# with nm, and runs every test target under EVERY feature combination declared
# in Cargo.toml (the powerset of the optional features, plus the
# --no-default-features baseline).
#
# Usage: translation/scripts/verify.sh
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls -t "$root"/c_src/build/lib*.so | head -1)"
echo "C  .so: $c_so"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# ---------------------------------------------------------------------------
# Feature names in the [features] section, excluding "default".
features=$(awk '
  /^\[features\]/ { inside=1; next }
  /^\[/           { inside=0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' "$crate/Cargo.toml")

combos=("--no-default-features" "")   # baseline + default
if [[ -n "$features" ]]; then
  mapfile -t flist <<<"$features"
  n=${#flist[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    set=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && set+="${flist[$i]},"
    done
    combos+=("--no-default-features --features ${set%,}")
  done
  combos+=("--all-features")
else
  echo "Cargo.toml declares no [features]; the default build is the only configuration."
fi
for combo in "${combos[@]}"; do
  echo "combination: ${combo:-<default features>}"
done

# ---------------------------------------------------------------------------
for combo in "${combos[@]}"; do
  label="${combo:-<default features>}"
  step "cargo check   ($label)"
  ( cd "$crate" && timeout 600 cargo check $combo --all-targets ) \
    || { echo "cargo check FAILED for $label"; fail=1; continue; }

  step "cargo build --release   ($label)"
  ( cd "$crate" && timeout 600 cargo build --release $combo ) \
    || { echo "release build FAILED for $label"; fail=1; continue; }

  rust_so="$crate/target/release/libdoubleneg_lib.so"
  step "nm -D symbol diff   ($label)"
  nm -D --defined-only "$c_so"   | awk '{print $3}' | sort -u > /tmp/verify_c.syms
  nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u > /tmp/verify_r.syms
  missing=$(comm -23 /tmp/verify_c.syms /tmp/verify_r.syms)
  if [[ -n "$missing" ]]; then
    echo "MISSING from the Rust .so:"; echo "$missing"; fail=1
  else
    echo "0 missing symbols ($(wc -l < /tmp/verify_c.syms) exported by C, \
$(wc -l < /tmp/verify_r.syms) by Rust)"
  fi

  step "cargo test   ($label)"
  ( cd "$crate" && timeout 600 cargo test $combo 2>&1 | grep -E \
      '^(test result|running|error|test .* FAILED|failures:|warning: unused)' ) \
    || true
  # Re-run capturing the exit status without the grep pipeline swallowing it.
  ( cd "$crate" && timeout 600 cargo test $combo >/tmp/verify_test.log 2>&1 ) \
    || { echo "TESTS FAILED for $label (see /tmp/verify_test.log)"; fail=1; }
done

step "Summary"
if (( fail )); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "VERIFICATION PASSED for all ${#combos[@]} feature combination(s)"
