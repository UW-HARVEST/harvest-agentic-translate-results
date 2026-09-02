#!/usr/bin/env bash
# Mutation test: a differential suite that cannot fail proves nothing. Inject a
# deliberate bug into the Rust translation, confirm the suite CATCHES it, then
# restore. Every mutation must be caught (except ones documented as genuinely
# semantics-preserving).
set -uo pipefail

CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$CRATE/src/lib.rs"
mkdir -p "$CRATE/target/mutlogs"
BAK="$CRATE/target/mutlogs/lib.rs.orig"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

FAIL=0

# name | sed expression | expect: CAUGHT or EQUIVALENT
run_mutation() {
  local name="$1" expr="$2" expect="$3"
  restore
  sed -i "$expr" "$SRC" || { echo "  [SKIP] $name (sed failed)"; return; }
  if cmp -s "$SRC" "$BAK"; then
    echo "  [SKIP] $name (mutation did not change the source)"
    return
  fi
  ( cd "$CRATE" && timeout 300 cargo build --release >/dev/null 2>&1 ) || {
    echo "  [SKIP] $name (mutant does not compile)"; return; }
  if ( cd "$CRATE" && timeout 600 cargo test --release >"$CRATE"/target/mutlogs/mut.log 2>&1 ); then
    got=SURVIVED
  else
    got=CAUGHT
  fi
  if [ "$expect" = CAUGHT ] && [ "$got" = CAUGHT ]; then
    echo "  [ok]   $name -> CAUGHT by: $(grep -o '^test [a-z0-9_]* \.\.\. FAILED' "$CRATE"/target/mutlogs/mut.log | sed 's/^test //;s/ \.\.\..*//' | tr '\n' ' ')"
  elif [ "$expect" = EQUIVALENT ] && [ "$got" = SURVIVED ]; then
    echo "  [ok]   $name -> survived, as expected (semantics-preserving)"
  else
    echo "  [FAIL] $name -> expected $expect, got $got"
    FAIL=1
  fi
}

echo "=== mutation testing the differential suite ==="

run_mutation "floor division instead of C truncation" \
  's|let hsize: c_int = size / 2;|let hsize: c_int = size.div_euclid(2);|' CAUGHT

run_mutation "exclusive loop bound (r < hsize)" \
  's|while r <= hsize {|while r < hsize {|' CAUGHT

run_mutation "clamp else-arm yields -0.0" \
  's|v = if v > 0.0f32 { v } else { 0.0f32 };|v = if v > 0.0f32 { v } else { -0.0f32 };|' CAUGHT

run_mutation "normalise when sum >= 0 instead of > 0" \
  's|if sum > 0.0f32 {|if sum >= 0.0f32 {|' CAUGHT

run_mutation "normalise the full stored range incl. the overrun element" \
  's|while r < size {|while r < size + 1 {|' CAUGHT

run_mutation "defensive null check (survives where C faults)" \
  's|let sigma: f32 = 1.6f32;|if dest.is_null() { return; } let sigma: f32 = 1.6f32;|' CAUGHT

# `f32::exp()` is lowered by rustc to a call to the very same
# `expf@GLIBC_2.27` (verified with `nm -D` on the mutant .so), so this is a
# genuinely semantics-preserving rewrite rather than a gap in the suite.
run_mutation "std f32::exp instead of libm expf (lowers to the same expf)" \
  's|unsafe { expf(x \* x) }|(x * x).exp()|' EQUIVALENT

run_mutation "divide by sum instead of multiplying by its reciprocal" \
  's|\*p \*= isum;|*p /= sum;|' CAUGHT

run_mutation "f64 intermediate precision" \
  's|let mut v: f32 = (1.0f32 / unsafe { expf(x \* x) }) - s2;|let mut v: f32 = ((1.0f64 / (unsafe { expf(x * x) } as f64)) - s2 as f64) as f32;|' CAUGHT

run_mutation "sigma typo (1.6 -> 1.60001)" \
  's|let sigma: f32 = 1.6f32;|let sigma: f32 = 1.60001f32;|' CAUGHT

run_mutation "tetha typo (2.25 -> 2.2499)" \
  's|let tetha: f32 = 2.25f32;|let tetha: f32 = 2.2499f32;|' CAUGHT

run_mutation "pointer bump via offset instead of add" \
  's|k = unsafe { k.add(1) };|k = unsafe { k.offset(1) };|' EQUIVALENT

run_mutation "clamp with >= instead of > (v is never -0.0, so equivalent)" \
  's|v = if v > 0.0f32 { v } else { 0.0f32 };|v = if v >= 0.0f32 { v } else { 0.0f32 };|' EQUIVALENT

restore
( cd "$CRATE" && cargo build --release >/dev/null 2>&1 )
echo
if [ "$FAIL" -eq 0 ]; then echo "MUTATION TESTING PASSED (suite has detection power)"; else echo "MUTATION TESTING FAILED"; fi
exit "$FAIL"
