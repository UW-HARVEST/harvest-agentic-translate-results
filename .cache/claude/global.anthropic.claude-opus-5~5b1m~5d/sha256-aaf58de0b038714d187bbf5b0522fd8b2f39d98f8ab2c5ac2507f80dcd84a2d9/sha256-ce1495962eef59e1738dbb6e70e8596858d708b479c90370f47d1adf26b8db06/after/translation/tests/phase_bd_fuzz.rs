//! Phase B/D — high-volume randomized differential fuzz over **all 38 exported
//! symbols**, driven through both `.so`s.
//!
//! The per-row tests in `phase_b_group*.rs` pin down specific configurations;
//! this file adds volume and, crucially, *cross-symbol* sequences (a warm-start
//! `c2GJK` chain whose cache is then fed back, simplex structs mutated by `c22`
//! and then handed to `c2D`/`c2L`/`c2Witness`, …) so state carried between calls
//! is compared too.
//!
//! Seeded and deterministic; bounded by a wall-clock budget so it can never
//! exceed the harness timeout.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(25);

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct Buf {
    bytes: [u8; 32],
}

impl Buf {
    fn of<T: Copy>(v: &T) -> Buf {
        let mut b = Buf { bytes: [0; 32] };
        let s = raw(v);
        b.bytes[..s.len()].copy_from_slice(&s);
        b
    }
    fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr() as *const c_void
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

fn nverts(t: c_int) -> u32 {
    match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    }
}

