#!/usr/bin/env bash
# Phase D driver: run the whole differential suite for every feature
# combination and for every profile.
#
# Usage: ./run_all.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"
LOGD="$PWD/target/difflogs"
mkdir -p "$LOGD"
FAIL=0

# --- 1. enumerate feature combinations from Cargo.toml ---------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
echo "=== declared features: ${FEATURES[*]:-<none>} ==="

COMBOS=()
COMBOS+=("default:")                       # default features
COMBOS+=("no-default:--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  # full power set of the declared features (on top of --no-default-features)
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$joined:--no-default-features --features $joined")
  done
fi

# --- 2. make sure the C reference library exists ---------------------------
C_SO=$(ls ../c_src/build/*.so 2>/dev/null | head -n1)
if [[ -z "$C_SO" ]]; then
  echo "!! C .so missing; building it"
  ( cd ../c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || exit 1
  C_SO=$(ls ../c_src/build/*.so | head -n1)
fi
echo "=== C reference: $C_SO ==="

# --- 3. run --------------------------------------------------------------
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"
  flags="${entry#*:}"

  for profile in dev release; do
    if [[ $profile == release ]]; then
      pflag="--release"; pdir="release"
    else
      pflag="";          pdir="debug"
    fi

    echo
    echo "############################################################"
    echo "# combo=$name  profile=$profile  flags='$flags'"
    echo "############################################################"

    # build the cdylib for this combo/profile ...
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $CARGO_FLAGS $flags $pflag > "$LOGD/build-$name-$profile.log" 2>&1; then
      tail -30 "$LOGD/build-$name-$profile.log"; FAIL=1; continue
    fi

    # ... and drive it with the (always-debug, unwinding) test harness so the
    # `panic = "abort"` release profile cannot interfere with libtest.
    # shellcheck disable=SC2086
    if ! RUST_SO="$PWD/target/$pdir/libinreftree_lib.so" \
         timeout 600 cargo test $CARGO_FLAGS $flags -- --test-threads=1 \
         > "$LOGD/test-$name-$profile.log" 2>&1; then
      echo "!! FAILED (combo=$name profile=$profile)"
      grep -E "^(test |error|failures:|thread)" "$LOGD/test-$name-$profile.log" | head -60
      FAIL=1
    else
      grep -E "^test result:" "$LOGD/test-$name-$profile.log"
      echo "   .so under test: target/$pdir/libinreftree_lib.so"
    fi
  done
done

echo
if (( FAIL )); then
  echo "########## SOME COMBINATIONS FAILED ##########"
else
  echo "########## ALL COMBINATIONS PASSED ##########"
fi
exit $FAIL
