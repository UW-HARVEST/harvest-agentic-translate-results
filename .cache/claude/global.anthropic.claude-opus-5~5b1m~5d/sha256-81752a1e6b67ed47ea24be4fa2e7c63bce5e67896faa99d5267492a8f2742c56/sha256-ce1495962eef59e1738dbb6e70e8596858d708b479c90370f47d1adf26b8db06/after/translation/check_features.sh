#!/usr/bin/env bash
# Phase D — run the full differential suite for EVERY feature combination and
# for BOTH cdylib build profiles.
#
# Feature combinations are extracted mechanically from Cargo.toml's [features]
# table (its powerset), never hard-coded. Both the debug and the release cdylib
# are tested, because optimisation level is exactly what could let the Rust
# constant-fold the exported mutable `matrix` global instead of loading it.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"
OFFLINE="--offline"   # crates.io index is unreachable in this sandbox
FAIL=0

echo "=== building the C shared library ==="
mkdir -p "$C_DIR/build" || exit 1
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$C_DIR"/build/lib*.so | head -1)"
echo "C .so: $C_SO"

# ---- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "=== ${N} non-default feature(s) found in Cargo.toml: ${FEATURES[*]:-<none>} ==="

COMBOS=("default")
COMBOS+=("--no-default-features")
if [ "$N" -gt 0 ]; then
  for ((mask = 1; mask < (1 << N); mask++)); do
    sel=()
    for ((i = 0; i < N; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
fi
echo "=== ${#COMBOS[@]} feature combination(s) to verify ==="

for combo in "${COMBOS[@]}"; do
  flags=""
  [ "$combo" != "default" ] && flags="$combo"

  echo
  echo "################################################################"
  echo "# feature combo: ${combo}"
  echo "################################################################"

  echo "--- cargo check ---"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $OFFLINE --all-targets $flags 2>&1 | tail -3; then
    echo "CHECK FAILED for [$combo]"; FAIL=1; continue
  fi

  for profile in debug release; do
    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      timeout 600 cargo build $OFFLINE --release $flags >/dev/null 2>&1 \
        || { echo "release build FAILED for [$combo]"; FAIL=1; continue; }
      RS_SO="$CRATE_DIR/target/release/libmatrixsum_lib.so"
    else
      # shellcheck disable=SC2086
      timeout 600 cargo build $OFFLINE $flags >/dev/null 2>&1 \
        || { echo "debug build FAILED for [$combo]"; FAIL=1; continue; }
      RS_SO="$CRATE_DIR/target/debug/libmatrixsum_lib.so"
    fi
    [ -f "$RS_SO" ] || { echo "missing cdylib $RS_SO"; FAIL=1; continue; }

    echo "--- cargo test  (combo=[$combo]  cdylib=$profile) ---"
    # shellcheck disable=SC2086
    C_SO="$C_SO" RUST_SO="$RS_SO" timeout 600 cargo test $OFFLINE $flags 2>&1 \
      | grep -E "^(test result|running|error|failures:|warning: unused)" \
      | sed "s/^/    [$profile] /"
    # shellcheck disable=SC2086
    C_SO="$C_SO" RUST_SO="$RS_SO" timeout 600 cargo test $OFFLINE $flags >/dev/null 2>&1 \
      || { echo "    TESTS FAILED for [$combo] cdylib=$profile"; FAIL=1; }
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "########## ALL FEATURE COMBINATIONS x BOTH CDYLIB PROFILES: PASS ##########"
else
  echo "########## FAILURES DETECTED ##########"
fi
exit "$FAIL"