/// Every leaf scalar/vector function, hammered together so their interaction
/// through shared operands is covered as well.
#[test]
fn fuzz_all_leaf_functions() {
    let p = load();
    let f_V: FnV = p.c.sym("c2V");
    let r_V: FnV = p.rs.sym("c2V");
    let f_Mulvs: FnMulvs = p.c.sym("c2Mulvs");
    let r_Mulvs: FnMulvs = p.rs.sym("c2Mulvs");
    let f_Maxv: FnVV = p.c.sym("c2Maxv");
    let r_Maxv: FnVV = p.rs.sym("c2Maxv");
    let f_Minv: FnVV = p.c.sym("c2Minv");
    let r_Minv: FnVV = p.rs.sym("c2Minv");
    let f_Clampv: FnVVV = p.c.sym("c2Clampv");
    let r_Clampv: FnVVV = p.rs.sym("c2Clampv");
    let f_Sub: FnVV = p.c.sym("c2Sub");
    let r_Sub: FnVV = p.rs.sym("c2Sub");
    let f_Add: FnVV = p.c.sym("c2Add");
    let r_Add: FnVV = p.rs.sym("c2Add");
    let f_Dot: FnVVf = p.c.sym("c2Dot");
    let r_Dot: FnVVf = p.rs.sym("c2Dot");
    let f_Det2: FnVVf = p.c.sym("c2Det2");
    let r_Det2: FnVVf = p.rs.sym("c2Det2");
    let f_Len: FnVf = p.c.sym("c2Len");
    let r_Len: FnVf = p.rs.sym("c2Len");
    let f_Div: FnDiv = p.c.sym("c2Div");
    let r_Div: FnDiv = p.rs.sym("c2Div");
    let f_Norm: FnV1 = p.c.sym("c2Norm");
    let r_Norm: FnV1 = p.rs.sym("c2Norm");
    let f_Neg: FnV1 = p.c.sym("c2Neg");
    let r_Neg: FnV1 = p.rs.sym("c2Neg");
    let f_Skew: FnV1 = p.c.sym("c2Skew");
    let r_Skew: FnV1 = p.rs.sym("c2Skew");
    let f_CCW: FnV1 = p.c.sym("c2CCW90");
    let r_CCW: FnV1 = p.rs.sym("c2CCW90");
    let f_Mulrv: FnMulrv = p.c.sym("c2Mulrv");
    let r_Mulrv: FnMulrv = p.rs.sym("c2Mulrv");
    let f_MulrvT: FnMulrv = p.c.sym("c2MulrvT");
    let r_MulrvT: FnMulrv = p.rs.sym("c2MulrvT");
    let f_Mulxv: FnMulxv = p.c.sym("c2Mulxv");
    let r_Mulxv: FnMulxv = p.rs.sym("c2Mulxv");
    let f_RotId: FnRotIdentity = p.c.sym("c2RotIdentity");
    let r_RotId: FnRotIdentity = p.rs.sym("c2RotIdentity");
    let f_xId: FnxIdentity = p.c.sym("c2xIdentity");
    let r_xId: FnxIdentity = p.rs.sym("c2xIdentity");

    let mut rng = Rng::new(0xF00D);
    let t0 = Instant::now();
    let mut n = 0u64;
    unsafe {
        assert_bits_eq!(f_RotId(), r_RotId(), "c2RotIdentity");
        assert_bits_eq!(f_xId(), r_xId(), "c2xIdentity");
        while t0.elapsed() < BUDGET / 4 {
            for _ in 0..2000 {
                let a = if rng.below(3) == 0 { rng.v_wild() } else { rng.v() };
                let b = if rng.below(3) == 0 { rng.v_wild() } else { rng.v() };
                let c = if rng.below(3) == 0 { rng.v_wild() } else { rng.v() };
                let s = if rng.below(3) == 0 { rng.wild() } else { rng.coord() };
                let rot = if rng.below(3) == 0 {
                    c2r { c: rng.wild(), s: rng.wild() }
                } else {
                    rng.rot()
                };
                let xf = c2x { p: a, r: rot };

                assert_bits_eq!(f_V(a.x, b.y), r_V(a.x, b.y), "c2V");
                assert_bits_eq!(f_Mulvs(a, s), r_Mulvs(a, s), "c2Mulvs");
                assert_bits_eq!(f_Maxv(a, b), r_Maxv(a, b), "c2Maxv");
                assert_bits_eq!(f_Minv(a, b), r_Minv(a, b), "c2Minv");
                assert_bits_eq!(f_Clampv(a, b, c), r_Clampv(a, b, c), "c2Clampv");
                assert_bits_eq!(f_Sub(a, b), r_Sub(a, b), "c2Sub");
                assert_bits_eq!(f_Add(a, b), r_Add(a, b), "c2Add");
                assert_f32_bits_eq!(f_Dot(a, b), r_Dot(a, b), "c2Dot");
                assert_f32_bits_eq!(f_Det2(a, b), r_Det2(a, b), "c2Det2");
                assert_f32_bits_eq!(f_Len(a), r_Len(a), "c2Len");
                assert_bits_eq!(f_Div(a, s), r_Div(a, s), "c2Div");
                assert_bits_eq!(f_Norm(a), r_Norm(a), "c2Norm");
                assert_bits_eq!(f_Neg(a), r_Neg(a), "c2Neg");
                assert_bits_eq!(f_Skew(a), r_Skew(a), "c2Skew");
                assert_bits_eq!(f_CCW(a), r_CCW(a), "c2CCW90");
                assert_bits_eq!(f_Mulrv(rot, b), r_Mulrv(rot, b), "c2Mulrv");
                assert_bits_eq!(f_MulrvT(rot, b), r_MulrvT(rot, b), "c2MulrvT");
                assert_bits_eq!(f_Mulxv(xf, b), r_Mulxv(xf, b), "c2Mulxv");

                // Chained: feed each result back in, so a divergence in one
                // function is amplified rather than cancelled.
                let chained_c = f_Norm(f_Sub(f_Mulrv(rot, a), f_Neg(b)));
                let chained_r = r_Norm(r_Sub(r_Mulrv(rot, a), r_Neg(b)));
                assert_bits_eq!(chained_c, chained_r, "chained leaf pipeline");
                n += 1;
            }
        }
    }
    eprintln!("fuzz_all_leaf_functions: {n} rounds x 19 symbols");
    assert!(n > 1000, "fuzz budget produced too few rounds: {n}");
}

