//! Phase B, Group 4 — CONFIGS.md rows C46..C77 (`c2GJK`).
//!
//! `c2GJK` is the lowest-level *composed* entry point: it wires the proxies,
//! the simplex evolution, the support function, the witness computation and the
//! optional warm-start cache together.  Bugs in that pipeline are invisible to
//! the per-function tests in groups 1-3, so every axis is driven here directly
//! through the exported symbol.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 400;

/// A 32-byte, 16-byte-aligned buffer that can hold any of the three shapes.
#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct ShapeBuf {
    bytes: [u8; 32],
}

impl ShapeBuf {
    fn of<T: Copy>(v: &T) -> ShapeBuf {
        assert!(std::mem::size_of::<T>() <= 32);
        let mut b = ShapeBuf { bytes: [0; 32] };
        let src = raw(v);
        b.bytes[..src.len()].copy_from_slice(&src);
        b
    }
    fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr() as *const c_void
    }
}

/// One `c2GJK` configuration.
#[derive(Clone, Copy)]
struct Cfg {
    ax: Option<c2x>,
    bx: Option<c2x>,
    use_radius: c_int,
    cache: Option<c2GJKCache>,
    want_outa: bool,
    want_outb: bool,
    want_iters: bool,
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg {
            ax: None,
            bx: None,
            use_radius: 0,
            cache: None,
            want_outa: true,
            want_outb: true,
            want_iters: true,
        }
    }
}

/// Result of one side of a differential `c2GJK` call.
#[derive(Clone, Copy)]
struct Out {
    rc: f32,
    a: c2v,
    b: c2v,
    iters: c_int,
    cache: c2GJKCache,
}

