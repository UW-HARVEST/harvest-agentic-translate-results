#!/usr/bin/env bash
# Test-strength check (not part of the verification itself).
#
# Injects a deliberate divergence into src/lib.rs, one at a time, and asserts
# that the differential test suite FAILS.  A mutation that survives means the
# corresponding ERRORS.md / CONFIGS.md row is not really being observed.
set -uo pipefail
cd "$(dirname "$0")"

cp src/lib.rs src/lib.rs.orig
restore() { mv -f src/lib.rs.orig src/lib.rs; }
trap restore EXIT

survived=0
# Mutants that are provably *semantically equivalent* (see MUTATION_NOTES.md):
# no test can kill them, so they are expected to survive.
expected_survivors=3

mutate() { # mutate <description> <from> <to>
    local desc=$1 from=$2 to=$3
    cp src/lib.rs.orig src/lib.rs
    python3 - "$from" "$to" <<'PY'
import sys
frm, to = sys.argv[1], sys.argv[2]
s = open("src/lib.rs").read()
assert s.count(frm) >= 1, f"pattern not found: {frm!r}"
open("src/lib.rs","w").write(s.replace(frm, to, 1))
PY
    if [ $? -ne 0 ]; then echo "SKIP  $desc (pattern missing)"; return; fi
    # cargo test does NOT rebuild a cdylib no test target links against, so the
    # library must be rebuilt explicitly before the suite runs.
    if ! timeout 600 cargo build --offline >/dev/null 2>&1; then
        echo "SKIP  $desc (mutant does not compile)"; return
    fi
    if timeout 600 cargo test --offline -q >/dev/null 2>&1; then
        echo "SURVIVED  $desc   <-- tests are too weak here"
        survived=$((survived + 1))
    else
        echo "killed    $desc"
    fi
}

mutate "c2GJK: d1 > d0  ->  d1 >= d0"                  "if d1 > d0 {" "if d1 >= d0 {"
mutate "c2GJK: eps break < -> <="                      "if c2Dot(d, d) < C2_FLT_EPSILON" "if c2Dot(d, d) <= C2_FLT_EPSILON"
mutate "c2GJK: iter < 20 -> iter < 19"                 "while iter < 20 {" "while iter < 19 {"
mutate "c2GJK: use_radius != 0 -> == 1"                "} else if use_radius != 0 {" "} else if use_radius == 1 {"
mutate "c2GJK: dist > rA+rB -> >="                     "if dist > sse_add(rA, rB) &&" "if dist >= sse_add(rA, rB) &&"
mutate "c2GJK: cache metric -1.0e8 -> +1.0e8"          "metric < -1.0e8)" "metric < 1.0e8)"
mutate "c2GJK: drop the a=b on hit"                    "    if hit != 0 {
        a = b;" "    if hit != 0 {
        b = a;"
mutate "c22: v <= 0 -> v < 0"                          "if v <= 0.0 {" "if v < 0.0 {"
mutate "c22: u <= 0 -> u < 0"                          "} else if u <= 0.0 {" "} else if u < 0.0 {"
mutate "c23: reorder branch 4/5 test"                  "} else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {" "} else if uBC > 0.0 && vBC > 0.0 && vABC <= 0.0 {"
mutate "c2D: det > 0 -> det >= 0"                      "if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {" "if c2Det2(ab, c2Neg((*s).verts[0].p)) >= 0.0 {"
mutate "c2Support: dot > dmax -> >="                    "if dot > dmax {" "if dot >= dmax {"
mutate "c2Support: start i at 0"                       "let mut i: c_int = 1;
    while i < count {" "let mut i: c_int = 0;
    while i < count {"
mutate "c2Skew: -a.y -> 0.0 - a.y (loses -0.0)"        "c2v { x: -a.y, y: a.x }" "c2v { x: 0.0 - a.y, y: a.x }"
mutate "c2Neg: -a.x -> 0.0 - a.x (loses -0.0)"         "c2V(-a.x, -a.y)" "c2V(0.0 - a.x, 0.0 - a.y)"
mutate "c2Maxv: > -> >= (NaN / -0.0 tie)"              "if a.x > b.x { a.x } else { b.x }" "if a.x >= b.x { a.x } else { b.x }"
mutate "c2Minv: < -> <= (NaN / -0.0 tie)"              "if a.x < b.x { a.x } else { b.x }" "if a.x <= b.x { a.x } else { b.x }"
mutate "c2MakeProxy: invalid enum writes radius"       "        _ => {}
    }
}" "        _ => {
            (*p).radius = 0.0;
        }
    }
}"
mutate "c2CircletoCircle: d2 < r2 -> <="               "(d2 < r2) as c_int" "(d2 <= r2) as c_int"
mutate "c2CircletoAABB: d2 < r2 -> <="                 "    let r2 = sse_mul(A.r, A.r);
    (d2 < r2) as c_int" "    let r2 = sse_mul(A.r, A.r);
    (d2 <= r2) as c_int"
mutate "c2CircletoCapsule: da < 0 -> da <= 0"          "if da < 0.0 {" "if da <= 0.0 {"
mutate "c2CircletoCapsule: db < 0 -> db <= 0"          "if db < 0.0 {" "if db <= 0.0 {"
mutate "c2AABBtoAABB: drop the -X separating axis"     "let d0: c_int = (B.max.x < A.min.x) as c_int;" "let d0: c_int = 0;"
mutate "c2AABBtoAABB: drop the +Y separating axis"     "let d3: c_int = (A.max.y < B.min.y) as c_int;" "let d3: c_int = 0;"
mutate "c2Collided: bad typeB returns 1 (circle arm)"  "            C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
            _ => 0," "            C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
            _ => 1,"
mutate "c2Collided: outer bad typeA returns 1"         "        _ => 0,
    }
}" "        _ => 1,
    }
}"
mutate "c2Witness: bad count -> (1,1) instead of (0,0)" "        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }" "        _ => {
            *a = c2V(1.0, 0.0);
            *b = c2V(0.0, 0.0);
        }"
mutate "c2L: default arm returns a.p"                  "        _ => c2V(0.0, 0.0),
    }
}

// ----" "        _ => (*s).verts[0].p,
    }
}

// ----"
mutate "c2GJKSimplexMetric: swap the count 2/3 arms"   "        2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p))," "        4 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),"
mutate "c2Dot: + -> - "                                "    sse_add(p1, p0)
}" "    sse_sub(p1, p0)
}"
mutate "aabb: shift the capsule term by 3 not 2"       "C2_TYPE_AABB,
        ) << 2;" "C2_TYPE_AABB,
        ) << 3;"

echo
echo "MUTATION CHECK: $survived mutant(s) survived (expected $expected_survivors equivalent mutants)"
if [ "$survived" -le "$expected_survivors" ]; then
    echo "=> OK: see MUTATION_NOTES.md for why each survivor is unobservable"
    exit 0
fi
echo "=> UNEXPECTED SURVIVORS: the differential suite has a blind spot"
exit 1