/// Simplex-mutating functions, run in *sequence* on the same struct so the
/// state each leaves behind is exercised by the next call.
#[test]
fn fuzz_simplex_pipeline() {
    let p = load();
    let c_c22: FnSimplexVoid = p.c.sym("c22");
    let r_c22: FnSimplexVoid = p.rs.sym("c22");
    let c_c23: FnSimplexVoid = p.c.sym("c23");
    let r_c23: FnSimplexVoid = p.rs.sym("c23");
    let c_D: FnSimplexv = p.c.sym("c2D");
    let r_D: FnSimplexv = p.rs.sym("c2D");
    let c_L: FnSimplexv = p.c.sym("c2L");
    let r_L: FnSimplexv = p.rs.sym("c2L");
    let c_M: FnSimplexf = p.c.sym("c2GJKSimplexMetric");
    let r_M: FnSimplexf = p.rs.sym("c2GJKSimplexMetric");
    let c_W: FnWitness = p.c.sym("c2Witness");
    let r_W: FnWitness = p.rs.sym("c2Witness");
    let c_S: FnSupport = p.c.sym("c2Support");
    let r_S: FnSupport = p.rs.sym("c2Support");

    let mut rng = Rng::new(0xBEEF);
    let t0 = Instant::now();
    let mut n = 0u64;
    unsafe {
        while t0.elapsed() < BUDGET / 4 {
            for _ in 0..500 {
                let count = 1 + (rng.below(3) as c_int);
                let mut cs = rng.simplex(count);
                if rng.below(4) == 0 {
                    for i in 0..4 {
                        cs.verts[i].p = rng.v_wild();
                        cs.verts[i].sA = rng.v_wild();
                        cs.verts[i].sB = rng.v_wild();
                        cs.verts[i].u = rng.wild();
                    }
                    cs.div = rng.wild();
                }
                let mut rs_ = cs;

                // 1. reduce
                match count {
                    2 => {
                        c_c22(&mut cs);
                        r_c22(&mut rs_);
                    }
                    3 => {
                        c_c23(&mut cs);
                        r_c23(&mut rs_);
                    }
                    _ => {}
                }
                assert!(
                    raw(&cs) == raw(&rs_),
                    "simplex diverged after reduction\n C   : {}\n Rust: {}",
                    simplex_hex(&cs),
                    simplex_hex(&rs_)
                );

                // 2. metric, direction, closest point on the REDUCED simplex
                assert_f32_bits_eq!(c_M(&mut cs), r_M(&mut rs_), "metric after reduction");
                let dc = c_D(&mut cs);
                let dr = r_D(&mut rs_);
                assert_bits_eq!(dc, dr, "c2D after reduction");
                let lc = c_L(&mut cs);
                let lr = r_L(&mut rs_);
                assert_bits_eq!(lc, lr, "c2L after reduction");

                // 3. support along that direction over a random vertex set
                let mut verts = [c2v::default(); 8];
                for i in 0..8 {
                    verts[i] = if rng.below(5) == 0 { rng.v_wild() } else { rng.v() };
                }
                for vc in [1i32, 2, 4, 8] {
                    assert_eq!(
                        c_S(verts.as_ptr(), vc, dc),
                        r_S(verts.as_ptr(), vc, dr),
                        "c2Support after c2D"
                    );
                }

                // 4. witness of the reduced simplex
                let (mut ca, mut cb) = (c2v { x: 7.0, y: 8.0 }, c2v { x: 9.0, y: 10.0 });
                let (mut ra, mut rb) = (ca, cb);
                c_W(&mut cs, &mut ca, &mut cb);
                r_W(&mut rs_, &mut ra, &mut rb);
                assert!(
                    raw(&ca) == raw(&ra) && raw(&cb) == raw(&rb) && raw(&cs) == raw(&rs_),
                    "c2Witness diverged\n C   a={} b={}\n Rust a={} b={}",
                    v_hex(&ca),
                    v_hex(&cb),
                    v_hex(&ra),
                    v_hex(&rb)
                );

                // 5. feed the reduced simplex back through the other reducer
                let mut cs2 = cs;
                let mut rs2 = rs_;
                cs2.count = 1 + (rng.below(3) as c_int);
                rs2.count = cs2.count;
                if cs2.count == 2 {
                    c_c22(&mut cs2);
                    r_c22(&mut rs2);
                } else if cs2.count == 3 {
                    c_c23(&mut cs2);
                    r_c23(&mut rs2);
                }
                assert!(raw(&cs2) == raw(&rs2), "second-pass reduction diverged");
                n += 1;
            }
        }
    }
    eprintln!("fuzz_simplex_pipeline: {n} rounds");
    assert!(n > 500, "fuzz budget produced too few rounds: {n}");
}