fn call(f: FnGJK, a: &ShapeBuf, ta: c_int, b: &ShapeBuf, tb: c_int, cfg: &Cfg) -> Out {
    // Sentinels so a *missing* store is detected, not silently accepted.
    let mut oa = c2v { x: 1234.5, y: -6789.0 };
    let mut ob = c2v { x: -4321.0, y: 9876.5 };
    let mut it: c_int = -99;
    let mut cache = cfg.cache.unwrap_or_default();
    let axb = cfg.ax;
    let bxb = cfg.bx;
    let rc = unsafe {
        f(
            a.ptr(),
            ta,
            axb.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            b.ptr(),
            tb,
            bxb.as_ref().map(|x| x as *const c2x).unwrap_or(std::ptr::null()),
            if cfg.want_outa { &mut oa } else { std::ptr::null_mut() },
            if cfg.want_outb { &mut ob } else { std::ptr::null_mut() },
            cfg.use_radius,
            if cfg.want_iters { &mut it } else { std::ptr::null_mut() },
            if cfg.cache.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    Out {
        rc,
        a: oa,
        b: ob,
        iters: it,
        cache,
    }
}

struct Gjk {
    c: FnGJK,
    r: FnGJK,
}

impl Gjk {
    fn new(p: &Pair) -> Gjk {
        Gjk {
            c: p.c.sym("c2GJK"),
            r: p.rs.sym("c2GJK"),
        }
    }
    /// Differential call; returns the (identical) C output so the caller can
    /// use it for coverage accounting.
    fn diff(
        &self,
        a: &ShapeBuf,
        ta: c_int,
        b: &ShapeBuf,
        tb: c_int,
        cfg: &Cfg,
        ctx: &str,
    ) -> Out {
        let co = call(self.c, a, ta, b, tb, cfg);
        let ro = call(self.r, a, ta, b, tb, cfg);
        let same = co.rc.to_bits() == ro.rc.to_bits()
            && raw(&co.a) == raw(&ro.a)
            && raw(&co.b) == raw(&ro.b)
            && co.iters == ro.iters
            && raw(&co.cache) == raw(&ro.cache);
        if !same {
            panic!(
                "DIVERGENCE c2GJK\n  context: {ctx}\n  typeA={ta} typeB={tb} use_radius={} ax={} bx={} cache={}\n  A bytes: {:02x?}\n  B bytes: {:02x?}\n  C   : rc={} outA={} outB={} iters={} cache=[{}]\n  Rust: rc={} outA={} outB={} iters={} cache=[{}]",
                cfg.use_radius,
                cfg.ax.is_some(),
                cfg.bx.is_some(),
                cfg.cache.map(|c| cache_hex(&c)).unwrap_or("NULL".into()),
                &a.bytes[..20],
                &b.bytes[..20],
                f32_hex(co.rc),
                v_hex(&co.a),
                v_hex(&co.b),
                co.iters,
                cache_hex(&co.cache),
                f32_hex(ro.rc),
                v_hex(&ro.a),
                v_hex(&ro.b),
                ro.iters,
                cache_hex(&ro.cache),
            );
        }
        co
    }
}

/// Random shape of the given type, returned as an opaque buffer.
fn rand_shape(rng: &mut Rng, ty: c_int, wild: bool) -> ShapeBuf {
    match ty {
        C2_TYPE_CIRCLE => {
            let mut c = rng.circle();
            if wild {
                c.p = rng.v_wild();
                c.r = rng.wild();
            }
            ShapeBuf::of(&c)
        }
        C2_TYPE_AABB => {
            let mut b = rng.aabb();
            if wild {
                b.min = rng.v_wild();
                b.max = rng.v_wild();
            }
            ShapeBuf::of(&b)
        }
        _ => {
            let mut c = rng.capsule();
            if wild {
                c.a = rng.v_wild();
                c.b = rng.v_wild();
                c.r = rng.wild();
            }
            ShapeBuf::of(&c)
        }
    }
}

const TYPES: [c_int; 3] = [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE];

// ---------------------------------------------------------------------------
// C46..C54 — the full 3x3 type cross-product, minimal options
// ---------------------------------------------------------------------------

#[test]
fn c46_to_c54_type_cross_product() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x46);
    let cfg = Cfg::default();
    for ta in TYPES {
        for tb in TYPES {
            for _ in 0..(N * 8) {
                let a = rand_shape(&mut rng, ta, false);
                let b = rand_shape(&mut rng, tb, false);
                g.diff(&a, ta, &b, tb, &cfg, "C46-54 use_radius=0 no transforms");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C55..C58 — use_radius and the transform matrix of combinations
// ---------------------------------------------------------------------------

#[test]
fn c55_to_c58_radius_and_transforms() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x55);
    for ta in TYPES {
        for tb in TYPES {
            for use_radius in [0, 1] {
                for (have_ax, have_bx) in [(false, false), (true, false), (false, true), (true, true)]
                {
                    for _ in 0..(N * 2) {
                        let a = rand_shape(&mut rng, ta, false);
                        let b = rand_shape(&mut rng, tb, false);
                        let cfg = Cfg {
                            ax: if have_ax { Some(rng.x()) } else { None },
                            bx: if have_bx { Some(rng.x()) } else { None },
                            use_radius,
                            ..Cfg::default()
                        };
                        g.diff(&a, ta, &b, tb, &cfg, "C55-58");
                    }
                }
            }
        }
    }
}

/// C59 — a `c2r` that is deliberately not a unit rotation.
#[test]
fn c59_non_normalised_rotation() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x59);
    for ta in TYPES {
        for tb in TYPES {
            for use_radius in [0, 1] {
                for _ in 0..(N * 2) {
                    let a = rand_shape(&mut rng, ta, false);
                    let b = rand_shape(&mut rng, tb, false);
                    let mk = |rng: &mut Rng| c2x {
                        p: rng.v(),
                        r: c2r {
                            c: rng.uniform(-4.0, 4.0),
                            s: rng.uniform(-4.0, 4.0),
                        },
                    };
                    let cfg = Cfg {
                        ax: Some(mk(&mut rng)),
                        bx: Some(mk(&mut rng)),
                        use_radius,
                        ..Cfg::default()
                    };
                    g.diff(&a, ta, &b, tb, &cfg, "C59 non-normalised rot");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C60..C63 — the warm-start cache
// ---------------------------------------------------------------------------

/// C60 — cold cache (`count == 0`); the C must still *write* the cache on exit.
#[test]
fn c60_cold_cache() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x60);
    for ta in TYPES {
        for tb in TYPES {
            for use_radius in [0, 1] {
                for _ in 0..(N * 2) {
                    let a = rand_shape(&mut rng, ta, false);
                    let b = rand_shape(&mut rng, tb, false);
                    let cfg = Cfg {
                        use_radius,
                        cache: Some(c2GJKCache::default()),
                        ..Cfg::default()
                    };
                    let o = g.diff(&a, ta, &b, tb, &cfg, "C60 cold cache");
                    // the C always writes count/div/metric on the way out
                    assert!(
                        o.cache.count >= 1 && o.cache.count <= 3,
                        "cold cache should come back with 1..3 vertices, got {}",
                        o.cache.count
                    );
                }
            }
        }
    }
}

/// C61/C62 — the real consumer pattern: a warm-start chain, where the shapes
/// move between calls so the cached indices go stale.
#[test]
fn c61_c62_warm_start_chain() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x61);
    for ta in TYPES {
        for tb in TYPES {
            for use_radius in [0, 1] {
                for _ in 0..N {
                    // C61: two calls, identical shapes
                    let a = rand_shape(&mut rng, ta, false);
                    let b = rand_shape(&mut rng, tb, false);
                    let mut cfg = Cfg {
                        use_radius,
                        cache: Some(c2GJKCache::default()),
                        ..Cfg::default()
                    };
                    let o1 = g.diff(&a, ta, &b, tb, &cfg, "C61 warm-start call 1");
                    cfg.cache = Some(o1.cache);
                    let o2 = g.diff(&a, ta, &b, tb, &cfg, "C61 warm-start call 2");
                    cfg.cache = Some(o2.cache);
                    let o3 = g.diff(&a, ta, &b, tb, &cfg, "C61 warm-start call 3");

                    // C62: 5-call chain with the shape *moving* each step
                    let mut chain = Cfg {
                        use_radius,
                        cache: Some(o3.cache),
                        ..Cfg::default()
                    };
                    for step in 0..5 {
                        let b2 = rand_shape(&mut rng, tb, false);
                        let o = g.diff(
                            &a,
                            ta,
                            &b2,
                            tb,
                            &chain,
                            &format!("C62 moving chain step {step}"),
                        );
                        chain.cache = Some(o.cache);
                    }
                }
            }
        }
    }
}

/// C63 — a hand-crafted cache with `count in {1,2,3}` and *valid* indices for
/// the proxy (so no indeterminate proxy slot is read).
#[test]
fn c63_handcrafted_valid_cache() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x63);
    let vcount = |t: c_int| match t {
        C2_TYPE_CIRCLE => 1,
        C2_TYPE_AABB => 4,
        _ => 2,
    };
    for ta in TYPES {
        for tb in TYPES {
            let (na, nb) = (vcount(ta), vcount(tb));
            for count in 1..=3i32 {
                for use_radius in [0, 1] {
                    for _ in 0..(N * 2) {
                        let a = rand_shape(&mut rng, ta, false);
                        let b = rand_shape(&mut rng, tb, false);
                        let mut cache = c2GJKCache {
                            metric: match rng.below(6) {
                                0 => 0.0,
                                1 => -1.0e9, // the only value that can clear the
                                             // `metric < -1.0e8f` conjunct
                                2 => f32::NAN,
                                _ => rng.uniform(-200.0, 200.0),
                            },
                            count,
                            iA: [0; 3],
                            iB: [0; 3],
                            div: match rng.below(4) {
                                0 => 0.0,
                                1 => 1.0,
                                _ => rng.uniform(0.05, 8.0),
                            },
                        };
                        for i in 0..3 {
                            cache.iA[i] = (rng.below(na as u32)) as c_int;
                            cache.iB[i] = (rng.below(nb as u32)) as c_int;
                        }
                        let cfg = Cfg {
                            use_radius,
                            cache: Some(cache),
                            ..Cfg::default()
                        };
                        g.diff(&a, ta, &b, tb, &cfg, "C63 handcrafted cache");
                    }
                }
            }
        }
    }
}

