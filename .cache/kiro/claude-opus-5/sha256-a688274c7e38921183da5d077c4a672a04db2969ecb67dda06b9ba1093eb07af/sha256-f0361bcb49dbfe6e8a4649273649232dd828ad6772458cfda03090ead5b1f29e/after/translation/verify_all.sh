#!/usr/bin/env bash
# Full verification sweep: build both libraries, then run every phase under
# every feature combination declared in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || exit 1
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO" || exit 1

# Enumerate feature combinations from Cargo.toml. With no [features] table the
# only combination is the default one.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)

COMBOS=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "features found: ${FEATURES[*]}"
  COMBOS+=("none")
  for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  # all features together
  COMBOS+=("$(IFS=,; echo "${FEATURES[*]}")")
else
  echo "no [features] table -> single configuration"
  COMBOS+=("none")   # --no-default-features, identical here but verified anyway
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) FLAGS=() ;;
    none)    FLAGS=(--no-default-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "############################################################"
  echo "### feature combination: $combo  (${FLAGS[*]:-<none>})"
  echo "############################################################"

  timeout 600 cargo build --release "${FLAGS[@]}" 2>&1 | tail -3 || { FAIL=1; continue; }

  RS_SO="target/release/libdriver.so"
  echo "--- nm -D symbol diff (C exported but missing from Rust) ---"
  diff <(nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only "$RS_SO" | awk '$2=="T"{print $3}' | sort) \
       | grep '^<' && { echo "SYMBOL GATE FAILED for $combo"; FAIL=1; } \
       || echo "symbol gate: no missing symbols"

  for t in phase_d_symbols phase_b_valid phase_c_errors; do
    echo "--- $t [$combo] ---"
    if ! timeout 600 cargo test --release "${FLAGS[@]}" --test "$t" -- --test-threads=1 2>&1 | tail -6; then
      echo "TEST FAILED: $t [$combo]"
      FAIL=1
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "=== ALL FEATURE COMBINATIONS PASSED ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit "$FAIL"
