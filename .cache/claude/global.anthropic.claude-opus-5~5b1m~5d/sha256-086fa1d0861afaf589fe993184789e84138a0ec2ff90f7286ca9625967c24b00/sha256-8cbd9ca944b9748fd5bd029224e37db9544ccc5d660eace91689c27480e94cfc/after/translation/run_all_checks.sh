#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust cdylib, then runs the
# differential suite under every Cargo feature combination and both profiles.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
fail=0

echo "==> building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$HERE"

# ---------------------------------------------------------------------------
# Enumerate the power set of the crate's declared features.
# The crate declares none, so the only combination is the (empty) default.
# ---------------------------------------------------------------------------
FEATURES=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["packages"][0]["features"].keys()))')
echo "==> declared features: [${FEATURES:-<none>}]"

combos=()
if [ -z "${FEATURES// }" ]; then
  combos+=("DEFAULT")
  combos+=("NO_DEFAULT")
else
  # shellcheck disable=SC2206
  arr=($FEATURES)
  n=${#arr[@]}
  combos+=("DEFAULT")
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then sel="$sel,${arr[$i]}"; fi
    done
    combos+=("NO_DEFAULT${sel:+:${sel#,}}")
  done
fi

for profile in dev release; do
  prof_flag=""
  [ "$profile" = "release" ] && prof_flag="--release"
  for combo in "${combos[@]}"; do
    case "$combo" in
      DEFAULT)      fflags=() ; label="default-features" ;;
      NO_DEFAULT)   fflags=(--no-default-features) ; label="no-default-features" ;;
      NO_DEFAULT:*) fflags=(--no-default-features --features "${combo#NO_DEFAULT:}")
                    label="no-default+${combo#NO_DEFAULT:}" ;;
    esac
    echo
    echo "======================================================================"
    echo "==> profile=$profile features=$label"
    echo "======================================================================"
    # The cdylib must exist on disk before the tests dlopen it.
    cargo build $prof_flag "${fflags[@]}" -q || { echo "BUILD FAILED"; fail=1; continue; }
    timeout 600 cargo test $prof_flag "${fflags[@]}" -- --test-threads=1 2>&1 \
      | grep -E "^test result|^error|FAILED|panicked" \
      || true
    # shellcheck disable=SC2181
    timeout 600 cargo test $prof_flag "${fflags[@]}" -- --test-threads=1 >/dev/null 2>&1 \
      || { echo "!! TESTS FAILED for profile=$profile features=$label"; fail=1; }
  done
done

echo
echo "==> symbol diff (C .so vs Rust .so, release)"
diff <(nm -D --defined-only --format=posix "$ROOT/c_src/build/libdriver.so" | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix "$HERE/target/release/libdriver.so" | awk '{print $1}' | sort) \
  && echo "SYMBOL DIFF EMPTY (identical export sets)" \
  || { echo "!! SYMBOL DIFF NON-EMPTY"; fail=1; }

echo
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
