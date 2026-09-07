#!/usr/bin/env bash
# Phase D driver: symbol parity + full suite under every feature combination.
#
#   ./scripts/verify_all.sh
#
# Enumerates the [features] table from Cargo.toml, builds the powerset of
# non-default features (plus the default, --no-default-features and
# --all-features configurations), and for each one:
#   * builds the release cdylib,
#   * diffs `nm -D` against the C .so and requires an EMPTY diff,
#   * runs the whole differential suite.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

C_BUILD="$ROOT/../c_src/build"

echo "== building the C shared library =="
( cd "$ROOT/../c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | head -1)"
[ -n "$C_SO" ] || { echo "no C .so found in $C_BUILD"; exit 1; }
echo "C  .so: $C_SO"

# ---- enumerate feature combinations -----------------------------------------
# Feature names are the `key = [...]` lines inside the [features] table.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

declare -a COMBOS=()
COMBOS+=("--")                          # default features
COMBOS+=("--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--all-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((k = 0; k < n; k++)); do
      (( (mask >> k) & 1 )) && sel+=("${FEATURES[$k]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
  echo "features declared: ${FEATURES[*]}"
else
  echo "features declared: (none — Cargo.toml has no [features] table)"
fi
echo "combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  args=()
  [ "$combo" != "--" ] && read -r -a args <<< "$combo"
  if [ "$combo" = "--" ]; then label="default"; else label="$combo"; fi
  echo
  echo "=========================================================="
  echo "== combo: ${label} =="
  echo "=========================================================="

  if ! timeout 600 cargo build --release "${args[@]}" >/tmp/vb.log 2>&1; then
    echo "  BUILD FAILED"; tail -20 /tmp/vb.log; fail=1; continue
  fi
  RUST_SO="$ROOT/target/release/libflip_horizontal_lib.so"

  # ---- symbol parity ----
  cdefs="$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)"
  rdefs="$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)"
  missing="$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))"
  if [ -n "$missing" ]; then
    echo "  SYMBOL DIFF NOT EMPTY — missing from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  symbols: OK ($(echo "$cdefs" | wc -l) C symbol(s), 0 missing from Rust)"
  fi

  # Undefined references in the Rust .so must all be toolchain/libc imports.
  # Anything else would mean the translation calls something it never defines.
  undef="$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
            | grep -vE '@GLIBC_|@GCC_' \
            | grep -vE '^(_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__gmon_start__|__tls_get_addr)$' \
            | sort -u)"
  if [ -n "$undef" ]; then
    echo "  undefined non-toolchain symbols in Rust .so:"
    echo "$undef" | sed 's/^/    /'
    fail=1
  else
    echo "  undefined refs: OK (all resolve to glibc / libgcc-unwind / weak toolchain hooks)"
  fi

  # ---- differential suite ----
  if timeout 600 cargo test "${args[@]}" >/tmp/vt.log 2>&1; then
    grep -E '^test result' /tmp/vt.log | sed 's/^/  /'
  else
    echo "  TESTS FAILED"
    grep -E '^test .* FAILED|^test result|panicked|signal:' /tmp/vt.log | head -30 | sed 's/^/    /'
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
