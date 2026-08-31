#!/usr/bin/env bash
# Enumerates every feature combination declared in Cargo.toml and runs
# `cargo check` + `cargo test` for each. The C shared library is rebuilt first
# so the differential tests always compare against fresh ground truth.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

echo "=== building C ground-truth library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

# Feature names declared under [features], excluding "default".
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/[ \t]/,"");split($0,a,"=");print a[1]}' \
    "$CRATE/Cargo.toml" | grep -v '^default$'
)

# Power set of the declared features; the empty set is --no-default-features.
COMBOS=("")
for feat in "${FEATURES[@]}"; do
  for existing in "${COMBOS[@]}"; do
    if [[ -z "$existing" ]]; then COMBOS+=("$feat"); else COMBOS+=("$existing,$feat"); fi
  done
done
# Plus the crate's default feature set.
COMBOS+=("__default__")

echo "=== ${#COMBOS[@]} configuration(s) to verify ==="
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "__default__" ]]; then
    ARGS=()
    label="(default features)"
  elif [[ -z "$combo" ]]; then
    ARGS=(--no-default-features)
    label="(no features)"
  else
    ARGS=(--no-default-features --features "$combo")
    label="(features: $combo)"
  fi

  for profile in "" "--release"; do
    plabel=${profile:---dev}
    echo "--- cargo check $plabel $label"
    ( cd "$CRATE" && timeout 600 cargo check $profile "${ARGS[@]}" ) >/dev/null 2>&1 \
      || { echo "    CHECK FAILED $plabel $label"; FAIL=1; continue; }

    echo "--- cargo test  $plabel $label"
    ( cd "$CRATE" && timeout 600 cargo build $profile "${ARGS[@]}" >/dev/null 2>&1 \
        && timeout 600 cargo test $profile "${ARGS[@]}" 2>&1 | tail -n 4 ) \
      || { echo "    TEST FAILED $plabel $label"; FAIL=1; }
  done
done

echo "=== symbol comparison ==="
c_syms=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u)
for so in "$CRATE"/target/release/libdriver.so "$CRATE"/target/debug/libdriver.so; do
  [[ -f "$so" ]] || continue
  r_syms=$(nm -D --defined-only "$so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [[ -n "$missing" ]]; then
    echo "MISSING in $so:"; echo "$missing"; FAIL=1
  else
    echo "ok: $so exports every C symbol ($(echo "$c_syms" | tr '\n' ' '))"
  fi
done

if [[ $FAIL -eq 0 ]]; then echo "=== ALL CONFIGURATIONS PASS ==="; else echo "=== FAILURES PRESENT ==="; fi
exit $FAIL
