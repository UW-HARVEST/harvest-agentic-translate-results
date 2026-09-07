//! Phase D — high-volume randomized fuzz across the whole public surface, with
//! seeds and value distributions deliberately different from Phases B and C, to
//! catch divergences that the hand-enumerated `CONFIGS.md` / `ERRORS.md` rows
//! might have missed.

mod common;

use common::*;
use std::ffi::c_void;

#[allow(clippy::type_complexity)]
type GjkFn = unsafe extern "C" fn(
    *const c_void,
    i32,
    *const c2x,
    *const c_void,
    i32,
    *const c2x,
    *mut c2v,
    *mut c2v,
    i32,
    *mut i32,
    *mut c2GJKCache,
) -> f32;

/// Draw from a wide, deliberately nasty magnitude distribution.
fn nasty(rng: &mut Rng) -> f32 {
    match rng.below(14) {
        0 => 0.0,
        1 => -0.0,
        2 => f32::MIN_POSITIVE,
        3 => -f32::MIN_POSITIVE,
        4 => f32::from_bits(rng.next_u32() & 0x007f_ffff), // subnormal
        5 => rng.range(-1.0, 1.0) * 1e-20,
        6 => rng.range(-1.0, 1.0) * 1e20,
        7 => rng.range(-1.0, 1.0) * 3.0e38, // near FLT_MAX
        8 => FLT_EPSILON * rng.range(-4.0, 4.0),
        9 => rng.range(-1.0, 1.0),
        10 => rng.range(-1e3, 1e3),
        11 => (rng.next_u32() % 2001) as f32 - 1000.0, // exact integers
        12 => f32::from_bits(rng.next_u32()),          // any bit pattern incl. NaN
        _ => rng.range(-100.0, 100.0),
    }
}

fn nasty_v(rng: &mut Rng) -> c2v {
    c2v {
        x: nasty(rng),
        y: nasty(rng),
    }
}

