#!/usr/bin/env bash
# Phase D driver: rebuild C + Rust, check symbol parity, run every feature
# combination of the test suite. Cargo.toml declares no [features], so the
# combination set is {default, --no-default-features, --all-features}.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fail=0

echo "=== Building C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
CSO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C .so: $CSO"

for prof in debug release; do
  echo
  echo "############ profile: $prof ############"
  relflag=""; [ "$prof" = release ] && relflag="--release"

  for combo in "" "--no-default-features" "--all-features"; do
    label="${combo:-<default>}"
    echo
    echo "--- features: $label / $prof ---"

    cargo build --offline $relflag $combo >/dev/null 2>&1 \
      || { echo "BUILD FAILED ($label/$prof)"; fail=1; continue; }

    RSO="$ROOT/translation/target/$prof/libhdr_compare_lib.so"
    [ -f "$RSO" ] || { echo "MISSING $RSO"; fail=1; continue; }

    # ---- symbol parity ----
    nm -D --defined-only "$CSO" | awk '{print $NF}' | sort -u > "$ROOT"/translation/target/c.sym
    nm -D --defined-only "$RSO" | awk '{print $NF}' | sort -u > "$ROOT"/translation/target/r.sym
    missing="$(comm -23 "$ROOT"/translation/target/c.sym "$ROOT"/translation/target/r.sym)"
    if [ -n "$missing" ]; then
      echo "SYMBOL PARITY FAIL - missing from Rust .so:"; echo "$missing"; fail=1
    else
      echo "symbol parity: OK ($(wc -l < "$ROOT"/translation/target/c.sym) C symbol(s), 0 missing)"
    fi
    # non-libc undefined symbols in Rust .so
    und="$(nm -D --undefined-only "$RSO" | awk '{print $NF}' \
      | grep -v '@GLIBC' | grep -v '@GCC' | grep -vE '^(_ITM_|__gmon_start__|statx|gettid)' )"
    if [ -n "$und" ]; then
      echo "UNDEFINED NON-LIBC SYMBOLS:"; echo "$und"; fail=1
    else
      echo "undefined non-libc symbols: 0"
    fi

    # ---- differential tests ----
    timeout 600 cargo test --offline $relflag $combo 2>&1 | grep -E "^test result|FAILED|panicked"
    [ "${PIPESTATUS[0]}" -eq 0 ] || { echo "TESTS FAILED ($label/$prof)"; fail=1; }
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "==== ALL CONFIGURATIONS PASS ===="; else echo "==== FAILURES PRESENT ===="; fi
exit $fail
