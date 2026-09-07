#!/usr/bin/env bash
# Full verification sweep: builds the C .so, then runs the whole differential
# test suite against BOTH the debug and the release Rust cdylib, for EVERY
# feature combination declared in Cargo.toml.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$here" || exit 1

CARGO_FLAGS="--offline"

# ---------------------------------------------------------------- C library
echo "==> building the C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$root"/c_src/build/lib*.so | head -1)"
echo "    $C_SO"
export C_LIB_SO="$C_SO"

# ------------------------------------------------------- feature combinations
# Extract the [features] table (if any) from Cargo.toml.
features="$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)"

combos=()
combos+=("DEFAULT|")
combos+=("NO-DEFAULT|--no-default-features")
if [ -n "$features" ]; then
  combos+=("ALL|--all-features")
  while read -r f; do
    [ -z "$f" ] && continue
    combos+=("only:$f|--no-default-features --features $f")
  done <<< "$features"
  # every pair, so interactions are covered too
  mapfile -t flist <<< "$features"
  n=${#flist[@]}
  for ((i = 0; i < n; i++)); do
    for ((j = i + 1; j < n; j++)); do
      combos+=("pair:${flist[$i]}+${flist[$j]}|--no-default-features --features ${flist[$i]},${flist[$j]}")
    done
  done
else
  echo "==> Cargo.toml declares no [features]; DEFAULT and NO-DEFAULT are the"
  echo "    only combinations (they are identical), plus --all-features."
  combos+=("ALL|--all-features")
fi

fail=0
for entry in "${combos[@]}"; do
  name="${entry%%|*}"
  flags="${entry#*|}"
  for profile in debug release; do
    label="[$name / $profile]"
    echo
    echo "=============================================================="
    echo "==> $label  cargo flags: '${flags:-<none>}'"
    echo "=============================================================="

    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      cargo build $CARGO_FLAGS --release $flags >/dev/null 2>&1 \
        || { echo "$label cdylib build FAILED"; fail=1; continue; }
      export RUST_LIB_SO="$here/target/release/libdoubleneg_lib.so"
    else
      # shellcheck disable=SC2086
      cargo build $CARGO_FLAGS $flags >/dev/null 2>&1 \
        || { echo "$label cdylib build FAILED"; fail=1; continue; }
      export RUST_LIB_SO="$here/target/debug/libdoubleneg_lib.so"
    fi
    echo "    Rust .so: $RUST_LIB_SO"

    # nm diff, independent of the test suite
    symdiff_file="${TMPDIR:-/tmp}/symdiff.$$"
    diff <(nm -D --defined-only --extern-only "$C_SO" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only --extern-only "$RUST_LIB_SO" | awk '{print $NF}' \
             | grep -v -E '^(_init|_fini|__|_ITM_|_Unwind|rust_)' | sort) \
      > "$symdiff_file"
    missing="$(grep '^<' "$symdiff_file" || true)"
    if [ -n "$missing" ]; then
      echo "$label SYMBOL PARITY FAILED -- missing from Rust .so:"
      echo "$missing"
      fail=1
    else
      echo "    symbol parity: OK (0 missing)"
    fi
    rm -f "$symdiff_file" 2>/dev/null

    # shellcheck disable=SC2086
    if ! cargo test $CARGO_FLAGS $flags -- --test-threads=1 2>&1 \
        | grep -E '^(     Running|running |test result|test .* FAILED|error|---- )'; then
      echo "$label TESTS FAILED"
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$fail"
