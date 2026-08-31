#!/usr/bin/env bash
# Rebuild both shared objects, then run the differential test suite.
#
# Both .so files must be current before any test runs, because every test
# dlopen()s them rather than linking against them.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$PWD

echo "== building C libjansson.so =="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . -j 2>&1 | tail -3 ) || { echo "C BUILD FAILED"; exit 1; }

echo "== building Rust libjansson.so =="
( cd translation && cargo build --release 2>&1 | tail -5 ) \
  || { echo "RUST BUILD FAILED"; exit 1; }

echo "== building va_list shim =="
mkdir -p .work
cc -shared -fPIC -O1 -o .work/libvashim.so translation/tests/vashim.c \
  || { echo "SHIM BUILD FAILED"; exit 1; }

echo "== symbol parity =="
nm -D --defined-only c_src/build/libjansson.so    | awk '{print $3}' | grep -v '^_' | sort -u > .work/c_syms.txt
nm -D --defined-only translation/target/release/libjansson.so | awk '{print $3}' | grep -v '^_' | sort -u > .work/rust_syms.txt
MISSING=$(comm -23 .work/c_syms.txt .work/rust_syms.txt)
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; exit 1
fi
echo "OK: $(wc -l < .work/c_syms.txt) symbols, 0 missing"

echo "== differential tests =="
cd translation
if [ $# -gt 0 ]; then
  timeout 600 cargo test --release "$@"
  exit $?
fi

# Run each test binary separately and total the results. `cargo test --release`
# in one shot works too, but its output is long enough that a summary is easy to
# lose, and a per-binary loop makes a single failing suite obvious.
TOTAL=0
FAILED=0
BINS=0
for f in tests/*.rs; do
  t=$(basename "$f" .rs)
  [ "$t" = "HARNESS" ] && continue
  out=$(timeout 600 cargo test --release --test "$t" 2>&1)
  line=$(printf '%s\n' "$out" | command grep -E '^test result' | head -1)
  p=$(printf '%s\n' "$line" | sed -n 's/.*[ .]\([0-9]\+\) passed.*/\1/p')
  fa=$(printf '%s\n' "$line" | sed -n 's/.* \([0-9]\+\) failed.*/\1/p')
  BINS=$((BINS + 1))
  TOTAL=$((TOTAL + ${p:-0}))
  FAILED=$((FAILED + ${fa:-0}))
  if [ -z "$line" ]; then
    printf "  %-28s CRASHED / DID NOT REPORT\n" "$t"
    FAILED=$((FAILED + 1))
    printf '%s\n' "$out" | tail -25
  elif [ "${fa:-0}" -ne 0 ]; then
    printf "  %-28s %s\n" "$t" "$line"
    printf '%s\n' "$out" | command grep -A6 'divergence\|panicked' | head -40
  else
    printf "  %-28s %s tests OK\n" "$t" "${p:-0}"
  fi
done
echo "  --------------------------------------------------"
printf "  %-28s %s tests across %s binaries, %s failed\n" "TOTAL" "$TOTAL" "$BINS" "$FAILED"
[ "$FAILED" -eq 0 ] || exit 1