#[test]
fn fuzz_gjk_full_option_cross_product() {
    let (c_f, r_f) = pair::<GjkFn>("c2GJK");
    let mut rng = Rng::new(0xD00D_F00D);

    // Observed maximum of `*iterations`. See the assertion at the end: this both
    // documents that the C's `while (iter < 20)` cap is unreachable and proves
    // that the one path which would leave `verts[count-1].u` uninitialised in the
    // C (exiting immediately after `++s.count`) cannot be taken.
    let mut max_iter = i32::MIN;
    let mut min_iter = i32::MAX;
    let mut nonzero_dists = 0usize;
    let mut zero_dists = 0usize;

    for i in 0..200_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];

        // Shapes built from the nasty distribution (may contain NaN/inf).
        let circ_a = c2Circle {
            p: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let aabb_a = c2AABB {
            min: nasty_v(&mut rng),
            max: nasty_v(&mut rng),
        };
        let cap_a = c2Capsule {
            a: nasty_v(&mut rng),
            b: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let circ_b = c2Circle {
            p: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let aabb_b = c2AABB {
            min: nasty_v(&mut rng),
            max: nasty_v(&mut rng),
        };
        let cap_b = c2Capsule {
            a: nasty_v(&mut rng),
            b: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let sel = |t: i32, c: &c2Circle, a: &c2AABB, p: &c2Capsule| -> *const c_void {
            match t {
                C2_TYPE_CIRCLE => c as *const c2Circle as *const c_void,
                C2_TYPE_AABB => a as *const c2AABB as *const c_void,
                _ => p as *const c2Capsule as *const c_void,
            }
        };
        let pa = sel(ta, &circ_a, &aabb_a, &cap_a);
        let pb = sel(tb, &circ_b, &aabb_b, &cap_b);

        // Randomize every option axis independently.
        let ax = c2x {
            p: nasty_v(&mut rng),
            r: c2r {
                c: nasty(&mut rng),
                s: nasty(&mut rng),
            },
        };
        let bx = c2x {
            p: nasty_v(&mut rng),
            r: c2r {
                c: nasty(&mut rng),
                s: nasty(&mut rng),
            },
        };
        let ax_ptr = if rng.below(3) == 0 {
            std::ptr::null()
        } else {
            &ax as *const c2x
        };
        let bx_ptr = if rng.below(3) == 0 {
            std::ptr::null()
        } else {
            &bx as *const c2x
        };
        let use_radius = (rng.below(2)) as i32;
        let want_a = rng.below(4) != 0;
        let want_b = rng.below(4) != 0;
        let want_it = rng.below(4) != 0;

        // Cache: NULL, cold, or warm with indices guaranteed inside the proxy's
        // valid vertex range for the chosen type (out-of-range indices are a
        // C-side OOB read, excluded in ERRORS.md).
        let max_a = match ta {
            C2_TYPE_CIRCLE => 0,
            C2_TYPE_AABB => 3,
            _ => 1,
        };
        let max_b = match tb {
            C2_TYPE_CIRCLE => 0,
            C2_TYPE_AABB => 3,
            _ => 1,
        };
        let cache_mode = rng.below(3);
        let seed_cache = match cache_mode {
            0 => None,
            1 => Some(c2GJKCache {
                metric: nasty(&mut rng),
                count: 0,
                iA: [0; 3],
                iB: [0; 3],
                div: nasty(&mut rng),
            }),
            _ => Some(c2GJKCache {
                metric: nasty(&mut rng),
                count: 1 + rng.below(3) as i32,
                iA: std::array::from_fn(|_| rng.below(max_a as u32 + 1) as i32),
                iB: std::array::from_fn(|_| rng.below(max_b as u32 + 1) as i32),
                div: nasty(&mut rng),
            }),
        };

        let sent_v = c2v {
            x: -55555.5,
            y: 44444.25,
        };
        let sent_i = -987654i32;

        unsafe {
            let run = |f: &GjkFn| {
                let mut oa = sent_v;
                let mut ob = sent_v;
                let mut it = sent_i;
                let mut cc = seed_cache;
                let d = f(
                    pa,
                    ta,
                    ax_ptr,
                    pb,
                    tb,
                    bx_ptr,
                    if want_a { &mut oa } else { std::ptr::null_mut() },
                    if want_b { &mut ob } else { std::ptr::null_mut() },
                    use_radius,
                    if want_it { &mut it } else { std::ptr::null_mut() },
                    cc.as_mut()
                        .map_or(std::ptr::null_mut(), |c| c as *mut c2GJKCache),
                );
                (d, oa, ob, it, cc)
            };
            let (cd, ca, cb, ci, cc) = run(&c_f);
            let (rd, ra, rb, ri, rc) = run(&r_f);

            let ctx = format!(
                "#{i} ta={ta} tb={tb} ur={use_radius} ax_null={} bx_null={} cache={cache_mode} \
                 A={:?}/{:?}/{:?} B={:?}/{:?}/{:?} ax={ax:?} bx={bx:?} seed={seed_cache:?}",
                ax_ptr.is_null(),
                bx_ptr.is_null(),
                circ_a,
                aabb_a,
                cap_a,
                circ_b,
                aabb_b,
                cap_b
            );
            assert_f32_bits!(cd, rd, "fuzz gjk dist :: {ctx}");
            assert_v_bits!(ca, ra, "fuzz gjk outA :: {ctx}");
            assert_v_bits!(cb, rb, "fuzz gjk outB :: {ctx}");
            assert_eq!(ci, ri, "fuzz gjk iterations :: {ctx}");
            match (cc, rc) {
                (Some(x), Some(y)) => assert!(
                    cache_bits_eq(&x, &y),
                    "fuzz gjk cache :: {ctx}\n  C={x:?}\n Rs={y:?}"
                ),
                (None, None) => {}
                _ => panic!("cache presence mismatch"),
            }

            if want_it {
                max_iter = max_iter.max(ci);
                min_iter = min_iter.min(ci);
            }
            if cd == 0.0 {
                zero_dists += 1;
            } else {
                nonzero_dists += 1;
            }
        }
    }

    assert!(
        zero_dists > 0 && nonzero_dists > 0,
        "fuzz produced only one distance class: {zero_dists}/{nonzero_dists}"
    );
    assert_eq!(min_iter, 0, "expected some immediate-exit calls");
    // `iter` is NOT bounded by 2: `c22`/`c23` can REDUCE `s.count` back to 1, so
    // the loop can append a vertex, collapse, and append again, incrementing
    // `iter` each time. It is bounded only by the C's own `while (iter < 20)`.
    //
    // This matters for one reason: exiting via that cap is the only way the C can
    // leave `verts[s.count-1].u` unwritten while `s.count >= 2` (every other exit
    // path runs `c22`/`c23` first, and the `dup` break exits WITHOUT the preceding
    // `++s.count`, so the half-written vertex is never counted). If the cap fired,
    // the C would read uninitialised stack there while the Rust reads a zeroed
    // field. `tests/hunt_cap.rs` searches 1.5M randomized calls -- including NaN,
    // infinite and subnormal geometry -- and tops out at iter == 6, with a sharply
    // decaying tail (38903 at 3, 1987 at 4, 129 at 5, 3 at 6). The cap is not
    // reachable in practice; it is not claimed to be provably unreachable.
    assert!(
        (0..=20).contains(&max_iter),
        "iterations reached {max_iter}, outside the C's own cap of 20"
    );
    println!("fuzz_gjk: iterations observed in [{min_iter}, {max_iter}] over 200k calls");
}

#[test]
fn fuzz_all_wrappers_and_entry_point() {
    type CIRCF = unsafe extern "C" fn(c2Circle, c2Circle) -> i32;
    type CABF = unsafe extern "C" fn(c2Circle, c2AABB) -> i32;
    type CCAP = unsafe extern "C" fn(c2Circle, c2Capsule) -> i32;
    type AAF = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
    type ACF = unsafe extern "C" fn(c2AABB, c2Capsule) -> i32;
    type CCF = unsafe extern "C" fn(c2Capsule, c2Capsule) -> i32;
    type COLL = unsafe extern "C" fn(*const c_void, i32, *const c_void, i32) -> i32;
    type RC = unsafe extern "C" fn(f32, f32, f32) -> i32;

    let (c_cc, r_cc) = pair::<CIRCF>("c2CircletoCircle");
    let (c_ca, r_ca) = pair::<CABF>("c2CircletoAABB");
    let (c_cp, r_cp) = pair::<CCAP>("c2CircletoCapsule");
    let (c_aa, r_aa) = pair::<AAF>("c2AABBtoAABB");
    let (c_ac, r_ac) = pair::<ACF>("c2AABBtoCapsule");
    let (c_pp, r_pp) = pair::<CCF>("c2CapsuletoCapsule");
    let (c_co, r_co) = pair::<COLL>("c2Collided");
    let (c_rc, r_rc) = pair::<RC>("reverse_collide");

    let mut rng = Rng::new(0xFEED_BEEF);
    for i in 0..300_000 {
        let circ = c2Circle {
            p: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let circ2 = c2Circle {
            p: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let aabb = c2AABB {
            min: nasty_v(&mut rng),
            max: nasty_v(&mut rng),
        };
        let aabb2 = c2AABB {
            min: nasty_v(&mut rng),
            max: nasty_v(&mut rng),
        };
        let cap = c2Capsule {
            a: nasty_v(&mut rng),
            b: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        let cap2 = c2Capsule {
            a: nasty_v(&mut rng),
            b: nasty_v(&mut rng),
            r: nasty(&mut rng),
        };
        unsafe {
            assert_eq!(c_cc(circ, circ2), r_cc(circ, circ2), "fuzz cc #{i}");
            assert_eq!(c_ca(circ, aabb), r_ca(circ, aabb), "fuzz ca #{i}");
            assert_eq!(c_cp(circ, cap), r_cp(circ, cap), "fuzz cp #{i}");
            assert_eq!(c_aa(aabb, aabb2), r_aa(aabb, aabb2), "fuzz aa #{i}");
            assert_eq!(c_ac(aabb, cap), r_ac(aabb, cap), "fuzz ac #{i}");
            assert_eq!(c_pp(cap, cap2), r_pp(cap, cap2), "fuzz pp #{i}");

            // c2Collided over every ordered type pair, including invalid tags.
            let all: Vec<i32> = ALL_TYPES
                .iter()
                .copied()
                .chain([-1, 3, i32::MAX, i32::MIN])
                .collect();
            let ta = all[rng.below(all.len() as u32) as usize];
            let tb = all[rng.below(all.len() as u32) as usize];
            let sel = |t: i32| -> *const c_void {
                match t {
                    C2_TYPE_AABB => &aabb as *const c2AABB as *const c_void,
                    C2_TYPE_CAPSULE => &cap as *const c2Capsule as *const c_void,
                    // any invalid tag never dereferences the pointer
                    _ => &circ as *const c2Circle as *const c_void,
                }
            };
            assert_eq!(
                c_co(sel(ta), ta, sel(tb), tb),
                r_co(sel(ta), ta, sel(tb), tb),
                "fuzz collided ({ta},{tb}) #{i} circ={circ:?} aabb={aabb:?} cap={cap:?}"
            );

            let (x, y, r) = (nasty(&mut rng), nasty(&mut rng), nasty(&mut rng));
            assert_eq!(
                c_rc(x, y, r),
                r_rc(x, y, r),
                "fuzz reverse_collide({x}, {y}, {r}) #{i}"
            );
        }
    }
}

#[test]
fn fuzz_simplex_helpers_with_arbitrary_bit_patterns() {
    type S1 = unsafe extern "C" fn(*mut c2Simplex) -> ();
    type SV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    type SF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
    type WF = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v) -> ();

    let (c_22, r_22) = pair::<S1>("c22");
    let (c_23, r_23) = pair::<S1>("c23");
    let (c_d, r_d) = pair::<SV>("c2D");
    let (c_l, r_l) = pair::<SV>("c2L");
    let (c_m, r_m) = pair::<SF>("c2GJKSimplexMetric");
    let (c_w, r_w) = pair::<WF>("c2Witness");

    let mut rng = Rng::new(0xBADD_CAFE);
    for i in 0..200_000 {
        // Fully arbitrary simplex contents, including NaN/inf/subnormal floats
        // and out-of-range vertex indices, with `count` drawn from a set that
        // straddles every `switch` boundary.
        let mut s = c2Simplex {
            verts: std::array::from_fn(|_| c2sv {
                sA: nasty_v(&mut rng),
                sB: nasty_v(&mut rng),
                p: nasty_v(&mut rng),
                u: nasty(&mut rng),
                iA: rng.next_u32() as i32,
                iB: rng.next_u32() as i32,
            }),
            div: nasty(&mut rng),
            count: match rng.below(8) {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 3,
                4 => 4,
                5 => -1,
                6 => i32::MAX,
                _ => i32::MIN,
            },
        };
        // Occasionally force coincident / collinear points.
        match rng.below(8) {
            0 => s.verts[1].p = s.verts[0].p,
            1 => s.verts[2].p = s.verts[0].p,
            2 => {
                s.verts[1].p = s.verts[0].p;
                s.verts[2].p = s.verts[0].p;
            }
            _ => {}
        }

        unsafe {
            let ctx = format!("#{i} count={} div={}", s.count, s.div);

            // c22 and c23 are called by c2GJK only for count 2 / 3, but they are
            // exported symbols and an external caller may invoke them with any
            // simplex, so fuzz them unconditionally.
            let (mut cs, mut rs) = (s, s);
            c_22(&mut cs);
            r_22(&mut rs);
            assert!(simplex_bits_eq(&cs, &rs), "fuzz c22 :: {ctx}");

            let (mut cs, mut rs) = (s, s);
            c_23(&mut cs);
            r_23(&mut rs);
            assert!(simplex_bits_eq(&cs, &rs), "fuzz c23 :: {ctx}");

            let (mut cs, mut rs) = (s, s);
            assert_v_bits!(c_d(&mut cs), r_d(&mut rs), "fuzz c2D :: {ctx}");
            assert!(simplex_bits_eq(&cs, &rs), "fuzz c2D side effects :: {ctx}");

            let (mut cs, mut rs) = (s, s);
            assert_v_bits!(c_l(&mut cs), r_l(&mut rs), "fuzz c2L :: {ctx}");

            let (mut cs, mut rs) = (s, s);
            assert_f32_bits!(c_m(&mut cs), r_m(&mut rs), "fuzz metric :: {ctx}");

            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v::default(), c2v::default());
            let (mut ra, mut rb) = (c2v::default(), c2v::default());
            c_w(&mut cs, &mut ca, &mut cb);
            r_w(&mut rs, &mut ra, &mut rb);
            assert_v_bits!(ca, ra, "fuzz witness A :: {ctx}");
            assert_v_bits!(cb, rb, "fuzz witness B :: {ctx}");
        }
    }
}

#[test]
fn fuzz_leaf_math_with_arbitrary_bit_patterns() {
    type V1 = unsafe extern "C" fn(c2v) -> c2v;
    type V2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type V3 = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
    type VS = unsafe extern "C" fn(c2v, f32) -> c2v;
    type F1 = unsafe extern "C" fn(c2v) -> f32;
    type F2 = unsafe extern "C" fn(c2v, c2v) -> f32;
    type RV = unsafe extern "C" fn(c2r, c2v) -> c2v;
    type XV = unsafe extern "C" fn(c2x, c2v) -> c2v;
    type BB = unsafe extern "C" fn(*mut c2v, *mut c2AABB) -> ();
    type SUP = unsafe extern "C" fn(*const c2v, i32, c2v) -> i32;

    let v1s: [(&str, (_, _)); 4] = [
        ("c2Neg", { let (a, b) = pair::<V1>("c2Neg"); (*a, *b) }),
        ("c2Skew", { let (a, b) = pair::<V1>("c2Skew"); (*a, *b) }),
        ("c2CCW90", { let (a, b) = pair::<V1>("c2CCW90"); (*a, *b) }),
        ("c2Norm", { let (a, b) = pair::<V1>("c2Norm"); (*a, *b) }),
    ];
    let v2s: [(&str, (_, _)); 4] = [
        ("c2Sub", { let (a, b) = pair::<V2>("c2Sub"); (*a, *b) }),
        ("c2Add", { let (a, b) = pair::<V2>("c2Add"); (*a, *b) }),
        ("c2Maxv", { let (a, b) = pair::<V2>("c2Maxv"); (*a, *b) }),
        ("c2Minv", { let (a, b) = pair::<V2>("c2Minv"); (*a, *b) }),
    ];
    let f2s: [(&str, (_, _)); 2] = [
        ("c2Dot", { let (a, b) = pair::<F2>("c2Dot"); (*a, *b) }),
        ("c2Det2", { let (a, b) = pair::<F2>("c2Det2"); (*a, *b) }),
    ];
    let (c_len, r_len) = pair::<F1>("c2Len");
    let (c_clamp, r_clamp) = pair::<V3>("c2Clampv");
    let (c_muls, r_muls) = pair::<VS>("c2Mulvs");
    let (c_div, r_div) = pair::<VS>("c2Div");
    let (c_mrv, r_mrv) = pair::<RV>("c2Mulrv");
    let (c_mrvt, r_mrvt) = pair::<RV>("c2MulrvT");
    let (c_mxv, r_mxv) = pair::<XV>("c2Mulxv");
    let (c_bb, r_bb) = pair::<BB>("c2BBVerts");
    let (c_sup, r_sup) = pair::<SUP>("c2Support");

    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for i in 0..400_000 {
        let a = nasty_v(&mut rng);
        let b = nasty_v(&mut rng);
        let c = nasty_v(&mut rng);
        let s = nasty(&mut rng);
        let rot = c2r {
            c: nasty(&mut rng),
            s: nasty(&mut rng),
        };
        let xf = c2x { p: c, r: rot };
        unsafe {
            for (name, (cf, rf)) in &v1s {
                assert_v_bits!(cf(a), rf(a), "fuzz {name} #{i} {a:?}");
            }
            for (name, (cf, rf)) in &v2s {
                assert_v_bits!(cf(a, b), rf(a, b), "fuzz {name} #{i} {a:?} {b:?}");
            }
            for (name, (cf, rf)) in &f2s {
                assert_f32_bits!(cf(a, b), rf(a, b), "fuzz {name} #{i} {a:?} {b:?}");
            }
            assert_f32_bits!(c_len(a), r_len(a), "fuzz c2Len #{i} {a:?}");
            assert_v_bits!(
                c_clamp(a, b, c),
                r_clamp(a, b, c),
                "fuzz c2Clampv #{i} {a:?} {b:?} {c:?}"
            );
            assert_v_bits!(c_muls(a, s), r_muls(a, s), "fuzz c2Mulvs #{i} {a:?} {s}");
            assert_v_bits!(c_div(a, s), r_div(a, s), "fuzz c2Div #{i} {a:?} {s}");
            assert_v_bits!(c_mrv(rot, a), r_mrv(rot, a), "fuzz c2Mulrv #{i}");
            assert_v_bits!(c_mrvt(rot, a), r_mrvt(rot, a), "fuzz c2MulrvT #{i}");
            assert_v_bits!(c_mxv(xf, a), r_mxv(xf, a), "fuzz c2Mulxv #{i}");

            // c2BBVerts into an 8-slot buffer, sentinel-checked.
            let sent = c2v { x: 3.5, y: -3.5 };
            let mut co = [sent; 8];
            let mut ro = [sent; 8];
            let mut cbb = c2AABB { min: a, max: b };
            let mut rbb = cbb;
            c_bb(co.as_mut_ptr(), &mut cbb);
            r_bb(ro.as_mut_ptr(), &mut rbb);
            for k in 0..8 {
                assert_v_bits!(co[k], ro[k], "fuzz c2BBVerts #{i} out[{k}] bb={cbb:?}");
            }
            assert!(aabb_bits_eq(cbb, rbb), "fuzz c2BBVerts #{i} input mutated");

            // c2Support over a fully random 8-vertex buffer, valid counts only.
            let verts: [c2v; 8] = std::array::from_fn(|_| nasty_v(&mut rng));
            let n = (rng.below(9)) as i32; // 0..=8, all within the buffer
            assert_eq!(
                c_sup(verts.as_ptr(), n, a),
                r_sup(verts.as_ptr(), n, a),
                "fuzz c2Support #{i} n={n} d={a:?} verts={verts:?}"
            );
        }
    }
}
