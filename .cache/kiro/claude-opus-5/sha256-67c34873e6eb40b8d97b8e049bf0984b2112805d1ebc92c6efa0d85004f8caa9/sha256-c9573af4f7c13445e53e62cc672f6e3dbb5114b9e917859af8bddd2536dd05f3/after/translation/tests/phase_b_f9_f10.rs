//! Phase B — valid-path differential tests for `f9` (barycentric) and `f10`
//! (half-float decode). CONFIGS.md rows C48-C55.

mod common;

use common::*;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

const N: usize = 6000;

fn v(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

fn f9_sweep(tag: &str, gen: impl Fn(&mut Rng) -> [LmVec2; 4], n: usize) {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let mut g = Rng::seeded();
    for i in 0..n {
        let [p1, p2, p3, p] = gen(&mut g);
        unsafe {
            eq_lmvec2(
                &format!("{tag} f9 #{i} p1={p1:?} p2={p2:?} p3={p3:?} p={p:?}"),
                c(p1, p2, p3, p),
                r(p1, p2, p3, p),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// C48-C53 — f9
// ---------------------------------------------------------------------------

#[test]
fn c48_f9_point_inside_triangle() {
    f9_sweep(
        "C48",
        |g| {
            let p1 = v(g.finite_f32(10.0), g.finite_f32(10.0));
            let p2 = v(p1.x + g.range_f32(1.0, 10.0), p1.y + g.range_f32(-5.0, 5.0));
            let p3 = v(p1.x + g.range_f32(-5.0, 5.0), p1.y + g.range_f32(1.0, 10.0));
            // convex combination -> strictly inside
            let mut a = g.range_f32(0.05, 0.9);
            let mut b = g.range_f32(0.05, 0.9);
            if a + b > 0.95 {
                a *= 0.5;
                b *= 0.5;
            }
            let p = v(
                p1.x + a * (p2.x - p1.x) + b * (p3.x - p1.x),
                p1.y + a * (p2.y - p1.y) + b * (p3.y - p1.y),
            );
            [p1, p2, p3, p]
        },
        N,
    );
}

#[test]
fn c49_f9_point_outside_triangle() {
    f9_sweep(
        "C49",
        |g| {
            let p1 = v(g.finite_f32(10.0), g.finite_f32(10.0));
            let p2 = v(p1.x + g.range_f32(1.0, 10.0), p1.y + g.range_f32(-5.0, 5.0));
            let p3 = v(p1.x + g.range_f32(-5.0, 5.0), p1.y + g.range_f32(1.0, 10.0));
            // barycentrics well outside [0,1]
            let a = g.range_f32(-4.0, 5.0);
            let b = g.range_f32(-4.0, 5.0);
            let p = v(
                p1.x + a * (p2.x - p1.x) + b * (p3.x - p1.x),
                p1.y + a * (p2.y - p1.y) + b * (p3.y - p1.y),
            );
            [p1, p2, p3, p]
        },
        N,
    );
}

#[test]
fn c50_f9_point_at_vertices() {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let mut g = Rng::seeded();
    for i in 0..N {
        let p1 = v(g.finite_f32(10.0), g.finite_f32(10.0));
        let p2 = v(p1.x + g.range_f32(1.0, 10.0), p1.y + g.range_f32(-5.0, 5.0));
        let p3 = v(p1.x + g.range_f32(-5.0, 5.0), p1.y + g.range_f32(1.0, 10.0));
        for (k, p) in [p1, p2, p3].iter().enumerate() {
            unsafe {
                eq_lmvec2(
                    &format!("C50 f9 vertex{k} #{i}"),
                    c(p1, p2, p3, *p),
                    r(p1, p2, p3, *p),
                )
            }
        }
        // edge midpoints too
        let mids = [
            v((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5),
            v((p2.x + p3.x) * 0.5, (p2.y + p3.y) * 0.5),
            v((p3.x + p1.x) * 0.5, (p3.y + p1.y) * 0.5),
        ];
        for (k, p) in mids.iter().enumerate() {
            unsafe {
                eq_lmvec2(
                    &format!("C50 f9 mid{k} #{i}"),
                    c(p1, p2, p3, *p),
                    r(p1, p2, p3, *p),
                )
            }
        }
    }
}

#[test]
fn c51_f9_degenerate_triangles() {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = v(g.finite_f32(10.0), g.finite_f32(10.0));
        let p = v(g.finite_f32(10.0), g.finite_f32(10.0));
        // all three coincident -> denominator 0
        unsafe { eq_lmvec2(&format!("C51 coincident #{i}"), c(a, a, a, p), r(a, a, a, p)) }
        // two coincident
        let b = v(g.finite_f32(10.0), g.finite_f32(10.0));
        unsafe {
            eq_lmvec2(&format!("C51 p1==p2 #{i}"), c(a, a, b, p), r(a, a, b, p));
            eq_lmvec2(&format!("C51 p1==p3 #{i}"), c(a, b, a, p), r(a, b, a, p));
            eq_lmvec2(&format!("C51 p2==p3 #{i}"), c(a, b, b, p), r(a, b, b, p));
        }
        // collinear: p3 = p1 + t*(p2 - p1)
        let t = g.range_f32(-3.0, 3.0);
        let col = v(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        unsafe {
            eq_lmvec2(&format!("C51 collinear #{i}"), c(a, b, col, p), r(a, b, col, p))
        }
    }
}

#[test]
fn c52_f9_full_random_f32() {
    // Random bit patterns for all 8 coordinates: exercises huge/tiny
    // magnitudes, subnormals, inf and NaN in every position at once.
    f9_sweep(
        "C52 anybits",
        |g| {
            [
                v(g.any_f32(), g.any_f32()),
                v(g.any_f32(), g.any_f32()),
                v(g.any_f32(), g.any_f32()),
                v(g.any_f32(), g.any_f32()),
            ]
        },
        N * 3,
    );
    f9_sweep(
        "C52 mixed",
        |g| {
            [
                v(g.mixed_f32(), g.mixed_f32()),
                v(g.mixed_f32(), g.mixed_f32()),
                v(g.mixed_f32(), g.mixed_f32()),
                v(g.mixed_f32(), g.mixed_f32()),
            ]
        },
        N * 3,
    );
    // extreme-magnitude finite triangles (overflow in the dot products)
    f9_sweep(
        "C52 huge",
        |g| {
            let s = 1e19f32;
            [
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
            ]
        },
        N,
    );
    // tiny-magnitude (underflow to subnormal / zero denominators)
    f9_sweep(
        "C52 tiny",
        |g| {
            let s = 1e-22f32;
            [
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
                v(g.finite_f32(s), g.finite_f32(s)),
            ]
        },
        N,
    );
}

#[test]
fn c53_f9_nan_in_each_position() {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFF80_0002),
        f32::from_bits(0xFFD5_5555),
    ];
    let base = [v(0.0, 0.0), v(3.0, 0.0), v(0.0, 4.0), v(1.0, 1.0)];
    for &n in &nans {
        for slot in 0..8 {
            let mut pts = base;
            let (pi, ci) = (slot / 2, slot % 2);
            if ci == 0 {
                pts[pi].x = n;
            } else {
                pts[pi].y = n;
            }
            unsafe {
                eq_lmvec2(
                    &format!("C53 f9 nan 0x{:08x} slot{slot}", n.to_bits()),
                    c(pts[0], pts[1], pts[2], pts[3]),
                    r(pts[0], pts[1], pts[2], pts[3]),
                )
            }
        }
        // two NaNs with different payloads simultaneously
        for &m in &nans {
            let pts = [v(n, m), v(m, n), v(n, n), v(m, m)];
            unsafe {
                eq_lmvec2(
                    &format!("C53 f9 dual 0x{:08x}/0x{:08x}", n.to_bits(), m.to_bits()),
                    c(pts[0], pts[1], pts[2], pts[3]),
                    r(pts[0], pts[1], pts[2], pts[3]),
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C54, C55 — f10, exhaustive
// ---------------------------------------------------------------------------

#[test]
fn c54_f10_exhaustive_all_65536() {
    let l = libs();
    let (c, r) = bind!(l, "f10", FnF10);
    for h in 0u16..=u16::MAX {
        unsafe { eq_f32(&format!("C54 f10(0x{h:04x})"), c(h), r(h)) }
        if h == u16::MAX {
            break;
        }
    }
}

#[test]
fn c55_f10_exponent_classes() {
    let l = libs();
    let (c, r) = bind!(l, "f10", FnF10);
    // n = h >> 10 selects the m__offset / m__exponent entry.
    // Verify every class explicitly, including the boundary mantissas.
    for n in 0u16..64 {
        for m in [0u16, 1, 2, 511, 512, 1021, 1022, 1023] {
            let h = (n << 10) | m;
            unsafe {
                eq_f32(
                    &format!("C55 f10 n={n} m={m} (0x{h:04x})"),
                    c(h),
                    r(h),
                )
            }
        }
    }
    // half-precision special encodings
    for &h in &[
        0x0000u16, // +0
        0x8000,    // -0
        0x0001,    // smallest positive subnormal
        0x03FF,    // largest subnormal
        0x0400,    // smallest normal
        0x3C00,    // 1.0
        0x7BFF,    // largest finite
        0x7C00,    // +inf
        0x7C01,    // NaN
        0x7E00,    // qNaN
        0x7FFF,    // NaN, max payload
        0xFC00,    // -inf
        0xFC01,    // -NaN
        0xFFFF,    // -NaN max payload
        0xBC00,    // -1.0
    ] {
        unsafe { eq_f32(&format!("C55 f10 special 0x{h:04x}"), c(h), r(h)) }
    }
    // no input may index out of m__mantissa[2048]: if it did, the C would
    // read garbage and the two would diverge somewhere in C54. Assert the
    // index bound holds structurally too.
    let mut g = Rng::seeded();
    for _ in 0..20000 {
        let h = g.next_u16();
        unsafe { eq_f32(&format!("C55 f10 rnd 0x{h:04x}"), c(h), r(h)) }
    }
}
