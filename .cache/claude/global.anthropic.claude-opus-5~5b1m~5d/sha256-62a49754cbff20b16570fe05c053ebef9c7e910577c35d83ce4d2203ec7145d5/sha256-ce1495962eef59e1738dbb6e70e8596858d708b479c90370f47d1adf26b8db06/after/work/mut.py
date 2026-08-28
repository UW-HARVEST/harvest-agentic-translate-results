import subprocess, sys, os, json
R="$HARVEST_WORKDIR"
SRC=R+"/translation/src/lib.rs"
BAK=R+"/work/lib.rs.bak"
os.chdir(R+"/translation")
orig=open(BAK).read()

M=[
("c2V-swap","    a.x = x;\n    a.y = y;","    a.x = y;\n    a.y = x;"),
("c2Mulvs-y","    a.y *= b;","    a.y *= b + 0.0000001;"),
("c2Maxv-x","if a.x > b.x { a.x } else { b.x }","if a.x >= b.x { a.x } else { b.x }"),
("c2Minv-y","if a.y < b.y { a.y } else { b.y }","if a.y <= b.y { a.y } else { b.y }"),
("c2Clampv-swap","    c2Maxv(lo, c2Minv(a, hi))","    c2Minv(hi, c2Maxv(a, lo))"),
("c2Sub-y","    a.y -= b.y;","    a.y -= b.x;"),
("c2Dot-sign","    a.x * b.x + a.y * b.y","    a.x * b.x - a.y * b.y"),
("c2RotIdentity","    r.c = 1.0f32;","    r.c = -1.0f32;"),
("c2xIdentity","    x.p = c2V(0.0, 0.0);","    x.p = c2V(1.0, 0.0);"),
("c2BBVerts-1","    *out.add(1) = c2V((*bb).max.x, (*bb).min.y);","    *out.add(1) = c2V((*bb).min.x, (*bb).max.y);"),
("c2BBVerts-3","    *out.add(3) = c2V((*bb).min.x, (*bb).max.y);","    *out.add(3) = c2V((*bb).max.x, (*bb).min.y);"),
("proxy-circle-count","            (*p).count = 1;","            (*p).count = 2;"),
("proxy-aabb-radius","            (*p).radius = 0.0;","            (*p).radius = 1.0;"),
("proxy-aabb-count","            (*p).count = 4;","            (*p).count = 3;"),
("proxy-capsule-count","            (*p).count = 2;","            (*p).count = 1;"),
("proxy-capsule-swap","            (*p).verts[0] = (*c).a;\n            (*p).verts[1] = (*c).b;","            (*p).verts[0] = (*c).b;\n            (*p).verts[1] = (*c).a;"),
("proxy-no-default","        // The C switch has no `default:` label -- an unknown type leaves the\n        // proxy completely untouched.\n        _ => {}","        _ => { (*p).radius = 0.0; (*p).count = 0; }"),
("c2Len-abs","    c2Dot(a, a).sqrt()","    c2Dot(a, a).abs().sqrt()"),
("c2Det2-swap","    a.x * b.y - a.y * b.x","    a.y * b.x - a.x * b.y"),
("metric-2","        2 => c2Len(c2Sub((*s).verts[1].p, (*s).verts[0].p)),","        2 => c2Len(c2Sub((*s).verts[0].p, (*s).verts[1].p)) + 0.0000001,"),
("metric-3-order","        3 => c2Det2(\n            c2Sub((*s).verts[1].p, (*s).verts[0].p),\n            c2Sub((*s).verts[2].p, (*s).verts[0].p),\n        ),","        3 => c2Det2(\n            c2Sub((*s).verts[2].p, (*s).verts[0].p),\n            c2Sub((*s).verts[1].p, (*s).verts[0].p),\n        ),"),
("metric-default-1","        // `default:` falls through into `case 1:` which returns 0.\n        _ => 0.0,","        _ => 1.0,"),
("c2Mulrv-sign","    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)","    c2V(a.c * b.x + a.s * b.y, a.s * b.x + a.c * b.y)"),
("c2Add-y","    a.y += b.y;","    a.y += b.x;"),
("c2Mulxv-order","    c2Add(c2Mulrv(a.r, b), a.p)","    c2Mulrv(a.r, c2Add(b, a.p))"),
("c22-v-strict","    if v <= 0.0 {","    if v < 0.0 {"),
("c22-u-strict","    } else if u <= 0.0 {","    } else if u < 0.0 {"),
("c22-uv-swap","        (*s).verts[0].u = u;\n        (*s).verts[1].u = v;","        (*s).verts[0].u = v;\n        (*s).verts[1].u = u;"),
("c22-div","        (*s).div = u + v;","        (*s).div = u - v;"),
("c22-count","        (*s).count = 2;\n    }\n}","        (*s).count = 3;\n    }\n}"),
("c23-b0","    if vAB <= 0.0 && uCA <= 0.0 {","    if vAB < 0.0 && uCA <= 0.0 {"),
("c23-b0b","    if vAB <= 0.0 && uCA <= 0.0 {","    if vAB <= 0.0 && uCA < 0.0 {"),
("c23-b1","    } else if uAB <= 0.0 && vBC <= 0.0 {","    } else if uAB < 0.0 && vBC <= 0.0 {"),
("c23-b1b","    } else if uAB <= 0.0 && vBC <= 0.0 {","    } else if uAB <= 0.0 && vBC < 0.0 {"),
("c23-b2","    } else if uBC <= 0.0 && vCA <= 0.0 {","    } else if uBC < 0.0 && vCA <= 0.0 {"),
("c23-b2b","    } else if uBC <= 0.0 && vCA <= 0.0 {","    } else if uBC <= 0.0 && vCA < 0.0 {"),
("c23-b3","    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {","    } else if uAB > 0.0 && vAB > 0.0 && wABC < 0.0 {"),
("c23-b3b","    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {","    } else if uAB >= 0.0 && vAB > 0.0 && wABC <= 0.0 {"),
("c23-b4","    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {","    } else if uBC > 0.0 && vBC > 0.0 && uABC < 0.0 {"),
("c23-b5","    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {","    } else if uCA > 0.0 && vCA > 0.0 && vABC < 0.0 {"),
("c23-area","    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));","    let area = c2Det2(c2Sub(c, a), c2Sub(b, a));"),
("c23-uABC","    let uABC = c2Det2(b, c) * area;","    let uABC = c2Det2(c, b) * area;"),
("c23-vABC","    let vABC = c2Det2(c, a) * area;","    let vABC = c2Det2(a, c) * area;"),
("c23-wABC","    let wABC = c2Det2(a, b) * area;","    let wABC = c2Det2(b, a) * area;"),
("c23-uAB","    let uAB = c2Dot(b, c2Sub(b, a));","    let uAB = c2Dot(a, c2Sub(b, a));"),
("c23-b5-assign","        (*s).verts[1] = (*s).verts[0];\n        (*s).verts[0] = (*s).verts[2];","        (*s).verts[0] = (*s).verts[2];\n        (*s).verts[1] = (*s).verts[0];"),
("c23-b4-assign","        (*s).verts[0] = (*s).verts[1];\n        (*s).verts[1] = (*s).verts[2];","        (*s).verts[1] = (*s).verts[2];\n        (*s).verts[0] = (*s).verts[1];"),
("c23-interior-div","        (*s).div = uABC + vABC + wABC;","        (*s).div = uABC + vABC - wABC;"),
("c2Neg","    c2V(-a.x, -a.y)","    c2V(-a.y, -a.x)"),
("c2Skew","    b.x = -a.y;\n    b.y = a.x;","    b.x = a.y;\n    b.y = -a.x;"),
("c2CCW90","    b.x = a.y;\n    b.y = -a.x;","    b.x = -a.y;\n    b.y = a.x;"),
("c2D-det","            if c2Det2(ab, c2Neg((*s).verts[0].p)) > 0.0 {","            if c2Det2(ab, c2Neg((*s).verts[0].p)) >= 0.0 {"),
("c2D-swap","                return c2Skew(ab);\n            }\n            c2CCW90(ab)","                return c2CCW90(ab);\n            }\n            c2Skew(ab)"),
("c2D-count1","        1 => c2Neg((*s).verts[0].p),","        1 => (*s).verts[0].p,"),
("c2Support-init","    let mut i: c_int = 1;","    let mut i: c_int = 0;"),
("c2Support-cmp","        if dot > dmax {","        if dot >= dmax {"),
("c2Witness-1","            *a = (*s).verts[0].sA;\n            *b = (*s).verts[0].sB;","            *a = (*s).verts[0].sB;\n            *b = (*s).verts[0].sA;"),
("c2Witness-den","    let den = 1.0f32 / (*s).div;","    let den = 1.0f32 / ((*s).div + 0.0);"),
("c2Witness-3-drop","                c2Mulvs((*s).verts[2].sA, den * (*s).verts[2].u),\n            );","                c2Mulvs((*s).verts[2].sA, den * (*s).verts[1].u),\n            );"),
("c2Div","    c2Mulvs(a, 1.0f32 / b)","    c2Mulvs(a, 1.0f32 / (b + 0.0))"),
("c2Norm","    c2Div(a, c2Len(a))","    c2Div(a, c2Len(a) + 0.0)"),
("c2L-2","        2 => c2Add(\n            c2Mulvs((*s).verts[0].p, den * (*s).verts[0].u),\n            c2Mulvs((*s).verts[1].p, den * (*s).verts[1].u),\n        ),","        2 => c2Add(\n            c2Mulvs((*s).verts[0].p, den * (*s).verts[1].u),\n            c2Mulvs((*s).verts[1].p, den * (*s).verts[0].u),\n        ),"),
("c2L-3","        _ => c2V(0.0, 0.0),\n    }\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2MulrvT","        _ => (*s).verts[2].p,\n    }\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2MulrvT"),
("c2MulrvT-sign","    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)","    c2V(a.c * b.x + a.s * b.y, a.s * b.x + a.c * b.y)"),
("gjk-ax-null","    if ax_ptr.is_null() {","    if !ax_ptr.is_null() {"),
("gjk-iter-cap","    while iter < 20 {","    while iter < 19 {"),
("gjk-iter-cap2","    while iter < 20 {","    while iter < 21 {"),
("gjk-d1d0","        if d1 > d0 {","        if d1 >= d0 {"),
("gjk-eps","        if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {","        if c2Dot(d, d) <= C2_FLT_EPSILON * C2_FLT_EPSILON {"),
("gjk-eps2","        if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {","        if c2Dot(d, d) < C2_FLT_EPSILON {"),
("gjk-count3","        if s.count == 3 {","        if s.count >= 3 {"),
("gjk-hit-ab","    if hit != 0 {\n        a = b;","    if hit != 0 {\n        b = a;"),
("gjk-radius-cond","    } else if use_radius != 0 {","    } else if use_radius == 1 {"),
("gjk-dist-gt","        if dist > rA + rB && dist > C2_FLT_EPSILON {","        if dist >= rA + rB && dist > C2_FLT_EPSILON {"),
("gjk-dist-eps","        if dist > rA + rB && dist > C2_FLT_EPSILON {","        if dist > rA + rB && dist >= C2_FLT_EPSILON {"),
("gjk-mid","            let p = c2Mulvs(c2Add(a, b), 0.5f32);","            let p = c2Mulvs(c2Add(a, b), 0.25f32);"),
("gjk-shrink-a","            a = c2Add(a, c2Mulvs(n, rA));","            a = c2Add(a, c2Mulvs(n, rB));"),
("gjk-shrink-b","            b = c2Sub(b, c2Mulvs(n, rB));","            b = c2Sub(b, c2Mulvs(n, rA));"),
("gjk-norm-dir","            let n = c2Norm(c2Sub(b, a));","            let n = c2Norm(c2Sub(a, b));"),
("gjk-ab-eq","            if a.x == b.x && a.y == b.y {","            if a.x == b.x || a.y == b.y {"),
("gjk-supportA","        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));","        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, d));"),
("gjk-supportB","        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));","        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, c2Neg(d)));"),
("gjk-p-sub","        (*v).p = c2Sub((*v).sB, (*v).sA);\n        let mut dup: c_int = 0;","        (*v).p = c2Sub((*v).sA, (*v).sB);\n        let mut dup: c_int = 0;"),
("gjk-cache-good","        let cache_was_good = (*cache).count != 0;","        let cache_was_good = (*cache).count > 0;"),
("gjk-cache-u","                (*v).u = 0.0;","                (*v).u = 1.0;"),
("gjk-cache-p","                (*v).p = c2Sub((*v).sB, (*v).sA);\n                (*v).u = 0.0;","                (*v).p = c2Sub((*v).sA, (*v).sB);\n                (*v).u = 0.0;"),
("gjk-cache-div","            s.div = (*cache).div;","            s.div = 1.0;"),
("gjk-metric-thresh","            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {","            if !(min_metric < max_metric * 2.0f32 && metric < 1.0e8f32) {"),
("gjk-metric-le","            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {","            if !(min_metric <= max_metric * 2.0f32 && metric < -1.0e8f32) {"),
("gjk-metric-2","            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {","            if !(min_metric < max_metric * 3.0f32 && metric < -1.0e8f32) {"),
("gjk-fresh-div","        s.div = 1.0f32;\n        s.count = 1;","        s.div = 2.0f32;\n        s.count = 1;"),
("gjk-fresh-u","        s.verts[0].u = 1.0f32;","        s.verts[0].u = 2.0f32;"),
("gjk-dup","            if iA == *saveA.as_ptr().offset(i as isize) && iB == *saveB.as_ptr().offset(i as isize) {","            if iA == *saveA.as_ptr().offset(i as isize) || iB == *saveB.as_ptr().offset(i as isize) {"),
("gjk-d0-assign","        d0 = d1;","        d0 = d1 * 0.5;"),
("gjk-writeback-count","        (*cache).count = s.count;","        (*cache).count = s.count.max(1);"),
("gjk-writeback-div","        (*cache).div = s.div;","        (*cache).div = 1.0;"),
("gjk-iter-out","        *iterations = iter;","        *iterations = iter + 1;"),
("aabb-d0","    let d0: c_int = (B.max.x < A.min.x) as c_int;","    let d0: c_int = (B.max.x <= A.min.x) as c_int;"),
("aabb-d1","    let d1: c_int = (A.max.x < B.min.x) as c_int;","    let d1: c_int = (A.max.x <= B.min.x) as c_int;"),
("aabb-d2","    let d2: c_int = (B.max.y < A.min.y) as c_int;","    let d2: c_int = (B.max.y < A.min.x) as c_int;"),
("aabb-or","    ((d0 | d1 | d2 | d3) == 0) as c_int","    ((d0 | d1 | d2) == 0) as c_int"),
("aabb-not","    ((d0 | d1 | d2 | d3) == 0) as c_int","    ((d0 | d1 | d2 | d3) != 0) as c_int"),
("cc-strict","    (d2 < r2) as c_int","    (d2 <= r2) as c_int"),
("cc-r2","    let mut r2 = A.r + B.r;","    let mut r2 = A.r - B.r;"),
("ca-r2","    let r2 = A.r * A.r;","    let r2 = A.r.abs();"),
("ca-strict","    (d2 < r2) as c_int\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2CircletoCapsule","    (d2 <= r2) as c_int\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2CircletoCapsule"),
("ca-clamp","    let L = c2Clampv(A.p, B.min, B.max);","    let L = c2Clampv(A.p, B.max, B.min);"),
("cap-da","    if da < 0.0 {","    if da <= 0.0 {"),
("cap-db","        if db < 0.0 {","        if db <= 0.0 {"),
("cap-r","    (d2 < r * r) as c_int","    (d2 <= r * r) as c_int"),
("cap-n","    let n = c2Sub(B.b, B.a);","    let n = c2Sub(B.a, B.b);"),
("cap-e","            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));","            let e = c2Sub(ap, c2Mulvs(n, da));"),
("collided-swap-ac","            C2_TYPE_CIRCLE => c2CircletoAABB(\n                (B as *const c2Circle).read_unaligned(),\n                (A as *const c2AABB).read_unaligned(),\n            ),","            C2_TYPE_CIRCLE => c2CircletoAABB(\n                (A as *const c2Circle).read_unaligned(),\n                (B as *const c2AABB).read_unaligned(),\n            ),"),
("collided-default","            _ => 0,\n        },\n        C2_TYPE_AABB => match typeB {","            _ => 1,\n        },\n        C2_TYPE_AABB => match typeB {"),
("collided-outer-default","        _ => 0,\n    }\n}","        _ => 1,\n    }\n}"),
("pred-ne0","        ) != 0.0\n        {\n            return 0;\n        }\n    }\n    1\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2CapsuletoCapsule","        ) > 0.0\n        {\n            return 0;\n        }\n    }\n    1\n}\n\n#[unsafe(no_mangle)]\npub extern \"C\" fn c2CapsuletoCapsule"),
("pred-useradius","            1,\n            core::ptr::null_mut(),\n            core::ptr::null_mut(),\n        ) != 0.0","            0,\n            core::ptr::null_mut(),\n            core::ptr::null_mut(),\n        ) != 0.0"),
("ptr-circle-r","            (*circle).r = c;","            (*circle).r = d;"),
("ptr-aabb-max","            (*aabb).max = c2V(c, d);","            (*aabb).max = c2V(d, c);"),
("ptr-capsule-r","            (*capsule).r = e;","            (*capsule).r = d;"),
("ptr-capsule-b","            (*capsule).b = c2V(c, d);","            (*capsule).b = c2V(d, c);"),
("omni-order","    c2Collided(A, type_a, B, type_b)","    c2Collided(B, type_b, A, type_a)"),
("omni-args","    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);","    let A = ptr_from_parts(type_a, a1, a2, a3, a5, a4);"),
]

