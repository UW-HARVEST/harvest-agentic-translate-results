#!/usr/bin/env bash
# Enumerate every build-time configuration of this project and check/test each.
#
# Sources of build-time configuration:
#   * translation/Cargo.toml [features]
#   * c_src/CMakeLists.txt   option() / set() / *_definitions
#
# Usage: ./verify_all_features.sh [check|test]     (default: test)
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CRATE="$ROOT/translation"
MODE="${1:-test}"
FAIL=0

echo "=== Build-time configuration discovery ==="

# --- Cargo features -------------------------------------------------------
FEATURES="$(awk '
  /^\[features\]/ { inside = 1; next }
  /^\[/           { inside = 0 }
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ { sub(/[ \t]*=.*/, ""); print }
' "$CRATE/Cargo.toml")"

# --- CMake build switches -------------------------------------------------
CMAKE_OPTS="$(grep -Eio '^[[:space:]]*(option|set)\([[:space:]]*[A-Za-z0-9_]+' \
  "$ROOT/c_src/CMakeLists.txt" 2>/dev/null | sed -E 's/.*\(//' || true)"
CMAKE_DEFS="$(grep -Eic 'target_compile_definitions|add_definitions|add_compile_definitions' \
  "$ROOT/c_src/CMakeLists.txt" || true)"

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml   : no [features] section -> no optional cargo features"
else
  echo "Cargo.toml   : features found:"
  echo "$FEATURES" | sed 's/^/               - /'
fi

if [ -z "$CMAKE_OPTS" ]; then
  echo "CMakeLists   : no option/set build switches"
else
  echo "CMakeLists   : switches found:"
  echo "$CMAKE_OPTS" | sed 's/^/               - /'
fi
echo "CMakeLists   : compile-definition directives: $CMAKE_DEFS"

# --- Enumerate combinations ----------------------------------------------
# With N cargo features there are 2^N subsets; with none there is exactly one
# configuration, which must still be checked with --no-default-features.
COMBOS=()
if [ -z "$FEATURES" ]; then
  COMBOS+=("")
else
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then
        combo="${combo:+$combo,}${FARR[$b]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo
echo "=== ${#COMBOS[@]} feature combination(s) to verify (mode: $MODE) ==="

run() {
  local label="$1"
  shift
  echo
  echo "--- $label ---"
  if timeout 600 "$@" >/tmp/featcheck.log 2>&1; then
    tail -n 4 /tmp/featcheck.log
    echo "OK: $label"
  else
    echo "FAIL: $label"
    tail -n 40 /tmp/featcheck.log
    FAIL=1
  fi
}

cd "$CRATE" || exit 1

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="--no-default-features"
    featargs=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    featargs=(--no-default-features --features "$combo")
  fi

  run "cargo check $label" cargo check "${featargs[@]}" --all-targets
  if [ "$MODE" = "test" ]; then
    # The cdylib under test is loaded via libloading, so it must be rebuilt
    # for this configuration before the tests run. Both profiles are exercised:
    # they differ in optimisation level and in `panic = "abort"`.
    run "cargo build --release $label" cargo build --release "${featargs[@]}"
    run "cargo build (debug) $label" cargo build "${featargs[@]}"
    STR_DUPS_RUST_SO="$CRATE/target/release/libstr_dups_lib.so" \
      run "cargo test $label [release cdylib]" cargo test "${featargs[@]}"
    STR_DUPS_RUST_SO="$CRATE/target/debug/libstr_dups_lib.so" \
      run "cargo test $label [debug cdylib]" cargo test "${featargs[@]}"
  fi
done

# Also verify the default configuration explicitly.
run "cargo check (default features)" cargo check --all-targets
if [ "$MODE" = "test" ]; then
  run "cargo build --release (default features)" cargo build --release
  run "cargo build (debug, default features)" cargo build
  STR_DUPS_RUST_SO="$CRATE/target/release/libstr_dups_lib.so" \
    run "cargo test (default features) [release cdylib]" cargo test
  STR_DUPS_RUST_SO="$CRATE/target/debug/libstr_dups_lib.so" \
    run "cargo test (default features) [debug cdylib]" cargo test
fi

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
