#!/usr/bin/env bash
# Mutation check: proves the differential suite is not vacuous. Each mutation
# injects a plausible translation bug into src/lib.rs; the suite MUST fail for
# every one, and MUST pass once the original source is restored.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE"

BAK="$(mktemp)"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

# name|sed expression
MUTATIONS=(
  "signed_char_promotion|s/\*p.offset(i as isize) as c_int,/*p.offset(i as isize) as i8 as c_int,/"
  "short_length|s/core::mem::size_of::<c_int>() as c_int,/(core::mem::size_of::<c_int>() as c_int) - 1,/"
  "long_length|s/core::mem::size_of::<c_int>() as c_int,/(core::mem::size_of::<c_int>() as c_int) + 1,/"
  "byte_swap|s/pub extern \"C\" fn driver(x: c_int) {/pub extern \"C\" fn driver(x: c_int) { let x = x.swap_bytes();/"
  "uppercase_hex|s/b\"%02x\\\\0\"/b\"%02X\\\\0\"/"
  "no_padding|s/b\"%02x\\\\0\"/b\"%x\\\\0\"/"
  "missing_newline|s/printf(b\"\\\\n\\\\0\".as_ptr() as \*const c_char);//"
)

fail=0
for m in "${MUTATIONS[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  restore
  sed -i "$expr" src/lib.rs
  if ! diff -q "$BAK" src/lib.rs >/dev/null; then
    :
  else
    echo "MUTATION $name: sed did not change src/lib.rs — mutation is a no-op!"
    fail=1
    continue
  fi
  timeout 600 cargo build >/dev/null 2>&1
  out=$(timeout 600 cargo test 2>&1)
  if echo "$out" | grep -q "^result: FAILED"; then
    echo "MUTATION $name: detected (suite failed) OK"
  else
    echo "MUTATION $name: NOT DETECTED — suite is vacuous for this bug"
    echo "$out" | tail -n 5
    fail=1
  fi
done

restore
timeout 600 cargo build >/dev/null 2>&1
if timeout 600 cargo test 2>&1 | grep -q "^result: ok"; then
  echo "RESTORED original src/lib.rs: suite passes OK"
else
  echo "RESTORED original src/lib.rs: suite FAILS — unexpected"
  fail=1
fi

exit $fail
