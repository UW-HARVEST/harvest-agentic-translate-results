#!/usr/bin/env bash
# Mutation battery: injects a known-wrong change into translation/src/lib.rs,
# rebuilds the cdylib and runs the whole differential suite. A mutant that
# SURVIVES (0 failing tests) is a hole in the test suite, unless it is provably
# equivalent to the original.
#
# Usage: ./scripts/mutation_battery.sh
set -uo pipefail
cd "$(dirname "$0")/.."

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
trap 'cp "$ORIG" src/lib.rs; cargo build --release -q' EXIT

apply() {
  python3 - "$1" "$2" <<'PY'
import sys
p = 'src/lib.rs'
s = open(p).read()
a, b = sys.argv[1], sys.argv[2]
if s.count(a) < 1:
    sys.exit("PATTERN NOT FOUND: " + a)
open(p, 'w').write(s.replace(a, b, 1))
PY
}

survivors=0
total=0
run() {
  local desc="$1" from="$2" to="$3"
  total=$((total + 1))
  cp "$ORIG" src/lib.rs
  if ! apply "$from" "$to"; then
    echo "SKIP    | $desc (pattern missing)"
    return
  fi
  if ! cargo build --release -q 2>/dev/null; then
    echo "SKIP    | $desc (does not compile)"
    cp "$ORIG" src/lib.rs
    return
  fi
  local n
  n=$(timeout 900 cargo test --release -q 2>&1 | grep -oE '[0-9]+ failed' | awk '{s+=$1} END{print s+0}')
  if [ "${n:-0}" -eq 0 ]; then
    echo "SURVIVED| $desc"
    survivors=$((survivors + 1))
  else
    echo "killed  | $desc ($n failing tests)"
  fi
  cp "$ORIG" src/lib.rs
}

echo "=== constants ==="
run "FLT_MAX seed -> 1e30"                  'let mut d0 = C2_FLT_MAX;' 'let mut d0 = 1.0e30f32;'
run "FLT_EPSILON 1e-7 -> 1e-6"              'C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32' 'C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-6_f32'
run "FLT_EPSILON 1e-7 -> 1e-8"              'C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32' 'C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-8_f32'
run "cache metric floor -1e8 -> -1e7"       'metric < -1.0e8' 'metric < -1.0e7'
run "cache metric floor -1e8 -> -1e9"       'metric < -1.0e8' 'metric < -1.0e9'
run "cache ratio 2.0 -> 1.5"                'max_metric * 2.0' 'max_metric * 1.5'
run "cache ratio 2.0 -> 3.0"                'max_metric * 2.0' 'max_metric * 3.0'
run "midpoint 0.5 -> 0.4"                   'c2Mulvs(c2Add(a, b), 0.5)' 'c2Mulvs(c2Add(a, b), 0.4)'
run "iter cap 20 -> 7 (measured max)"       'while iter < 20 {' 'while iter < 7 {'
run "iter cap 20 -> 8 (unreachable)"        'while iter < 20 {' 'while iter < 8 {'
run "iter cap 20 -> 19 (unreachable)"       'while iter < 20 {' 'while iter < 19 {'
run "eps^2 -> eps"                          'if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {' 'if c2Dot(d, d) < C2_FLT_EPSILON {'

echo "=== comparison operators ==="
run "d1>d0 -> d1>=d0"                       '            if d1 > d0 {' '            if d1 >= d0 {'
run "support > -> >="                       '            if dot > dmax {' '            if dot >= dmax {'
run "radius dist> -> dist>="                'if dist > rA + rB && dist > C2_FLT_EPSILON {' 'if dist >= rA + rB && dist > C2_FLT_EPSILON {'
run "radius eps> -> eps>="                  'if dist > rA + rB && dist > C2_FLT_EPSILON {' 'if dist > rA + rB && dist >= C2_FLT_EPSILON {'
run "cache good !=0 -> >0"                  'let cache_was_good = (*cache).count != 0;' 'let cache_was_good = (*cache).count > 0;'
run "c22 v<=0 -> v<0"                       '        if v <= 0.0 {' '        if v < 0.0 {'
run "c22 u<=0 -> u<0"                       '        } else if u <= 0.0 {' '        } else if u < 0.0 {'
run "c2D det>0 -> det>=0"                   'if c2Det2(ab, c2Neg(s.verts[0].p)) > 0.0 {' 'if c2Det2(ab, c2Neg(s.verts[0].p)) >= 0.0 {'
run "min<max*2 -> min<=max*2"               'if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {' 'if !(min_metric <= max_metric * 2.0 && metric < -1.0e8) {'
run "c23 wABC<=0 -> wABC<0"                 'uAB > 0.0 && vAB > 0.0 && wABC <= 0.0' 'uAB > 0.0 && vAB > 0.0 && wABC < 0.0'
run "c23 uABC<=0 -> uABC<0"                 'uBC > 0.0 && vBC > 0.0 && uABC <= 0.0' 'uBC > 0.0 && vBC > 0.0 && uABC < 0.0'
run "c23 vABC<=0 -> vABC<0"                 'uCA > 0.0 && vCA > 0.0 && vABC <= 0.0' 'uCA > 0.0 && vCA > 0.0 && vABC < 0.0'
run "c23 arm1 vAB<=0 -> vAB<0"              'if vAB <= 0.0 && uCA <= 0.0 {' 'if vAB < 0.0 && uCA <= 0.0 {'
run "c23 arm2 uAB<=0 -> uAB<0"              '} else if uAB <= 0.0 && vBC <= 0.0 {' '} else if uAB < 0.0 && vBC <= 0.0 {'
run "c23 arm3 uBC<=0 -> uBC<0"              '} else if uBC <= 0.0 && vCA <= 0.0 {' '} else if uBC < 0.0 && vCA <= 0.0 {'
run "post-shrink a==b test dropped"         'if a.x == b.x && a.y == b.y {' 'if a.x == b.x && a.y == b.y && false {'

