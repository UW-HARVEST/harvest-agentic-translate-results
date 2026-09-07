#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff the exported symbol tables, then
# run every phase under every feature combination declared in Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
fail=0

echo "=== build C ==="
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) \
  || { echo "C build FAILED"; exit 1; }
CSO="$(find "$CBUILD" -maxdepth 1 -name '*.so' | head -1)"
echo "C  .so: $CSO"

echo "=== enumerate feature combinations ==="
# Every combination of the declared features, plus the default and the
# no-default-features build. No [features] section => just the two baselines.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}' \
    "$CRATE/Cargo.toml"
)
COMBOS=("<default>" "<no-default-features>")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((k=0; k<n; k++)); do
      if (( mask & (1<<k) )); then combo="${combo:+$combo,}${FEATURES[$k]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'features: %s\n' "${FEATURES[*]:-<none declared>}"
printf 'combos:   %s\n' "${COMBOS[*]}"

for combo in "${COMBOS[@]}"; do
  echo
  echo "############ COMBINATION: $combo ############"
  case "$combo" in
    "<default>")             FLAGS=() ;;
    "<no-default-features>") FLAGS=(--no-default-features) ;;
    *)                       FLAGS=(--no-default-features --features "$combo") ;;
  esac

  ( cd "$CRATE" && timeout 600 cargo build --release "${FLAGS[@]}" >/dev/null 2>&1 ) \
    || { echo "  cargo build FAILED"; fail=1; continue; }

  RSO="$CRATE/target/release/libcapsule_lib.so"
  nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
  nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
  missing="$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)"
  extra="$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)"
  echo "  symbols: C=$(wc -l < /tmp/pd_c.txt) Rust=$(wc -l < /tmp/pd_r.txt)"
  if [ -n "$missing" ]; then
    echo "  MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  missing from Rust: none"
  fi
  if [ -n "$extra" ]; then
    echo "  extra in Rust (not exported by C):"; echo "$extra" | sed 's/^/    /'
  fi
  # Undefined references must all be libc / libgcc-unwind.
  undef="$(nm -D --undefined-only "$RSO" | awk '{print $NF}' \
            | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^statx$|^gettid$' || true)"
  if [ -n "$undef" ]; then
    echo "  NON-LIBC UNDEFINED SYMBOLS:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc symbols: none"
  fi

  ( cd "$CRATE" && timeout 600 cargo test --release "${FLAGS[@]}" 2>&1 \
      | grep -E 'Running tests|test result|^test .* FAILED|DIVERGENCE' ) | sed 's/^/  /'
  ( cd "$CRATE" && timeout 600 cargo test --release "${FLAGS[@]}" >/dev/null 2>&1 ) \
    || { echo "  TESTS FAILED for $combo"; fail=1; }
done

echo
if [ "$fail" -eq 0 ]; then echo "PHASE D: ALL COMBINATIONS PASS"; else echo "PHASE D: FAILURES PRESENT"; fi
exit "$fail"