def run(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True)

results=[]
for label, old, new in M:
    n = orig.count(old)
    if n == 0:
        results.append(("NOMATCH", label, "")); print("NOMATCH ", label, flush=True); continue
    mutated = orig.replace(old, new, 1)
    open(SRC,"w").write(mutated)
    b1=run("cargo build --offline -q"); b2=run("cargo build --release --offline -q")
    if b1.returncode or b2.returncode:
        results.append(("BUILDERR", label, "")); print("BUILDERR", label, flush=True)
        open(SRC,"w").write(orig); continue
    t=run("cargo test --offline -q")
    names=sorted(set(l.split(" ---")[0] for l in t.stdout.splitlines() if l.endswith("--- FAILED")))
    if not names:
        results.append(("SURVIVED", label, "")); print("SURVIVED", label, "(occurrences: %d)"%n, flush=True)
    else:
        results.append(("KILLED", label, names)); print("KILLED  ", label, "n=%d"%len(names), names[:4], flush=True)

open(SRC,"w").write(orig)
run("cargo build --offline -q"); run("cargo build --release --offline -q")
print("\n=== SUMMARY ===")
from collections import Counter
print(Counter(r[0] for r in results))
print("\nSURVIVED:", [r[1] for r in results if r[0]=="SURVIVED"])
print("\nNOMATCH:", [r[1] for r in results if r[0]=="NOMATCH"])
print("\nBUILDERR:", [r[1] for r in results if r[0]=="BUILDERR"])
