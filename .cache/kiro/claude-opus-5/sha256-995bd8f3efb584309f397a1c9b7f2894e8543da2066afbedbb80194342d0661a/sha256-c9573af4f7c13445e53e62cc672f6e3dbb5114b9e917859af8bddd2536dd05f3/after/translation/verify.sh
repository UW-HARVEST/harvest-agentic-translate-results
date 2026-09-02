#!/usr/bin/env bash
# Phase D driver: build both libraries, prove exported-symbol parity, and run
# the whole differential suite under EVERY feature combination and both build
# profiles. Anything non-zero in the summary is a verification failure.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
step "build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations straight out of Cargo.toml (power set).
step "enumerate feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)
echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=("--all-features" "")            # default build + all features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  for ((mask=1; mask < (1<<n); mask++)); do
    sel=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then sel="${sel:+$sel,}${FEATURES[b]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  step "cargo check   [$label]"
  ( cd "$CRATE" && timeout 600 cargo check $combo 2>&1 | tail -3 ) || FAIL=1

  for profile in release debug; do
    flag=""; [ "$profile" = release ] && flag="--release"
    step "build + symbol parity + full suite   [$label] [$profile]"
    ( cd "$CRATE" && timeout 600 cargo build $flag $combo 2>&1 | tail -2 ) || { FAIL=1; continue; }
    R_SO="$CRATE/target/$profile/libomni_manifold_lib.so"

    nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort > /tmp/c_syms.txt
    nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort > /tmp/r_syms.txt
    missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
    extra="$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)"
    echo "symbols: C=$(wc -l < /tmp/c_syms.txt) Rust=$(wc -l < /tmp/r_syms.txt)"
    if [ -n "$missing" ]; then echo "MISSING FROM RUST:"; echo "$missing"; FAIL=1; fi
    if [ -n "$extra" ];   then echo "EXTRA IN RUST:";    echo "$extra";   FAIL=1; fi

    # Undefined symbols in the Rust .so must all be libc / libgcc-unwind.
    nonlibc="$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
      | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
      | grep -vE '@GLIBC' | grep -vE '^(_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable)$' || true)"
    if [ -n "$nonlibc" ]; then echo "NON-LIBC UNDEFINED:"; echo "$nonlibc"; FAIL=1; fi

    ( cd "$CRATE" && RUST_SO="$R_SO" timeout 600 cargo test --release $combo 2>&1 \
        | grep -E 'test result|FAILED|panicked' ) || FAIL=1
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL PHASE A-D CHECKS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
