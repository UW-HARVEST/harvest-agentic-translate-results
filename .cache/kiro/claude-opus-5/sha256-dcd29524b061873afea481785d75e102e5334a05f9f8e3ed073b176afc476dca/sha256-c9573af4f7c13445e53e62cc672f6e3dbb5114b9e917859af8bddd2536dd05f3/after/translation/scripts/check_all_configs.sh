#!/usr/bin/env bash
# Phase D driver: enumerate every cargo feature combination from Cargo.toml and
# run cargo check + the full differential suite for each, then repeat the suite
# against every build profile's .so (different optimisation levels are different
# code paths for this crate, since bit-exactness depends on codegen).
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
TIMEOUT=${TIMEOUT:-600}
fail=0

echo "=== C library ==="
(
  cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO=$(find ../c_src/build -name '*.so' | head -1)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /=/      {gsub(/[ \t]/,""); split($0,a,"="); if (a[1]!="default") print a[1]}
  ' Cargo.toml
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo
  echo "=== Feature combinations ==="
  echo "Cargo.toml declares no [features] table -> exactly one configuration."
  COMBOS=("__default__")
else
  echo "declared features: ${FEATURES[*]}"
  COMBOS=("__default__" "__none__")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

run() { echo "  \$ $*"; timeout "$TIMEOUT" "$@" >/tmp/gjk_cfg.log 2>&1; }

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) args=() ;             label="default features" ;;
    __none__)    args=(--no-default-features); label="no default features" ;;
    *)           args=(--no-default-features --features "$combo"); label="features=$combo" ;;
  esac
  echo
  echo "=== $label ==="
  if run cargo check "${args[@]}"; then echo "  check: OK"; else
    echo "  check: FAILED"; tail -20 /tmp/gjk_cfg.log; fail=1; continue
  fi

  for profile in release debug; do
    if [ "$profile" = release ]; then pargs=(--release); else pargs=(); fi
    if ! run cargo build "${pargs[@]}" "${args[@]}"; then
      echo "  build/$profile: FAILED"; tail -20 /tmp/gjk_cfg.log; fail=1; continue
    fi
    so="target/$profile/libgjk_lib.so"
    if [ ! -f "$so" ]; then echo "  build/$profile: no .so produced"; fail=1; continue; fi

    # symbol parity for this artifact
    nm -D --defined-only "$C_SO"  | awk '$2=="T"{print $3}' | sort > /tmp/gjk_c.txt
    nm -D --defined-only "$so"    | awk '$2=="T"{print $3}' | sort > /tmp/gjk_r.txt
    miss=$(comm -23 /tmp/gjk_c.txt /tmp/gjk_r.txt)
    if [ -n "$miss" ]; then
      echo "  symbols/$profile: MISSING -> $miss"; fail=1
    else
      echo "  symbols/$profile: OK ($(wc -l < /tmp/gjk_c.txt) symbols, 0 missing)"
    fi

    # full differential suite against this artifact
    if GJK_RUST_SO="$PWD/$so" timeout "$TIMEOUT" cargo test --release "${args[@]}" \
        >/tmp/gjk_test.log 2>&1; then
      echo "  tests/$profile: OK ($(grep -c '\.\.\. ok' /tmp/gjk_test.log) tests)"
    else
      echo "  tests/$profile: FAILED"
      grep -E 'panicked|FAILED|test result' /tmp/gjk_test.log | head -20
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"
