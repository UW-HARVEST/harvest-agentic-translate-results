#!/usr/bin/env bash
# Mutation sanity check: proves the differential suite is not vacuous.
#
# Each mutant introduces ONE behavioural change into translation/src/lib.rs,
# rebuilds the cdylib, and re-runs the whole suite. A mutant that the suite fails
# to detect is either a real test gap or a provably-equivalent mutant (documented
# in VERIFICATION.md). Nothing in c_src/ is touched.
set -uo pipefail
cd "$(dirname "$0")/.."          # translation/

ORIG=$(mktemp); cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; cargo build -q 2>/dev/null; }
trap restore EXIT

fail=0
mutate() {
  local name="$1"; shift
  cp "$ORIG" src/lib.rs
  "$@"
  if ! cargo build -q 2>/dev/null; then
    echo "  [$name] SKIP (mutant does not compile)"
    return
  fi
  # `cargo test` exits non-zero on assertion failures AND on a SIGABRT/SIGSEGV
  # from the loaded cdylib, both of which count as detection.
  local out; out=$(timeout 300 cargo test 2>&1)
  if [ $? -ne 0 ]; then
    local who
    who=$(printf '%s\n' "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' \
          | sed 's/^test //; s/ \.\.\. FAILED//' | paste -sd' ' -)
    [ -z "$who" ] && who="(process aborted: $(printf '%s\n' "$out" | grep -oE 'signal: [0-9]+, [A-Z]+' | head -1))"
    echo "  [$name] DETECTED by: $who"
  else
    echo "  [$name] *** SURVIVED ***"
    fail=$((fail+1))
  fi
}

s() { sed -i "$1" src/lib.rs; }

echo "== mutation sanity check =="
mutate c2dot-addss-order        s 's/^    addss(q, p)$/    addss(p, q)/'
mutate c2dot-mulss-order        s 's/mulss(b.y, a.y)/mulss(a.y, b.y)/'
mutate aabb-lt-to-le            s 's/(B.max.x < A.min.x) as c_int/(B.max.x <= A.min.x) as c_int/'
mutate aabb-drop-d3             s 's/(d0 | d1 | d2 | d3)/(d0 | d1 | d2)/'
mutate circle-lt-to-le          s 's/(d2 < r2) as c_int$/(d2 <= r2) as c_int/'
mutate maxv-uses-f32-max        s 's/if a.x > b.x { a.x } else { b.x }/a.x.max(b.x)/; s/if a.y > b.y { a.y } else { b.y }/a.y.max(b.y)/'
mutate minv-uses-f32-min        s 's/if a.x < b.x { a.x } else { b.x }/a.x.min(b.x)/; s/if a.y < b.y { a.y } else { b.y }/a.y.min(b.y)/'
mutate clampv-swap-lo-hi        s 's/c2Maxv(lo, c2Minv(a, hi))/c2Minv(hi, c2Maxv(a, lo))/'
mutate no-snan-quieting         s 's/x.to_bits() | 0x0040_0000/x.to_bits()/'
mutate wrong-qnan-indefinite    s 's/0xFFC0_0000/0x7FC0_0000/'
mutate sub-arg-order            s 's/a.x = subss(a.x, b.x);/a.x = subss(b.x, a.x);/'
mutate unswap-aabb-circle       s 's/C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { \*(B as \*const c2Circle) }/C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { *(A as *const c2Circle) }/'
# The outer `default:` arm dispatches instead of rejecting.
mutate outer-default-dispatches s 's/^        _ => 0,$/        _ => c2AABBtoAABB(unsafe { *(A as *const c2AABB) }, unsafe { *(B as *const c2AABB) }),/'
# The inner `default:` arms dispatch instead of rejecting.
mutate inner-default-dispatches s 's/^            _ => 0,$/            _ => c2AABBtoAABB(unsafe { *(A as *const c2AABB) }, unsafe { *(B as *const c2AABB) }),/'
# An out-of-range tag is folded onto a valid variant.
mutate tag-normalised           s 's/    match typeA {/    let typeA = typeA \& 1; let typeB = typeB \& 1; match typeA {/'

echo
if [ "$fail" -eq 0 ]; then
  echo "RESULT: all mutants detected."
else
  echo "RESULT: $fail mutant(s) SURVIVED — investigate before trusting the suite."
fi
exit "$fail"