echo "=== arithmetic / arm bodies ==="
run "c2Maxv NaN semantics flipped"          'if a.x > b.x { a.x } else { b.x },' 'if b.x < a.x { a.x } else { b.x },'
run "c2Minv NaN semantics flipped"          'if a.x < b.x { a.x } else { b.x },' 'if b.x > a.x { a.x } else { b.x },'
run "c2MulrvT sign"                         'c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)' 'c2V(a.c * b.x + a.s * b.y, a.s * b.x + a.c * b.y)'
run "c2Mulrv sign"                          'c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)' 'c2V(a.c * b.x + a.s * b.y, a.s * b.x + a.c * b.y)'
run "c2Det2 operand swap"                   'a.x * b.y - a.y * b.x' 'a.y * b.x - a.x * b.y'
run "c2Dot term order (FP-visible)"         'a.x * b.x + a.y * b.y' 'a.y * b.y + a.x * b.x'
run "c2Skew/CCW90 swapped"                  'pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }' 'pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }'
run "c2Norm uses div by dot not len"        'c2Div(a, c2Len(a))' 'c2Div(a, c2Dot(a, a))'
run "c2Witness den = div (not 1/div)"       '        let den = 1.0f32 / s.div;
        match s.count {
            1 => {' '        let den = s.div;
        match s.count {
            1 => {'
run "c2L den = div (not 1/div)"             '        let den = 1.0f32 / s.div;
        match s.count {
            1 => s.verts[0].p,' '        let den = s.div;
        match s.count {
            1 => s.verts[0].p,'
run "c23 uCA arm shuffle order"             '            s.verts[1] = s.verts[0];
            s.verts[0] = s.verts[2];' '            s.verts[0] = s.verts[2];
            s.verts[1] = s.verts[0];'
run "c23 uBC arm shuffle order"             '            s.verts[0] = s.verts[1];
            s.verts[1] = s.verts[2];' '            s.verts[1] = s.verts[2];
            s.verts[0] = s.verts[1];'
run "c2BBVerts corner 1/3 swap"             '        *out.add(1) = c2V(bb.max.x, bb.min.y);' '        *out.add(1) = c2V(bb.min.x, bb.max.y);'
run "c2MakeProxy AABB radius 0 -> input r"  '                p.radius = 0.0;
                p.count = 4;' '                p.radius = (*(shape as *const c2Capsule)).r;
                p.count = 4;'
run "c2MakeProxy capsule verts swapped"     '                p.verts[0] = c.a;
                p.verts[1] = c.b;' '                p.verts[0] = c.b;
                p.verts[1] = c.a;'
run "c2GJK hit sets b=a (not a=b)"          '        if hit != 0 {
            a = b;' '        if hit != 0 {
            b = a;'
run "c2GJK support dir A not negated"       'c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)))' 'c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, d))'
run "c2GJK p = sA - sB (append path)"       '            (*v).p = c2Sub((*v).sB, (*v).sA);

            let mut dup = 0;' '            (*v).p = c2Sub((*v).sA, (*v).sB);

            let mut dup = 0;'
run "c2GJK p = sA - sB (cache path)"        '                    (*v).p = c2Sub((*v).sB, (*v).sA);
                    (*v).u = 0.0;' '                    (*v).p = c2Sub((*v).sA, (*v).sB);
                    (*v).u = 0.0;'
run "c2GJK dup checks iA only"              'if iA == *saveA.as_ptr().offset(i as isize)
                    && iB == *saveB.as_ptr().offset(i as isize)' 'if iA == *saveA.as_ptr().offset(i as isize)'
run "c2GJK cache seeds u=1 not 0"           '                    (*v).u = 0.0;' '                    (*v).u = 1.0;'
run "c2GJK fresh simplex div 1 -> 0"        '            s.verts[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;' '            s.verts[0].u = 1.0;
            s.div = 0.0;
            s.count = 1;'
run "gjk_cache reverse branch swapped"      '        if reverse != 0 {' '        if reverse == 0 {'
run "gjk_cache skips the warm-cache call"   '        let d1 = c2GJK(' '        let d1 = 0.0f32; let _unused = c2GJK('
run "c2Len uses dot (no sqrt)"              'pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()' 'pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a)'
run "metric count2 operand order (equiv)"   '            2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),' '            2 => c2Len(c2Sub(s.verts[0].p, s.verts[1].p)),'
run "metric count3 det arg order"           '            3 => c2Det2(
                c2Sub(s.verts[1].p, s.verts[0].p),
                c2Sub(s.verts[2].p, s.verts[0].p),
            ),' '            3 => c2Det2(
                c2Sub(s.verts[2].p, s.verts[0].p),
                c2Sub(s.verts[1].p, s.verts[0].p),
            ),'

echo
echo "=============================================="
echo "mutants run: $total   SURVIVORS: $survivors"
echo "=============================================="