/// `c2GJK` over the full option cross-product, including multi-call warm-start
/// chains where the cache written by call *k* is the input of call *k+1*.
#[test]
fn fuzz_gjk_full_option_space() {
    let p = load();
    let cf: FnGJK = p.c.sym("c2GJK");
    let rf: FnGJK = p.rs.sym("c2GJK");
    let mut rng = Rng::new(0x1234_5678);
    let t0 = Instant::now();
    let mut n = 0u64;
    let mut hist = [0usize; 22];

    let call = |f: FnGJK,
                    a: &Buf,
                    ta: c_int,
                    b: &Buf,
                    tb: c_int,
                    ax: Option<&c2x>,
                    bx: Option<&c2x>,
                    ur: c_int,
                    mask: u32,
                    cache: Option<c2GJKCache>|
     -> (f32, c2v, c2v, c_int, c2GJKCache) {
        let mut oa = c2v { x: -1.5, y: 2.5 };
        let mut ob = c2v { x: 3.5, y: -4.5 };
        let mut it: c_int = -13;
        let mut cc = cache.unwrap_or_default();
        let rc = unsafe {
            f(
                a.ptr(),
                ta,
                ax.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
                b.ptr(),
                tb,
                bx.map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
                if mask & 1 != 0 { &mut oa } else { std::ptr::null_mut() },
                if mask & 2 != 0 { &mut ob } else { std::ptr::null_mut() },
                ur,
                if mask & 4 != 0 { &mut it } else { std::ptr::null_mut() },
                if cache.is_some() { &mut cc } else { std::ptr::null_mut() },
            )
        };
        (rc, oa, ob, it, cc)
    };

    while t0.elapsed() < BUDGET / 2 {
        for _ in 0..500 {
            let ta = TYPES[rng.below(3) as usize];
            let tb = TYPES[rng.below(3) as usize];
            let wild = rng.below(4) == 0;
            let mk = |rng: &mut Rng, t: c_int| -> Buf {
                match (t, wild) {
                    (C2_TYPE_CIRCLE, false) => Buf::of(&rng.circle()),
                    (C2_TYPE_AABB, false) => Buf::of(&rng.aabb()),
                    (C2_TYPE_CIRCLE, true) => Buf::of(&c2Circle { p: rng.v_wild(), r: rng.wild() }),
                    (C2_TYPE_AABB, true) => Buf::of(&c2AABB { min: rng.v_wild(), max: rng.v_wild() }),
                    (_, true) => Buf::of(&c2Capsule { a: rng.v_wild(), b: rng.v_wild(), r: rng.wild() }),
                    _ => Buf::of(&rng.capsule()),
                }
            };
            let axv = if rng.below(4) == 0 {
                c2x { p: rng.v_wild(), r: c2r { c: rng.wild(), s: rng.wild() } }
            } else {
                rng.x()
            };
            let bxv = if rng.below(4) == 0 {
                c2x { p: rng.v_wild(), r: c2r { c: rng.wild(), s: rng.wild() } }
            } else {
                rng.x()
            };
            let use_ax = rng.below(2) == 0;
            let use_bx = rng.below(2) == 0;
            let ur = rng.below(2) as c_int;
            let mask = rng.below(8);
            let want_cache = rng.below(2) == 0;

            let mut ccache = if want_cache {
                let mut cc = c2GJKCache {
                    metric: match rng.below(5) {
                        0 => rng.wild(),
                        1 => -1.0e9,
                        _ => rng.uniform(-2.0e8, 2.0e8),
                    },
                    count: rng.below(4) as c_int,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: if rng.below(6) == 0 { rng.wild() } else { rng.uniform(0.01, 8.0) },
                };
                for i in 0..3 {
                    cc.iA[i] = rng.below(nverts(ta)) as c_int;
                    cc.iB[i] = rng.below(nverts(tb)) as c_int;
                }
                Some(cc)
            } else {
                None
            };
            let mut rcache = ccache;

            // A 4-call warm-start chain with the shapes changing each step.
            for step in 0..4 {
                let a = mk(&mut rng, ta);
                let b = mk(&mut rng, tb);
                let (crc, ca, cb, cit, cc) = call(
                    cf, &a, ta, &b, tb,
                    if use_ax { Some(&axv) } else { None },
                    if use_bx { Some(&bxv) } else { None },
                    ur, mask, ccache,
                );
                let (rrc, ra, rb, rit, rc_) = call(
                    rf, &a, ta, &b, tb,
                    if use_ax { Some(&axv) } else { None },
                    if use_bx { Some(&bxv) } else { None },
                    ur, mask, rcache,
                );
                assert!(
                    crc.to_bits() == rrc.to_bits()
                        && raw(&ca) == raw(&ra)
                        && raw(&cb) == raw(&rb)
                        && cit == rit
                        && raw(&cc) == raw(&rc_),
                    "DIVERGENCE c2GJK fuzz step={step} ta={ta} tb={tb} ur={ur} mask={mask:03b} ax={use_ax} bx={use_bx}\n  A={:02x?}\n  B={:02x?}\n  in-cache={}\n  C   : rc={} a={} b={} it={} cache=[{}]\n  Rust: rc={} a={} b={} it={} cache=[{}]",
                    &a.bytes[..20],
                    &b.bytes[..20],
                    ccache.map(|c| cache_hex(&c)).unwrap_or("NULL".into()),
                    f32_hex(crc), v_hex(&ca), v_hex(&cb), cit, cache_hex(&cc),
                    f32_hex(rrc), v_hex(&ra), v_hex(&rb), rit, cache_hex(&rc_),
                );
                if want_cache {
                    ccache = Some(cc);
                    rcache = Some(rc_);
                }
                if mask & 4 != 0 {
                    hist[cit.clamp(0, 21) as usize] += 1;
                }
                n += 1;
            }
        }
    }
    eprintln!("fuzz_gjk_full_option_space: {n} c2GJK calls; iters histogram {hist:?}");
    assert!(n > 10_000, "fuzz budget produced too few calls: {n}");
}