/// C64 — a cache whose `metric` is small enough (`< -1.0e8f`) that the
/// *rejection* branch of the (inverted-looking) validity test is reachable, so
/// both sides of `if (!(min < max*2 && metric < -1.0e8f))` are exercised.
#[test]
fn c64_cache_validity_both_branches() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x64);
    for ta in TYPES {
        for tb in TYPES {
            for count in 1..=3i32 {
                for metric in [
                    -1.0e9f32,
                    -f32::MAX,
                    f32::NEG_INFINITY,
                    -1.00000001e8,
                    -1.0e8,
                    -9.9e7,
                    0.0,
                    f32::NAN,
                    f32::INFINITY,
                ] {
                    for _ in 0..64 {
                        let a = rand_shape(&mut rng, ta, false);
                        let b = rand_shape(&mut rng, tb, false);
                        let nb = match tb {
                            C2_TYPE_CIRCLE => 1u32,
                            C2_TYPE_AABB => 4,
                            _ => 2,
                        };
                        let na = match ta {
                            C2_TYPE_CIRCLE => 1u32,
                            C2_TYPE_AABB => 4,
                            _ => 2,
                        };
                        let mut cache = c2GJKCache {
                            metric,
                            count,
                            iA: [0; 3],
                            iB: [0; 3],
                            div: rng.uniform(0.05, 8.0),
                        };
                        for i in 0..3 {
                            cache.iA[i] = rng.below(na) as c_int;
                            cache.iB[i] = rng.below(nb) as c_int;
                        }
                        for use_radius in [0, 1] {
                            let cfg = Cfg {
                                use_radius,
                                cache: Some(cache),
                                ..Cfg::default()
                            };
                            g.diff(
                                &a,
                                ta,
                                &b,
                                tb,
                                &cfg,
                                &format!("C64 metric={} count={count}", f32_hex(metric)),
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C65 — every NULL / non-NULL combination of the three output parameters
// ---------------------------------------------------------------------------

#[test]
fn c65_output_pointer_null_combinations() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x65);
    for ta in TYPES {
        for tb in TYPES {
            for mask in 0..8u32 {
                for use_radius in [0, 1] {
                    for _ in 0..48 {
                        let a = rand_shape(&mut rng, ta, false);
                        let b = rand_shape(&mut rng, tb, false);
                        let cfg = Cfg {
                            use_radius,
                            want_outa: mask & 1 != 0,
                            want_outb: mask & 2 != 0,
                            want_iters: mask & 4 != 0,
                            ..Cfg::default()
                        };
                        g.diff(&a, ta, &b, tb, &cfg, &format!("C65 mask={mask:03b}"));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C66..C68 — the three special terminations
// ---------------------------------------------------------------------------

#[test]
fn c66_c67_c68_special_terminations() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x66);
    let mut zero_rc = 0usize;
    let mut imm = 0usize;
    for use_radius in [0, 1] {
        // C66: deeply overlapping -> simplex reaches count 3 -> hit
        for _ in 0..(N * 4) {
            let c = rng.v();
            let big = c2AABB {
                min: c2v { x: c.x - 50.0, y: c.y - 50.0 },
                max: c2v { x: c.x + 50.0, y: c.y + 50.0 },
            };
            let inner = c2Capsule {
                a: c2v { x: c.x - 1.0, y: c.y - 1.0 },
                b: c2v { x: c.x + 1.0, y: c.y + 1.0 },
                r: 2.0,
            };
            let cfg = Cfg { use_radius, ..Cfg::default() };
            let o = g.diff(
                &ShapeBuf::of(&big),
                C2_TYPE_AABB,
                &ShapeBuf::of(&inner),
                C2_TYPE_CAPSULE,
                &cfg,
                "C66 deep overlap",
            );
            // Coverage accounting only — the pass/fail decision is the
            // byte-for-byte C-vs-Rust comparison inside `diff`.
            if o.rc.to_bits() == 0 {
                zero_rc += 1;
            }
        }
        // C67: exactly touching (dist == rA + rB) -> midpoint branch
        for _ in 0..(N * 4) {
            let rA = (rng.below(20) + 1) as f32;
            let rB = (rng.below(20) + 1) as f32;
            let y = rng.coord();
            let a = c2Circle { p: c2v { x: 0.0, y }, r: rA };
            let b = c2Circle {
                p: c2v { x: rA + rB, y },
                r: rB,
            };
            let cfg = Cfg { use_radius, ..Cfg::default() };
            g.diff(
                &ShapeBuf::of(&a),
                C2_TYPE_CIRCLE,
                &ShapeBuf::of(&b),
                C2_TYPE_CIRCLE,
                &cfg,
                "C67 exact touch",
            );
        }
        // C68: identical shapes at the identical position
        for ta in TYPES {
            for _ in 0..(N * 2) {
                let s = rand_shape(&mut rng, ta, false);
                let cfg = Cfg { use_radius, ..Cfg::default() };
                let o = g.diff(&s, ta, &s, ta, &cfg, "C68 identical shapes");
                if o.iters == 0 {
                    imm += 1;
                }
            }
        }
    }
    eprintln!("C66 rc==+0.0 cases = {zero_rc}; C68 iters==0 cases = {imm}");
    assert!(zero_rc > 0, "the `hit`/midpoint path (rc == +0.0) was never reached");
    assert!(imm > 0, "identical shapes never terminated on iteration 0");
}

// ---------------------------------------------------------------------------
// C69..C73 — degenerate shapes and radii
// ---------------------------------------------------------------------------

#[test]
fn c69_to_c73_degenerate_shapes() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x69);
    for use_radius in [0, 1] {
        let cfg = Cfg { use_radius, ..Cfg::default() };
        for _ in 0..(N * 4) {
            // C69: zero-length capsules
            let z = rng.v();
            let deg = c2Capsule { a: z, b: z, r: rng.radius() };
            for tb in TYPES {
                let b = rand_shape(&mut rng, tb, false);
                g.diff(&ShapeBuf::of(&deg), C2_TYPE_CAPSULE, &b, tb, &cfg, "C69 A degenerate");
                g.diff(&b, tb, &ShapeBuf::of(&deg), C2_TYPE_CAPSULE, &cfg, "C69 B degenerate");
            }
            let deg2 = c2Capsule {
                a: rng.v(),
                b: z,
                r: rng.radius(),
            };
            let deg2 = c2Capsule { a: deg2.b, b: deg2.b, r: deg2.r };
            g.diff(
                &ShapeBuf::of(&deg),
                C2_TYPE_CAPSULE,
                &ShapeBuf::of(&deg2),
                C2_TYPE_CAPSULE,
                &cfg,
                "C69 both degenerate",
            );

            // C70: zero-area AABBs
            let q = rng.v();
            let flat = c2AABB { min: q, max: q };
            for tb in TYPES {
                let b = rand_shape(&mut rng, tb, false);
                g.diff(&ShapeBuf::of(&flat), C2_TYPE_AABB, &b, tb, &cfg, "C70 A flat");
                g.diff(&b, tb, &ShapeBuf::of(&flat), C2_TYPE_AABB, &cfg, "C70 B flat");
            }

            // C71: inverted AABBs
            let m = rng.v();
            let inv = c2AABB {
                min: c2v { x: m.x + 10.0, y: m.y + 10.0 },
                max: c2v { x: m.x - 10.0, y: m.y - 10.0 },
            };
            for tb in TYPES {
                let b = rand_shape(&mut rng, tb, false);
                g.diff(&ShapeBuf::of(&inv), C2_TYPE_AABB, &b, tb, &cfg, "C71 A inverted");
                g.diff(&b, tb, &ShapeBuf::of(&inv), C2_TYPE_AABB, &cfg, "C71 B inverted");
            }

            // C72 / C73: radius == 0 and radius < 0
            for rv in [0.0f32, -0.0, -1.0, -25.0, -f32::MAX] {
                let c0 = c2Circle { p: rng.v(), r: rv };
                let k0 = c2Capsule { a: rng.v(), b: rng.v(), r: rv };
                let other = rand_shape(&mut rng, C2_TYPE_AABB, false);
                g.diff(
                    &ShapeBuf::of(&c0),
                    C2_TYPE_CIRCLE,
                    &other,
                    C2_TYPE_AABB,
                    &cfg,
                    "C72/73 circle r<=0",
                );
                g.diff(
                    &ShapeBuf::of(&k0),
                    C2_TYPE_CAPSULE,
                    &ShapeBuf::of(&c0),
                    C2_TYPE_CIRCLE,
                    &cfg,
                    "C72/73 capsule r<=0",
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C74..C76 — extreme magnitudes and non-finite input
// ---------------------------------------------------------------------------

#[test]
fn c74_c75_c76_extreme_values() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x74);
    for use_radius in [0, 1] {
        // C74: huge coordinates -> dot() overflows to +inf
        for ta in TYPES {
            for tb in TYPES {
                for _ in 0..(N / 2) {
                    let scale = [1.0e18f32, 1.0e30, f32::MAX, 1.0e-30][rng.below(4) as usize];
                    let mk = |rng: &mut Rng, t: c_int| -> ShapeBuf {
                        let s = |rng: &mut Rng| c2v {
                            x: rng.uniform(-1.0, 1.0) * scale,
                            y: rng.uniform(-1.0, 1.0) * scale,
                        };
                        match t {
                            C2_TYPE_CIRCLE => ShapeBuf::of(&c2Circle {
                                p: s(rng),
                                r: rng.uniform(0.0, 1.0) * scale,
                            }),
                            C2_TYPE_AABB => {
                                let (u, v) = (s(rng), s(rng));
                                ShapeBuf::of(&c2AABB {
                                    min: c2v { x: u.x.min(v.x), y: u.y.min(v.y) },
                                    max: c2v { x: u.x.max(v.x), y: u.y.max(v.y) },
                                })
                            }
                            _ => ShapeBuf::of(&c2Capsule {
                                a: s(rng),
                                b: s(rng),
                                r: rng.uniform(0.0, 1.0) * scale,
                            }),
                        }
                    };
                    let a = mk(&mut rng, ta);
                    let b = mk(&mut rng, tb);
                    let cfg = Cfg { use_radius, ..Cfg::default() };
                    g.diff(&a, ta, &b, tb, &cfg, "C74/75 extreme magnitude");
                }
            }
        }
        // C76: NaN / inf / denormal coordinates and radii
        for ta in TYPES {
            for tb in TYPES {
                for _ in 0..(N * 2) {
                    let a = rand_shape(&mut rng, ta, true);
                    let b = rand_shape(&mut rng, tb, true);
                    let cfg = Cfg {
                        use_radius,
                        ax: if rng.below(3) == 0 { Some(rng.x()) } else { None },
                        bx: if rng.below(3) == 0 { Some(rng.x()) } else { None },
                        ..Cfg::default()
                    };
                    g.diff(&a, ta, &b, tb, &cfg, "C76 non-finite");
                }
            }
        }
        // C76 (systematic): one special value at a time in a circle-vs-circle pair
        for sv in specials() {
            for slot in 0..6 {
                let mut a = c2Circle { p: c2v { x: 3.0, y: 4.0 }, r: 5.0 };
                let mut b = c2Circle { p: c2v { x: -3.0, y: -4.0 }, r: 2.0 };
                match slot {
                    0 => a.p.x = sv,
                    1 => a.p.y = sv,
                    2 => a.r = sv,
                    3 => b.p.x = sv,
                    4 => b.p.y = sv,
                    _ => b.r = sv,
                }
                let cfg = Cfg { use_radius, ..Cfg::default() };
                g.diff(
                    &ShapeBuf::of(&a),
                    C2_TYPE_CIRCLE,
                    &ShapeBuf::of(&b),
                    C2_TYPE_CIRCLE,
                    &cfg,
                    &format!("C76 systematic slot={slot} v={}", f32_hex(sv)),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C77 — iteration-count / terminator agreement across a big random sweep
// ---------------------------------------------------------------------------

#[test]
fn c77_iteration_terminator_agreement() {
    let p = load();
    let g = Gjk::new(&p);
    let mut rng = Rng::new(0x77);
    // histogram of *iterations to prove the sweep really reaches several
    // different terminators (0 == immediate, 20 == iteration cap)
    let mut hist = [0usize; 22];
    for ta in TYPES {
        for tb in TYPES {
            for use_radius in [0, 1] {
                for _ in 0..(N * 6) {
                    let a = rand_shape(&mut rng, ta, false);
                    let b = rand_shape(&mut rng, tb, false);
                    let cfg = Cfg {
                        use_radius,
                        ax: if rng.below(2) == 0 { Some(rng.x()) } else { None },
                        bx: if rng.below(2) == 0 { Some(rng.x()) } else { None },
                        cache: if rng.below(2) == 0 {
                            Some(c2GJKCache::default())
                        } else {
                            None
                        },
                        ..Cfg::default()
                    };
                    let o = g.diff(&a, ta, &b, tb, &cfg, "C77 sweep");
                    let idx = o.iters.clamp(0, 21) as usize;
                    hist[idx] += 1;
                }
            }
        }
    }
    eprintln!("C77 *iterations histogram = {hist:?}");
    let distinct = hist.iter().filter(|&&n| n > 0).count();
    assert!(
        distinct >= 3,
        "sweep only ever produced {distinct} distinct iteration counts: {hist:?}"
    );
}
