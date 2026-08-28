#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check` plus the
# full C-vs-Rust differential test suite for each one.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
CRATE=$ROOT/translation
CARGO_TOML=$CRATE/Cargo.toml

# --- 1. build the C reference shared object --------------------------------
if [ ! -d "$ROOT/c_src/build" ] || ! ls "$ROOT/c_src/build"/lib*.so >/dev/null 2>&1; then
  echo "== building C reference library =="
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1 ) \
    || { echo "C build FAILED, see /tmp/cmake.log"; exit 1; }
fi

# --- 2. enumerate feature combinations ------------------------------------
# Feature names in [features], minus "default" (covered by the default build).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CARGO_TOML"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "== no [features] declared: the crate has a single configuration =="
  COMBOS+=("--no-default-features")
  COMBOS+=("")            # default features (identical here, checked anyway)
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo+="${FEATURES[$i]},"
      fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
  COMBOS+=("")            # default feature set
fi

# --- 3. check + test every combination ------------------------------------
FAILED=0
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo${profile_flag:+ $profile_flag} ${combo:-<default features>}"

    echo "== check: $label =="
    if ! ( cd "$CRATE" && timeout 600 cargo check $profile_flag $combo --all-targets \
            >/tmp/check.log 2>&1 ); then
      echo "   CHECK FAILED"; tail -30 /tmp/check.log; FAILED=1; continue
    fi

    # The tests dlopen the cdylib, so it has to exist for this profile/feature set.
    echo "== build: $label =="
    if ! ( cd "$CRATE" && timeout 600 cargo build $profile_flag $combo \
            >/tmp/build.log 2>&1 ); then
      echo "   BUILD FAILED"; tail -30 /tmp/build.log; FAILED=1; continue
    fi

    echo "== test:  $label =="
    if ! ( cd "$CRATE" && timeout 600 cargo test $profile_flag $combo \
            -- --test-threads=1 >/tmp/test.log 2>&1 ); then
      echo "   TEST FAILED"; tail -40 /tmp/test.log; FAILED=1; continue
    fi
    grep -h "^test result:" /tmp/test.log | sed 's/^/   /'
  done
done

# --- 4. symbol parity (also asserted as a test, reported here for the log) --
echo "== dynamic symbol comparison =="
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
for so in "$C_SO" "$CRATE/target/debug/libintput_lib.so" "$CRATE/target/release/libintput_lib.so"; do
  [ -f "$so" ] || continue
  echo "-- $(basename "$so")"
  nm -D --defined-only "$so" | awk '{print $3}' | sort | sed 's/^/   /'
done

if [ "$FAILED" -ne 0 ]; then
  echo "RESULT: FAILURES"; exit 1
fi
echo "RESULT: all feature combinations checked and tested successfully"
