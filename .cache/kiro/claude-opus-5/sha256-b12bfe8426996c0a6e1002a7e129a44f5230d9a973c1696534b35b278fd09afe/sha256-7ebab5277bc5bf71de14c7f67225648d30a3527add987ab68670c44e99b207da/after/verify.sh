#!/usr/bin/env bash
# Full verification sweep: enumerate every Cargo feature combination, type-check
# each, then build both libraries, diff their exported symbols and run the
# differential test suite in both the dev and release profiles.
#
# Usage: ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOGDIR="/tmp/tfm-verify"
mkdir -p "$LOGDIR"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
note "Feature enumeration"
FEATURES=$(awk '
  /^\[features\]/ { inside=1; next }
  /^\[/           { inside=0 }
  inside && /=/   { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default") print a[1] }
' "$CRATE/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features]; the crate has a single configuration."
  COMBOS=("")
else
  # Power set of the declared features.
  readarray -t FEATURE_ARR <<< "$FEATURES"
  n=${#FEATURE_ARR[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then
        combo="${combo:+$combo,}${FEATURE_ARR[b]}"
      fi
    done
    COMBOS+=("$combo")
  done
  printf '  declared: %s\n' "$(echo "$FEATURES" | tr '\n' ' ')"
  printf '  %d combination(s)\n' "${#COMBOS[@]}"
fi

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
note "cargo check per feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  log="$LOGDIR/check-${combo//,/_}.log"
  if timeout 600 cargo check --manifest-path "$CRATE/Cargo.toml" \
       --all-targets --no-default-features --features "$combo" > "$log" 2>&1; then
    ok "cargo check --features $label"
  else
    bad "cargo check --features $label (see $log)"
    tail -n 20 "$log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Build the C shared library (default CMake configuration)
# ---------------------------------------------------------------------------
note "Build C shared library"
if (cd "$ROOT/c_src" && mkdir -p build && cd build \
      && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && timeout 600 cmake --build .) > "$LOGDIR/cmake.log" 2>&1; then
  C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -n1)
  ok "C library: $C_SO"
else
  bad "C build failed (see $LOGDIR/cmake.log)"
  tail -n 20 "$LOGDIR/cmake.log"
  exit 1
fi

# ---------------------------------------------------------------------------
# 4-6. Per combination, per profile: build, diff symbols, run differential tests
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label="features=${combo:-<none>} profile=$profile"
    tag="${combo//,/_}-$profile"
    relflag=()
    outdir="$CRATE/target/debug"
    if [ "$profile" = release ]; then
      relflag=(--release)
      outdir="$CRATE/target/release"
    fi

    note "$label"

    # Build the cdylib (cargo test does not emit it on its own).
    if timeout 600 cargo build --manifest-path "$CRATE/Cargo.toml" "${relflag[@]}" \
         --no-default-features --features "$combo" > "$LOGDIR/build-$tag.log" 2>&1; then
      ok "cargo build"
    else
      bad "cargo build (see $LOGDIR/build-$tag.log)"
      tail -n 20 "$LOGDIR/build-$tag.log"
      continue
    fi

    RUST_SO="$outdir/libtfm_lib.so"

    # Exported-symbol comparison: every symbol the C .so exports must also be
    # exported by the Rust .so under the identical name.
    c_syms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ { print $3 }' | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '$2 ~ /^[A-Z]$/ { print $3 }' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -z "$missing" ]; then
      ok "exports: all $(echo "$c_syms" | grep -c .) C symbol(s) present in Rust .so"
    else
      bad "exports missing from Rust .so:"
      echo "$missing" | sed 's/^/         /'
    fi

    # Differential tests through the FFI boundary.
    if RUST_TFM_SO="$RUST_SO" timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" \
         "${relflag[@]}" --no-default-features --features "$combo" \
         > "$LOGDIR/test-$tag.log" 2>&1; then
      ok "cargo test: $(grep -h '^test result' "$LOGDIR/test-$tag.log" | tr '\n' ' ')"
    else
      bad "cargo test (see $LOGDIR/test-$tag.log)"
      grep -A 8 '^failures:\|panicked at' "$LOGDIR/test-$tag.log" | head -n 40
    fi
  done
done

note "Summary"
if [ "$fail" -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  FAILURES PRESENT"
fi
exit "$fail"
