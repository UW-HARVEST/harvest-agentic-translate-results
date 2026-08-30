#!/usr/bin/env bash
# Phase D -- run the full differential suite under EVERY feature combination
# and under BOTH optimization profiles.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so this stays correct if features are ever added.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR=$PWD
C_BUILD=$CRATE_DIR/../c_src/build
OFFLINE=${OFFLINE:---offline}
fail=0

echo "=== Building the C ground-truth shared library ==="
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$C_BUILD/libdriver.so" || { echo "libdriver.so missing"; exit 1; }

# ---- enumerate the declared features -------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
    }
  ' Cargo.toml
)

echo
echo "=== Declared features: ${#FEATURES[@]} (${FEATURES[*]:-none}) ==="

# Build the list of feature configurations to test.
CONFIGS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table => the only configurations that exist are the default
  # build and --no-default-features, which are the same code.
  CONFIGS+=("default:")
  CONFIGS+=("no-default-features:--no-default-features")
else
  CONFIGS+=("default:")
  CONFIGS+=("no-default-features:--no-default-features")
  CONFIGS+=("all-features:--all-features")
  # Every non-empty subset of the declared features (powerset).
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${combo[*]}")
    CONFIGS+=("$joined:--no-default-features --features $joined")
  done
fi

# ---- run cargo check + the suite for each (config x profile) --------------
for profile_flag in "--release" ""; do
  profile_name=${profile_flag:---debug}
  for entry in "${CONFIGS[@]}"; do
    name=${entry%%:*}
    flags=${entry#*:}

    echo
    echo "------------------------------------------------------------------"
    echo ">>> profile=${profile_name}  features=[${name}]"
    echo "------------------------------------------------------------------"

    # shellcheck disable=SC2086
    if ! cargo check $OFFLINE $profile_flag $flags >/dev/null 2>&1; then
      echo "!!! cargo check FAILED for ${name} / ${profile_name}"
      fail=1
      continue
    fi

    # Build the cdylib for this exact configuration, then point the harness at
    # it explicitly so we always test the artifact we just produced.
    # shellcheck disable=SC2086
    if ! cargo build $OFFLINE $profile_flag $flags >/dev/null 2>&1; then
      echo "!!! cargo build FAILED for ${name} / ${profile_name}"
      fail=1
      continue
    fi

    if [ -n "$profile_flag" ]; then so=target/release/libdriver.so; else so=target/debug/libdriver.so; fi
    if [ ! -f "$so" ]; then
      echo "!!! cdylib $so missing for ${name} / ${profile_name}"
      fail=1
      continue
    fi

    echo "--- nm -D parity for $so ---"
    if diff <(nm -D --defined-only "$C_BUILD/libdriver.so" | awk '{print $3}' | sort) \
            <(nm -D --defined-only "$so"                   | awk '{print $3}' | sort); then
      echo "symbol diff: EMPTY (ok)"
    else
      echo "!!! SYMBOL DIFF NON-EMPTY for ${name} / ${profile_name}"
      fail=1
    fi

    # shellcheck disable=SC2086
    if RUST_DRIVER_SO="$CRATE_DIR/$so" C_DRIVER_SO="$C_BUILD/libdriver.so" \
       timeout 600 cargo test $OFFLINE $profile_flag $flags 2>&1 | grep -E 'result:|FAILED|panicked'; then
      :
    fi
    # shellcheck disable=SC2086
    RUST_DRIVER_SO="$CRATE_DIR/$so" C_DRIVER_SO="$C_BUILD/libdriver.so" \
      timeout 600 cargo test $OFFLINE $profile_flag $flags >/dev/null 2>&1
    if [ $? -ne 0 ]; then
      echo "!!! TESTS FAILED for ${name} / ${profile_name}"
      fail=1
    else
      echo "tests: PASS for ${name} / ${profile_name}"
    fi
  done
done

echo
echo "=================================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL feature combinations x profiles PASSED"
else
  echo "FAILURES detected -- see output above"
fi
echo "=================================================================="
exit $fail
