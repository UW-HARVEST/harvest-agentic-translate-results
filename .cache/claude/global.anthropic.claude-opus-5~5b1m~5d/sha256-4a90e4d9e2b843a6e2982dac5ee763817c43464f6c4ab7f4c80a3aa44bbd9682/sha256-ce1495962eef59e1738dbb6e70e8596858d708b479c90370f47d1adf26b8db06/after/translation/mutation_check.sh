#!/usr/bin/env bash
# Negative control for the differential harness.
#
# A test suite that passes is only meaningful if it can FAIL.  This script
# injects known-wrong behaviour into the Rust translation, one mutant at a
# time, and asserts the differential suite catches each one.  It always
# restores src/lib.rs afterwards.
#
# Usage: translation/mutation_check.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"
LIB=src/lib.rs
BAK="$HERE/.lib.rs.orig"      # kept inside the repo dir: $TMPDIR is not stable
LOG="$HERE/.mutation.log"
CARGO_FLAGS="--offline"

cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; }
trap 'restore; rm -f "$BAK"' EXIT

# Runs the suite; echoes "passed=N failed=M".
run_suite() {
  # Both profiles: release inlines where debug does not, and some ABI
  # properties are only observable in one of them.
  if ! cargo build $CARGO_FLAGS -q 2>"$LOG.build" \
     || ! cargo build $CARGO_FLAGS --release -q 2>>"$LOG.build"; then
    echo "BUILD-ERROR"
    return
  fi
  # Run against BOTH profiles.  Several fidelity properties (argument
  # truncation, call interposability) only diverge in optimised builds, so a
  # debug-only run would report false "all clear".
  : >"$LOG"
  for prof in debug release; do
    RUST_DRIVER_SO="$HERE/target/$prof/libdriver.so" \
      timeout 600 cargo test $CARGO_FLAGS --no-fail-fast -- --test-threads=1 >>"$LOG" 2>&1
  done
  local p f
  p=$(grep -oE '[0-9]+ passed' "$LOG" | grep -oE '[0-9]+' | awk '{s+=$1} END{print s+0}')
  f=$(grep -oE '[0-9]+ failed' "$LOG" | grep -oE '[0-9]+' | awk '{s+=$1} END{print s+0}')
  echo "passed=$p failed=$f"
}

FAIL=0

echo "=== baseline: unmutated translation must pass everything ==="
restore
R=$(run_suite)
echo "  $R"
case "$R" in
  *"failed=0"*) echo "  OK baseline is green" ;;
  *) echo "  ERROR baseline is not green!"; FAIL=1 ;;
esac

# Each mutant: description, then a sed expression applied to src/lib.rs.
run_mutant() {
  local desc="$1"; shift
  restore
  for expr in "$@"; do sed -i "$expr" "$LIB"; done
  local r; r=$(run_suite)
  if [ "$r" = "BUILD-ERROR" ]; then
    echo "  [$desc] -> BUILD-ERROR (mutant did not apply cleanly)"; FAIL=1; return
  fi
  echo "  [$desc] -> $r"
  case "$r" in
    *"failed=0"*) echo "      NOT CAUGHT -- the suite has a blind spot here!"; FAIL=1 ;;
    *) grep -m1 -A3 'panicked at' "$LOG" | sed 's/^/      /' ;;
  esac
}

# An "equivalent mutant": a source change that provably cannot alter observable
# behaviour.  The suite is expected NOT to flag it; flagging it would mean a
# test is asserting something other than behaviour.
run_equivalent_mutant() {
  local desc="$1"; shift
  restore
  for expr in "$@"; do sed -i "$expr" "$LIB"; done
  local r; r=$(run_suite)
  if [ "$r" = "BUILD-ERROR" ]; then
    echo "  [$desc] -> BUILD-ERROR (mutant did not apply cleanly)"; FAIL=1; return
  fi
  echo "  [$desc] -> $r"
  case "$r" in
    *"failed=0"*) echo "      OK correctly reported as equivalent" ;;
    *) echo "      UNEXPECTED -- suite flagged a behaviour-preserving change"; FAIL=1 ;;
  esac
}

