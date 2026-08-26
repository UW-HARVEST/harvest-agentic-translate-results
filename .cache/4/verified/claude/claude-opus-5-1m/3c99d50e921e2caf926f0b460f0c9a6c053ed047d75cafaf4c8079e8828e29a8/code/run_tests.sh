#!/usr/bin/env bash
# Differential verification driver.
#
#   1. builds the C shared library with CMake (if needed)
#   2. builds the Rust cdylib for the dev and release profiles
#   3. runs the differential test suite for EVERY feature combination, in both
#      profiles, sequentially (fd-1 capture requires --test-threads=1)
#
# Usage: ./run_tests.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=${CARGO_FLAGS:---offline}

# --- 1. C shared library -----------------------------------------------------
if [[ ! -f c_src/build/libStaticLoop.so ]]; then
  echo "=== building the C shared library ==="
  ( mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
ls -l c_src/build/libStaticLoop.so

# --- feature combinations (powerset of Cargo.toml's [features]) --------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, "", $0); print }
  ' Cargo.toml
)
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  COMBOS=()
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      (( (mask >> i) & 1 )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations: ${#COMBOS[@]} (${FEATURES[*]:-none declared})"

rc=0
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    label=${combo:-"<none>"}
    echo
    echo "############################################################"
    echo "# features='$label'  profile=$profile"
    echo "############################################################"

    rel_flag=()
    [[ $profile == release ]] && rel_flag=(--release)

    # The cdylib is NOT built by `cargo test`, so build it explicitly.
    if ! timeout 600 cargo build $CARGO_FLAGS "${rel_flag[@]}" \
          --no-default-features --features "$combo"; then
      echo "BUILD FAILED (features='$label', profile=$profile)"
      rc=1
      continue
    fi

    if ! timeout 600 cargo test $CARGO_FLAGS "${rel_flag[@]}" \
          --no-default-features --features "$combo" -- --test-threads=1; then
      echo "TESTS FAILED (features='$label', profile=$profile)"
      rc=1
    fi
  done
done

# --- symbol parity (Phase D) -------------------------------------------------
echo
echo "=== nm -D symbol diff (C vs Rust) ==="
for so in target/debug/libStaticLoop.so target/release/libStaticLoop.so; do
  [[ -f $so ]] || continue
  echo "--- $so ---"
  if diff <(nm -D --defined-only c_src/build/libStaticLoop.so | awk '{print $NF}' | sort) \
          <(nm -D --defined-only "$so"                        | awk '{print $NF}' | sort); then
    echo "symbol sets are IDENTICAL"
  else
    echo "SYMBOL DIFF NON-EMPTY for $so"
    rc=1
  fi
done

echo
if (( rc == 0 )); then
  echo "ALL DIFFERENTIAL TESTS PASSED IN ALL CONFIGURATIONS"
else
  echo "FAILURES DETECTED"
fi
exit $rc
