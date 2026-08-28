#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination and
# every build configuration that can change generated code.
#
# Feature combinations are enumerated MECHANICALLY from Cargo.toml via
# `cargo metadata`, not hardcoded, so a future [features] table is picked up
# automatically (the powerset is used when features exist).
set -u -o pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
cd "$HERE"

CARGO_OFFLINE="--offline"

# ---------------------------------------------------------------------------
# 0. Make sure the C reference library exists.
# ---------------------------------------------------------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "### building the C reference library"
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "FATAL: C build failed"; exit 1; }
fi
echo "C reference: $(ls "$ROOT"/c_src/build/lib*.so)"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  cargo metadata $CARGO_OFFLINE --no-deps --format-version 1 2>/dev/null \
  | tr ',' '\n' | grep -o '"features":{[^}]*}' -m1 \
  | grep -o '"[a-zA-Z0-9_-]*":\[' | tr -d '":[' || true
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares NO [features]; the only configurations are the"
  echo "default build and --no-default-features (which are identical here)."
  COMBOS+=("<default>" "--no-default-features" "--all-features")
else
  echo "declared features: ${FEATURES[*]}"
  COMBOS+=("<default>" "--no-default-features" "--all-features")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    if [ "${#sel[@]}" -gt 0 ]; then
      COMBOS+=("--no-default-features --features $(
        IFS=,; echo "${sel[*]}"
      )")
    else
      COMBOS+=("--no-default-features")
    fi
  done
fi

# ---------------------------------------------------------------------------
# 2. Build configurations that can change codegen / arithmetic behaviour.
#    name | cargo profile flags | RUSTFLAGS | extra env
#
# LTO is set through CARGO_PROFILE_* rather than RUSTFLAGS, because `-C lto`
# conflicts with the `-C embed-bitcode=no` cargo passes by default.
# ---------------------------------------------------------------------------
CONFIGS=(
  "dev                        |          |                                                   |"
  "dev+overflow-checks        |          |-C overflow-checks=on                              |"
  "dev+opt2                   |          |-C opt-level=2                                     |"
  "dev+debug-assertions-off   |          |-C debug-assertions=off -C overflow-checks=off     |"
  "release                    |--release |                                                   |"
  "release+overflow-checks    |--release |-C overflow-checks=on                              |"
  "release+opt3               |--release |-C opt-level=3                                     |"
  "release+lto-thin           |--release |                                                   |CARGO_PROFILE_RELEASE_LTO=thin"
  "release+lto-fat            |--release |                                                   |CARGO_PROFILE_RELEASE_LTO=fat"
  "release+opt-s              |--release |-C opt-level=s                                     |"
  "release+codegen-units-1    |--release |-C codegen-units=1                                 |"
)

total=0
failed=0
declare -a FAILURES=()

for combo in "${COMBOS[@]}"; do
  cflags="$combo"
  [ "$combo" = "<default>" ] && cflags=""
  for cfg in "${CONFIGS[@]}"; do
    IFS='|' read -r cname pflags rflags cenv <<<"$cfg"
    cname="$(echo "$cname" | xargs)"
    pflags="$(echo "$pflags" | xargs)"
    rflags="$(echo "$rflags" | xargs)"
    cenv="$(echo "${cenv:-}" | xargs)"
    total=$((total + 1))
    label="features[$combo] profile[$cname]"
    printf '### %-64s ' "$label"

    log="$(mktemp "${TMPDIR:-/tmp}/combo.XXXXXX.log")"
    # The cdylib must be (re)built with the same flags the tests will load.
    if ! env ${cenv:+$cenv} RUSTFLAGS="$rflags" cargo build $CARGO_OFFLINE \
         $pflags $cflags >"$log" 2>&1; then
      echo "BUILD FAILED"; failed=$((failed + 1)); FAILURES+=("$label (build)")
      tail -20 "$log"; rm -f "$log"; continue
    fi
    if ! env ${cenv:+$cenv} RUSTFLAGS="$rflags" timeout 600 cargo test \
         $CARGO_OFFLINE $pflags $cflags >>"$log" 2>&1; then
      echo "TEST FAILED"; failed=$((failed + 1)); FAILURES+=("$label (test)")
      grep -E 'FAILED|panicked at|DIVERGENCE|test result:' "$log" | head -30
      rm -f "$log"; continue
    fi
    # Summarise how many assertions actually ran.
    n_ok=$(grep -c '^test .* \.\.\. ok$' "$log" || true)
    echo "OK ($n_ok tests)"
    rm -f "$log"
  done
done

# ---------------------------------------------------------------------------
# 3. `cargo check` every combination too (catches cfg-gated compile errors).
# ---------------------------------------------------------------------------
echo
for combo in "${COMBOS[@]}"; do
  cflags="$combo"; [ "$combo" = "<default>" ] && cflags=""
  printf '### cargo check %-50s ' "features[$combo]"
  if cargo check $CARGO_OFFLINE --all-targets $cflags >/dev/null 2>&1; then
    echo "OK"
  else
    echo "FAILED"; failed=$((failed + 1)); FAILURES+=("check features[$combo]")
  fi
done

echo
echo "=============================================================="
echo "Phase D matrix: $total configuration(s) run, $failed failure(s)"
for f in "${FAILURES[@]:-}"; do [ -n "$f" ] && echo "  FAILED: $f"; done
echo "=============================================================="
[ "$failed" -eq 0 ]