echo
echo "=== mutants: each MUST be caught (failed>0) ==="

run_mutant "M1 driver uses saturating_add (breaks the 0x7f overflow boundary)" \
  's/data\.wrapping_add(1)/data.saturating_add(1)/'

run_mutant "M2 format %2x instead of %02x (zero padding lost)" \
  's/const FORMAT: &\[u8; 6\] = b"%02x\\n\\0";/const FORMAT: \&[u8; 5] = b"%2x\\n\\0";/'

run_mutant "M3 zero-extend instead of sign-extend before printf" \
  's/charHex as c_int/charHex as u8 as c_int/'

run_mutant "M4 driver forwards data, dropping the +1" \
  's/^    let result: c_char = data\.wrapping_add(1);$/    let result: c_char = data;/'

run_mutant "M5 uppercase %02X" \
  's/const FORMAT: &\[u8; 6\] = b"%02x\\n\\0";/const FORMAT: \&[u8; 6] = b"%02X\\n\\0";/'

run_mutant "M6 newline dropped from the format string" \
  's/const FORMAT: &\[u8; 6\] = b"%02x\\n\\0";/const FORMAT: \&[u8; 5] = b"%02x\\0";/'

run_mutant "M7 driver subtracts instead of adds" \
  's/data\.wrapping_add(1)/data.wrapping_sub(1)/'

run_mutant "M8 printHexCharLine export removed (symbol parity must fail)" \
  '0,/#\[unsafe(no_mangle)\]/s/#\[unsafe(no_mangle)\]//'

# The naive translation: `driver` calls the Rust fn by name, which LLVM inlines
# in release builds, destroying the interposability the C's PLT call has.
run_mutant "M9 driver calls printHexCharLine directly (loses PLT interposability)" \
  's/    let f = unsafe { core::ptr::read_volatile(&PRINT_HEX_CHAR_LINE) };/    #[allow(unused)] let f = printHexCharLine_via_plt;/' \
  's/    unsafe { f(result as c_int) };/    printHexCharLine(result as c_int);/'

# The naive signature: `extern "C" fn(c_char)` makes LLVM assume the caller
# sign-extended, so release builds drop gcc's re-truncation of the argument
# register and a caller passing 0x000000ff sees `ff` instead of `ffffffff`.
run_mutant "M10 printHexCharLine takes c_char (drops gcc's argument truncation)" \
  's/pub extern "C" fn printHexCharLine(charHex: c_int) {/pub extern "C" fn printHexCharLine(charHex: c_char) {/' \
  's/^    let charHex: c_char = charHex as c_char;$//'

echo
echo "=== equivalent mutants: these MUST NOT be flagged (failed==0) ==="

# Same change as M10 but applied to `driver`.  Unlike `printHexCharLine`, whose
# body is a pure sign-extension that LLVM can satisfy by forwarding %edi
# untouched, `driver` does `+ 1` on an 8-bit value, so codegen operates on %dil
# and truncates either way.  Verified by disassembly: the instruction sequence is
# identical (`inc %dil; movsbl %dil,%edi`) with and without the explicit cast.
# The explicit cast is kept in the source anyway, so the property is guaranteed
# rather than incidental -- but the suite is CORRECT to report no divergence.
run_equivalent_mutant "E1 driver takes c_char (provably equivalent: +1 forces 8-bit codegen)" \
  's/pub extern "C" fn driver(data: c_int) {/pub extern "C" fn driver(data: c_char) {/' \
  's/^    let data: c_char = data as c_char;$//'

restore
cargo build $CARGO_FLAGS -q
rm -f "$LOG" "$LOG.build"

echo
if [ "$FAIL" -eq 0 ]; then
  echo "############ every mutant was caught; the harness has teeth ############"
else
  echo "############ MUTATION CHECK FAILED ############"
fi
exit $FAIL
