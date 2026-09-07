#!/usr/bin/env bash
# Full verification driver: builds both libraries, checks symbol parity, then
# runs the whole differential suite under EVERY feature combination and profile.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
MF="$CRATE/Cargo.toml"
CARGO_FLAGS="${CARGO_FLAGS:---offline}"   # drop --offline if crates.io is reachable
TMP="${TMPDIR:-/tmp}"
rc=0

echo "== 1. build C shared library =="
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$ROOT/c_src/build" >/dev/null || rc=1
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "   C .so: $C_SO"

echo "== 2. build Rust cdylib (both profiles) =="
# NOTE: `cargo test` does NOT rebuild a crate-type=["cdylib"] artifact, so the
# .so must be built explicitly or a STALE library gets verified.
cargo build $CARGO_FLAGS --manifest-path "$MF" -q           || rc=1
cargo build $CARGO_FLAGS --manifest-path "$MF" --release -q || rc=1
RS_SO="$CRATE/target/release/libnext_double_lib.so"
echo "   Rust .so: $RS_SO (+ target/debug/)"

echo "== 3. symbol parity (nm -D) =="
nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u > "$CRATE/c_syms.txt"
nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u > "$CRATE/rust_syms.txt"
missing="$(comm -23 "$CRATE/c_syms.txt" "$CRATE/rust_syms.txt")"
extra="$(comm -13 "$CRATE/c_syms.txt" "$CRATE/rust_syms.txt")"
echo "   C exports   : $(wc -l < "$CRATE/c_syms.txt")   [$(tr '\n' ' ' < "$CRATE/c_syms.txt")]"
echo "   Rust exports: $(wc -l < "$CRATE/rust_syms.txt")   [$(tr '\n' ' ' < "$CRATE/rust_syms.txt")]"
if [ -n "$missing" ]; then echo "   MISSING FROM RUST:"; echo "$missing"; rc=1
else echo "   missing from Rust: NONE"; fi
if [ -n "$extra" ]; then echo "   EXTRA IN RUST: $extra"; rc=1
else echo "   extra in Rust    : NONE"; fi

echo "   undefined non-libc symbols in Rust .so:"
nm -D -u "$RS_SO" | awk '{print $2}' \
  | grep -vE '@(GLIBC|GCC)' \
  | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
  | grep -vE '^(abort|bcmp|calloc|free|malloc|memcpy|memmove|memset|realloc|strlen|posix_memalign)$' \
  | grep -v '^$' > "$TMP/undef.$$" || true
if [ -s "$TMP/undef.$$" ]; then echo "   NON-LIBC UNDEFINED:"; cat "$TMP/undef.$$"; rc=1
else echo "   NONE"; fi
rm -f "$TMP/undef.$$"

echo "== 4. feature combinations x profiles =="
# The crate declares no [features] table, so these three invocations are the
# complete configuration space. Checked, not assumed:
if grep -q '^\[features\]' "$MF"; then
  echo "   !! a [features] table appeared; enumerate its power set here"; rc=1
fi
COMBOS=("" "--no-default-features" "--all-features")
for f in "${COMBOS[@]}"; do
  for prof in "" "--release"; do
    label="${f:-<default>} ${prof:-<dev>}"
    cargo build $CARGO_FLAGS $f $prof --manifest-path "$MF" -q || { echo "build failed: $label"; rc=1; }
    out="$(cargo test $CARGO_FLAGS $f $prof --manifest-path "$MF" --no-fail-fast 2>&1)"
    st=$?
    printf '  %-34s %s\n' "$label" "$(echo "$out" | grep -c '^test .* \.\.\. ok$') tests ok"
    if [ $st -ne 0 ]; then
      echo "   FAILED for combo [$label]"
      echo "$out" | grep -E '^test .* FAILED$|panicked' | head
      rc=1
    fi
  done
done

echo
if [ $rc -eq 0 ]; then echo "VERIFICATION: ALL CHECKS PASSED"; else echo "VERIFICATION: FAILURES PRESENT"; fi
exit $rc
