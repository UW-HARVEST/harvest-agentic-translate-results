#!/usr/bin/env bash
# Rebuild both libraries and run the differential test suite.
# Usage: ./run_diff_tests.sh [extra cargo test args...]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

echo "=== building C shared library ==="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
c_so="$(ls "$root"/c_src/build/lib*.so)"
echo "C  .so: $c_so"

echo "=== building Rust cdylib (dev profile, used by cargo test) ==="
( cd "$here" && timeout 600 cargo build )
echo "RS .so: $here/target/debug/libstr_dups_lib.so"

echo "=== symbol diff ==="
if diff <(nm -D --defined-only "$c_so" | awk '$2=="T"{print $3}' | sort) \
        <(nm -D --defined-only "$here/target/debug/libstr_dups_lib.so" | awk '$2=="T"{print $3}' | sort)
then
  echo "symbol diff EMPTY"
else
  echo "SYMBOL DIFF NON-EMPTY" >&2
  exit 1
fi

echo "=== cargo test ==="
cd "$here"
timeout 600 cargo test "$@"
