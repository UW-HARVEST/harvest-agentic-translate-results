#!/usr/bin/env bash
# UB audit: run the whole differential suite against a Rust `.so` built with
# `overflow-checks = on` (cargo's dev-profile default), while the C `.so` stays
# the same.
#
# The shipped profile sets `overflow-checks = false`, which makes Rust's integer
# arithmetic wrap exactly like C's unsigned arithmetic. That is what the
# translation needs — but it also means an accidental `a - b` where the C relies
# on a wrap would be *silently* correct in release and only wrong if the profile
# ever changed. Running the identical suite with the checks ON turns every such
# place into a loud panic naming the file and line, so the wraps can be made
# explicit (`wrapping_sub`/`wrapping_add`) and the behaviour becomes
# profile-independent.
#
# A clean run therefore means: every integer wrap the library depends on is
# spelled out in the source, not inherited from a compiler flag.
set -euo pipefail
cd "$(dirname "$0")"

TD="${TMPDIR:-/tmp}"
mkdir -p "$TD"

echo "=== building C shared object ==="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . -j "$(nproc)" >/dev/null )

echo "=== building Rust cdylib with overflow-checks ON (dev profile) ==="
cargo build --offline 2>&1 | tail -2
echo "=== building the test binaries (release, for speed) ==="
cargo build --offline --release --tests 2>&1 | tail -2

export ZSTD_C_SO="$PWD/c_src/build/libzstd.so"
export ZSTD_RUST_SO="$PWD/target/debug/libzstd.so"
echo
echo "C    .so : $ZSTD_C_SO"
echo "Rust .so : $ZSTD_RUST_SO   (overflow-checks=ON, debug-assertions=ON)"
echo

FAIL=0
for f in tests/t*.rs; do
  n="$(basename "$f" .rs)"
  BIN="$(ls -t "target/release/deps/${n}"-* 2>/dev/null | grep -v '\.d$' | head -1 || true)"
  [ -z "$BIN" ] && { echo "  (no binary for $n, skipped)"; continue; }
  printf '%-28s ' "$n"
  if timeout 580 "$BIN" --test-threads="$(nproc)" > "$TD/ovfaudit_$n.log" 2>&1; then
    grep -m1 'test result:' "$TD/ovfaudit_$n.log" || echo "ok"
  else
    echo "FAILED (exit $?)"
    grep -m4 -E 'panicked at|attempt to' "$TD/ovfaudit_$n.log" | sed 's/^/    /'
    FAIL=1
  fi
done

echo
if [ "$FAIL" = "0" ]; then
  echo "OVERFLOW AUDIT CLEAN: no arithmetic-overflow panic anywhere in the suite,"
  echo "so every wrap the C relies on is explicit in the Rust source."
else
  echo "OVERFLOW AUDIT FAILED — see the panics above; each names the src/ line"
  echo "whose arithmetic needs an explicit wrapping_* operation."
  exit 1
fi
