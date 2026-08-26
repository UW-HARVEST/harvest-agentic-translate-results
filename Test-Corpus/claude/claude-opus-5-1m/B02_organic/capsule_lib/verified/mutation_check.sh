#!/usr/bin/env bash
# Sanity-checks the differential suite itself: inject a series of small,
# behaviour-changing mutations into src/lib.rs (each one a plausible translation
# slip) and require that the tests FAIL for every single one. A suite that
# passes a mutation cannot be trusted to have verified that code path.
#
# src/lib.rs is restored from a backup after every mutation, and at exit.
set -uo pipefail
cd "$(dirname "$0")"
BK="$(mktemp)"
cp src/lib.rs "$BK"
restore() { cp "$BK" src/lib.rs; }
trap 'restore; rm -f "$BK"' EXIT

pass=0
fail=0

# $1 = description, $2 = python search string, $3 = python replacement
try() {
  local desc="$1" from="$2" to="$3"
  restore
  python3 - "$from" "$to" <<'PY'
import sys
p = 'src/lib.rs'
s = open(p).read()
frm, to = sys.argv[1], sys.argv[2]
n = s.count(frm)
if n != 1:
    sys.exit(f'anchor appears {n} times, need exactly 1: {frm[:70]!r}')
open(p, 'w').write(s.replace(frm, to))
PY
  if [ $? -ne 0 ]; then
    printf '  ?? SKIP  %s (anchor not unique)\n' "$desc"
    fail=$((fail + 1))
    return
  fi
  # A mutation counts as caught if cargo reports a non-zero exit, which covers
  # both "N tests failed" and "the test process aborted" (a mutant that
  # dereferences NULL makes the harness abort rather than fail a comparison).
  local out rc n
  out=$(timeout 600 cargo test --offline --no-fail-fast 2>&1)
  rc=$?
  n=$(printf '%s' "$out" | grep -oE '[0-9]+ failed' | grep -oE '^[0-9]+' \
        | awk '{t+=$1} END{print t+0}')
  if printf '%s' "$out" | grep -q '^error\[\|^error: could not compile'; then
    printf '  ??  BUILD BROKEN (not a behavioural test): %s\n' "$desc"
    fail=$((fail + 1))
  elif [ "$rc" -ne 0 ]; then
    if printf '%s' "$out" | grep -qE 'SIGABRT|SIGSEGV|non-unwinding panic'; then
      printf '  OK   caught (harness aborted: %s test target(s) crashed): %s\n' \
        "$(printf '%s' "$out" | grep -c 'process didn.t exit successfully')" "$desc"
    else
      printf '  OK   caught by %s test(s): %s\n' "$n" "$desc"
    fi
    pass=$((pass + 1))
  else
    printf '  FAIL NOT CAUGHT: %s\n' "$desc"
    fail=$((fail + 1))
  fi
}

echo "mutation battery (every mutation must be caught):"

try 'c2Skew returns the c2CCW90 rotation' \
    '    b.x = -a.y;
    b.y = a.x;' \
    '    b.x = a.y;
    b.y = -a.x;'

try 'c2Det2 sign flipped' \
    'a.x * b.y - a.y * b.x' \
    'a.y * b.x - a.x * b.y'

try 'c2CircletoCircle uses <= instead of < (touching counts as a hit)' \
    '    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB' \
    '    (d2 <= r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB'

try 'c2AABBtoAABB uses <= (separated boxes count as touching)' \
    'let d0 = (B.max.x < A.min.x) as c_int;' \
    'let d0 = (B.max.x <= A.min.x) as c_int;'

try 'c2Collided AABBxCIRCLE operands un-swapped ("fixing" the C quirk)' \
    'C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),' \
    'C2_TYPE_CIRCLE => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),'

try 'c2GJK no-progress guard uses >= instead of >' \
    '            if d1 > d0 {
                break;
            }' \
    '            if d1 >= d0 {
                break;
            }'

# NOTE: `while iter < 20` -> `while iter < 19` is a provably EQUIVALENT mutant:
# the highest reachable iteration count is 5 (see ERRORS.md row E29), so any cap
# above 5 is unobservable. A cap that is actually reachable must be detected, and
# is:
try 'c2GJK iteration cap lowered into the reachable range (20 -> 2)' \
    'while iter < 20 {' \
    'while iter < 2 {'

try 'c2GJK radius guard uses >= instead of >' \
    'if dist > rA + rB && dist > C2_FLT_EPSILON {' \
    'if dist >= rA + rB && dist > C2_FLT_EPSILON {'

try 'c2GJK cache metric guard drops the -1e8 clause' \
    'if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {' \
    'if !(min_metric < max_metric * 2.0f32) {'

try 'c2GJK writes the cache even when the pointer is NULL-checked differently' \
    '        if !outA.is_null() {
            *outA = a;
        }' \
    '        if !outB.is_null() {
            *outA = a;
        }'

try 'c22 collapse branches swapped' \
    '    if v <= 0.0 {
        s.verts[0].u = 1.0f32;' \
    '    if u <= 0.0 {
        s.verts[0].u = 1.0f32;'

try 'c23 edge branch uses the wrong barycentric sign' \
    '} else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {' \
    '} else if uAB > 0.0 && vAB > 0.0 && wABC < 0.0 {'

try 'c2Support breaks ties towards the last index (>= instead of >)' \
    '            if dot > dmax {' \
    '            if dot >= dmax {'

try 'c2MakeProxy gains a default arm (C has none)' \
    '            // No `default` label in the C switch: nothing happens.
            _ => {}' \
    '            _ => {
                p.radius = 0.0;
                p.count = 0;
            }'

try 'c2BBVerts vertex order rotated' \
    '        *out.add(1) = c2V(bb.max.x, bb.min.y);' \
    '        *out.add(1) = c2V(bb.min.x, bb.max.y);'

try 'c2Witness omits the third barycentric term' \
    '                    c2Mulvs(s.c().sA, den * s.c().u),
                );' \
    '                    c2Mulvs(s.c().sA, 0.0),
                );'

# NOTE: in c2CircletoCapsule, `da < 0` -> `da <= 0` (and likewise for `db`) is
# a provably EQUIVALENT mutant: the three distance formulas agree exactly on the
# branch boundaries. At da == 0 the perpendicular branch computes
# e = ap - n*(0/|n|^2) = ap, i.e. the same value as the `da < 0` branch; and the
# only way to reach da == 0 with db >= 0 is |n|^2 == 0, i.e. a degenerate capsule
# where ap == bp anyway. So the branch choice must be perturbed by changing the
# formula, not the comparison:
try 'c2CircletoCapsule measures to the wrong endpoint on the da < 0 branch' \
    '    if da < 0.0 {
        d2 = c2Dot(ap, ap);' \
    '    if da < 0.0 {
        d2 = c2Dot(ap, ap) + 1.0;'

try 'capsule() reference circle radius 20 -> 21' \
    'circle.r = 20.0f32;' \
    'circle.r = 21.0f32;'

try 'capsule() shifts the second result bit' \
    ') << 1;' \
    ') << 3;'

try 'c2Norm normalises by the squared length' \
    'c2Div(a, c2Len(a))' \
    'c2Div(a, c2Dot(a, a))'

restore
printf '\nmutations caught: %d, NOT caught: %d\n' "$pass" "$fail"
[ "$fail" -eq 0 ] && echo "MUTATION BATTERY PASSED" || echo "MUTATION BATTERY FAILED"
exit "$fail"
