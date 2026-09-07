#!/usr/bin/env bash
# Sensitivity check for the differential test suite.
#
# Injects one deliberate bug at a time into src/lib.rs, rebuilds the Rust .so
# and runs the whole suite. Every mutant MUST be killed (tests must fail); a
# surviving mutant means the suite has a blind spot.
set -u
cd "$(dirname "$0")"

ORIG=$(mktemp)
cp src/lib.rs "$ORIG"
restore() { cp "$ORIG" src/lib.rs; }
# Always leave the tree with the pristine source AND a matching fresh .so, so
# the staleness guard in tests/common/mod.rs does not trip afterwards.
trap 'restore; rm -f "$ORIG"; cargo build --release --offline >/dev/null 2>&1' EXIT

declare -a NAMES=()
declare -a FROMS=()
declare -a TOS=()
add() { NAMES+=("$1"); FROMS+=("$2"); TOS+=("$3"); }

add "c2Det2 sign flip"            'a.x * b.y - a.y * b.x' 'a.y * b.x - a.x * b.y'
add "c2Dot swap"                  'a.x * b.x + a.y * b.y' 'a.x * b.y + a.y * b.x'
add "c22 boundary <= -> <"        'if v <= 0.0 {' 'if v < 0.0 {'
add "c23 boundary <= -> <"        'if vAB <= 0.0 && uCA <= 0.0 {' 'if vAB < 0.0 && uCA <= 0.0 {'
add "c2Support tie >= "           'if dot > dmax {' 'if dot >= dmax {'
add "c2Support start index"       'let mut i: c_int = 1;' 'let mut i: c_int = 0;'
add "GJK iteration cap 20->19"    'while iter < 20 {' 'while iter < 19 {'
add "GJK cache metric guard"      'metric < -1.0e8f32' 'metric > -1.0e8f32'
add "GJK epsilon guard"           'c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON' 'c2Dot(d, d) < C2_FLT_EPSILON'
add "GJK d1>d0 -> d1>=d0"         'if d1 > d0 {' 'if d1 >= d0 {'
add "GJK radius guard"            'if dist > rA + rB && dist > C2_FLT_EPSILON {' 'if dist >= rA + rB && dist > C2_FLT_EPSILON {'
add "CircletoCircle < -> <="      '(d2 < r2) as c_int' '(d2 <= r2) as c_int'
add "CircletoCapsule da guard"    'if da < 0.0 {' 'if da <= 0.0 {'
add "AABBtoAABB negation"         '((d0 | d1 | d2 | d3) == 0) as c_int' '((d0 | d1 | d2 | d3) != 0) as c_int'
add "Collided AABBxCIRCLE swap"   'C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),' 'C2_TYPE_CIRCLE => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),'
add "MakeProxy default zeroes"    '        _ => {}
    }
}' '        _ => { (*p).radius = 0.0; (*p).count = 0; }
    }
}'
add "MakeProxy capsule count"     '(*p).count = 2;' '(*p).count = 1;'
add "BBVerts vertex order"        '*out.offset(1) = c2V((*bb).max.x, (*bb).min.y);' '*out.offset(1) = c2V((*bb).min.x, (*bb).max.y);'
add "c2L default -> a.p"          '        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT' '        _ => (*verts.offset(0)).p,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT'
add "c2Norm zero guard added"     'c2Div(a, c2Len(a))' 'if a.x == 0.0 && a.y == 0.0 { a } else { c2Div(a, c2Len(a)) }'
add "SimplexMetric count 1 -> 2"  '        2 => c2Len(c2Sub((*verts.offset(1)).p, (*verts.offset(0)).p)),' '        1 => c2Len(c2Sub((*verts.offset(1)).p, (*verts.offset(0)).p)),'
add "Witness den sign"            'let den = 1.0f32 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*verts.offset(0)).sA;' 'let den = -1.0f32 / (*s).div;
    match (*s).count {
        1 => {
            *a = (*verts.offset(0)).sA;'
add "reverse_collide shift"       ') << 2,' ') << 3,'
add "Clampv order"                'c2Maxv(lo, c2Minv(a, hi))' 'c2Minv(hi, c2Maxv(a, lo))'
add "Skew sign"                   'b.x = -a.y;
    b.y = a.x;' 'b.x = a.y;
    b.y = -a.x;'
add "GJK use_radius test"         'if hit != 0 {' 'if hit != 0 || use_radius == 0 {'
add "GJK dup break removed"       'if dup != 0 {
            break;
        }' 'if dup != 0 && false {
            break;
        }'
add "GJK cache_was_read invert"   'if cache_was_read == 0 {' 'if cache_was_read != 0 {'

killed=0
survived=0
for i in "${!NAMES[@]}"; do
  restore
  python3 - "${FROMS[$i]}" "${TOS[$i]}" <<'PY'
import sys, pathlib
frm, to = sys.argv[1], sys.argv[2]
p = pathlib.Path("src/lib.rs")
s = p.read_text()
if frm not in s:
    sys.exit(2)
p.write_text(s.replace(frm, to, 1))
PY
  rc=$?
  if [ $rc -eq 2 ]; then
    echo "SKIP (pattern not found): ${NAMES[$i]}"
    continue
  fi
  if ! cargo build --release --offline >/dev/null 2>&1; then
    echo "SKIP (mutant does not compile): ${NAMES[$i]}"
    continue
  fi
  out=$(RUST_SO="$PWD/target/release/libreverse_collide_lib.so" \
        timeout 600 cargo test --offline 2>&1)
  if echo "$out" | grep -q "test result: FAILED"; then
    n=$(echo "$out" | grep -c '^test .* FAILED$')
    echo "KILLED   ($n test(s)): ${NAMES[$i]}"
    killed=$((killed+1))
  else
    echo "SURVIVED  <<< BLIND SPOT: ${NAMES[$i]}"
    survived=$((survived+1))
  fi
done

restore
cargo build --release --offline >/dev/null 2>&1
echo
echo "mutants killed: $killed, survived: $survived"
[ "$survived" -eq 0 ]