/// The 8 predicate/entry-point symbols, at volume.
#[test]
fn fuzz_predicates_and_entry_point() {
    let p = load();
    let c_aa: FnAABBtoAABB = p.c.sym("c2AABBtoAABB");
    let r_aa: FnAABBtoAABB = p.rs.sym("c2AABBtoAABB");
    let c_ac: FnAABBtoCapsule = p.c.sym("c2AABBtoCapsule");
    let r_ac: FnAABBtoCapsule = p.rs.sym("c2AABBtoCapsule");
    let c_cc: FnCapsuletoCapsule = p.c.sym("c2CapsuletoCapsule");
    let r_cc: FnCapsuletoCapsule = p.rs.sym("c2CapsuletoCapsule");
    let c_kk: FnCircletoCircle = p.c.sym("c2CircletoCircle");
    let r_kk: FnCircletoCircle = p.rs.sym("c2CircletoCircle");
    let c_ka: FnCircletoAABB = p.c.sym("c2CircletoAABB");
    let r_ka: FnCircletoAABB = p.rs.sym("c2CircletoAABB");
    let c_kc: FnCircletoCapsule = p.c.sym("c2CircletoCapsule");
    let r_kc: FnCircletoCapsule = p.rs.sym("c2CircletoCapsule");
    let c_co: FnCollided = p.c.sym("c2Collided");
    let r_co: FnCollided = p.rs.sym("c2Collided");
    let c_cap: FnCapsule = p.c.sym("capsule");
    let r_cap: FnCapsule = p.rs.sym("capsule");
    let c_mp: FnMakeProxy = p.c.sym("c2MakeProxy");
    let r_mp: FnMakeProxy = p.rs.sym("c2MakeProxy");
    let c_bv: FnBBVerts = p.c.sym("c2BBVerts");
    let r_bv: FnBBVerts = p.rs.sym("c2BBVerts");

    let mut rng = Rng::new(0xCAFE);
    let t0 = Instant::now();
    let mut n = 0u64;
    unsafe {
        while t0.elapsed() < BUDGET / 4 {
            for _ in 0..1000 {
                let wild = rng.below(5) == 0;
                let ci = if wild {
                    c2Circle { p: rng.v_wild(), r: rng.wild() }
                } else {
                    rng.circle()
                };
                let ci2 = if wild {
                    c2Circle { p: rng.v_wild(), r: rng.wild() }
                } else {
                    rng.circle()
                };
                let bb = if wild {
                    c2AABB { min: rng.v_wild(), max: rng.v_wild() }
                } else {
                    rng.aabb()
                };
                let bb2 = if wild {
                    c2AABB { min: rng.v_wild(), max: rng.v_wild() }
                } else {
                    rng.aabb()
                };
                let ca = if wild {
                    c2Capsule { a: rng.v_wild(), b: rng.v_wild(), r: rng.wild() }
                } else {
                    rng.capsule()
                };
                let ca2 = if wild {
                    c2Capsule { a: rng.v_wild(), b: rng.v_wild(), r: rng.wild() }
                } else {
                    rng.capsule()
                };

                assert_eq!(c_aa(bb, bb2), r_aa(bb, bb2), "c2AABBtoAABB");
                assert_eq!(c_ac(bb, ca), r_ac(bb, ca), "c2AABBtoCapsule");
                assert_eq!(c_cc(ca, ca2), r_cc(ca, ca2), "c2CapsuletoCapsule");
                assert_eq!(c_kk(ci, ci2), r_kk(ci, ci2), "c2CircletoCircle");
                assert_eq!(c_ka(ci, bb), r_ka(ci, bb), "c2CircletoAABB");
                assert_eq!(c_kc(ci, ca), r_kc(ci, ca), "c2CircletoCapsule");

                // c2Collided over the full 3x3 (plus one bad type each round)
                let bufs = [Buf::of(&ci), Buf::of(&bb), Buf::of(&ca)];
                for (i, ta) in TYPES.iter().enumerate() {
                    for (j, tb) in TYPES.iter().enumerate() {
                        assert_eq!(
                            c_co(bufs[i].ptr(), *ta, bufs[j].ptr(), *tb),
                            r_co(bufs[i].ptr(), *ta, bufs[j].ptr(), *tb),
                            "c2Collided({ta},{tb})"
                        );
                    }
                    let bad = 3 + (rng.below(200) as c_int) - 100;
                    assert_eq!(
                        c_co(bufs[i].ptr(), *ta, bufs[0].ptr(), bad),
                        r_co(bufs[i].ptr(), *ta, bufs[0].ptr(), bad),
                        "c2Collided bad typeB={bad}"
                    );
                    assert_eq!(
                        c_co(bufs[i].ptr(), bad, bufs[0].ptr(), *ta),
                        r_co(bufs[i].ptr(), bad, bufs[0].ptr(), *ta),
                        "c2Collided bad typeA={bad}"
                    );
                }

                // c2MakeProxy for all types + one bad type, against a garbage seed
                let mut seed = c2Proxy {
                    radius: rng.wild(),
                    count: rng.next_u32() as c_int,
                    verts: Default::default(),
                };
                for k in 0..8 {
                    seed.verts[k] = rng.v_wild();
                }
                for (i, ty) in TYPES
                    .iter()
                    .copied()
                    .chain([3, -1, i32::MIN].into_iter())
                    .enumerate()
                {
                    let sh = bufs[i.min(2)];
                    let mut cp = seed;
                    let mut rp = seed;
                    c_mp(sh.ptr(), ty, &mut cp);
                    r_mp(sh.ptr(), ty, &mut rp);
                    assert!(
                        raw(&cp) == raw(&rp),
                        "c2MakeProxy(type={ty}) diverged\n C   : {}\n Rust: {}",
                        proxy_hex(&cp),
                        proxy_hex(&rp)
                    );
                }

                // c2BBVerts
                let mut bc = bb;
                let mut br = bb;
                let mut co = [c2v { x: 0.25, y: -0.75 }; 8];
                let mut ro = co;
                c_bv(co.as_mut_ptr(), &mut bc);
                r_bv(ro.as_mut_ptr(), &mut br);
                assert_eq!(raw(&co), raw(&ro), "c2BBVerts");
                assert_eq!(raw(&bc), raw(&br), "c2BBVerts input");

                // capsule()
                let args = [
                    if wild { rng.wild() } else { rng.uniform(-130.0, 60.0) },
                    if wild { rng.wild() } else { rng.uniform(-90.0, 130.0) },
                    if wild { rng.wild() } else { rng.uniform(-130.0, 60.0) },
                    if wild { rng.wild() } else { rng.uniform(-90.0, 130.0) },
                    if wild { rng.wild() } else { rng.uniform(-10.0, 45.0) },
                ];
                assert_eq!(
                    c_cap(args[0], args[1], args[2], args[3], args[4]),
                    r_cap(args[0], args[1], args[2], args[3], args[4]),
                    "capsule({}, {}, {}, {}, {})",
                    f32_hex(args[0]),
                    f32_hex(args[1]),
                    f32_hex(args[2]),
                    f32_hex(args[3]),
                    f32_hex(args[4])
                );
                n += 1;
            }
        }
    }
    eprintln!("fuzz_predicates_and_entry_point: {n} rounds");
    assert!(n > 1000, "fuzz budget produced too few rounds: {n}");
}
