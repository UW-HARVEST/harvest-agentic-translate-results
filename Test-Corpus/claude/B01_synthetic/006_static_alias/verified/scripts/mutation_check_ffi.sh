#!/usr/bin/env bash
# Like mutation_check.sh but runs ONLY the FFI (dlopen/libloading) test binaries,
# proving that the tests which go through the exported C ABI symbols - not just
# the process-level CLI comparison - detect divergence.  It also exercises the
# "cargo test --test <name> does not rebuild the example" path: the test helper
# must notice the stale .so and rebuild it.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
SRC=src/lib.rs
BACKUP="$(mktemp)"
cp "$SRC" "$BACKUP"
restore() { cp "$BACKUP" "$SRC"; rm -f "$BACKUP"; }
trap restore EXIT

MUTATIONS=(
  "alias_gt_instead_of_ge|s/if \*outer >= \*inner {/if *outer > *inner {/"
  "alias_then_saturating|s/\*inner = (\*inner).wrapping_add(\*outer);/*inner = (*inner).saturating_add(*outer);/"
  "alias_else_saturating|s/\*outer = (\*outer).wrapping_add(\*inner);/*outer = (*outer).saturating_add(*inner);/"
  "inner_not_static|s/static mut INNER: c_int = 1;/static mut INNER: c_int = 2;/"
  "swap_error_messages|s/second argument must be an integer/2nd argument must be an integer/"
  "strtol_no_saturation|s/return (if negative { c_long::MIN } else { c_long::MAX }, i);/return (0, i);/"
  "loop_off_by_one|s/while i < iterations {/while i <= iterations {/"
  "narrowing_clamped|s/let iterations: c_int = raw2 as c_int;/let iterations: c_int = raw2.clamp(c_int::MIN as c_long, c_int::MAX as c_long) as c_int;/"
)

TESTS=(ffi_static_alias ffi_main ffi_errors)

fail=0
for entry in "${MUTATIONS[@]}"; do
  name="${entry%%|*}"
  expr="${entry#*|}"
  cp "$BACKUP" "$SRC"
  sed -i "$expr" "$SRC"
  if cmp -s "$BACKUP" "$SRC"; then
    echo "!! $name: no-op mutation"; fail=1; continue
  fi
  detected=()
  for t in "${TESTS[@]}"; do
    if timeout 300 cargo test --offline --test "$t" >/dev/null 2>&1; then
      :
    else
      detected+=("$t")
      # Stop at the first FFI test binary that catches the mutation: the point is
      # that the dlopen-based tests detect it, and this keeps the runtime bounded.
      break
    fi
  done
  if [ "${#detected[@]}" -eq 0 ]; then
    echo "FAIL $name NOT detected by any FFI test binary"
    fail=1
  else
    echo "OK   $name detected by: ${detected[*]}"
  fi
done

cp "$BACKUP" "$SRC"
[ "$fail" -eq 0 ] && echo "all mutations detected through the FFI exports"
exit "$fail"
