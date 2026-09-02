//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlsym`ed exports of both `.so`s and every comparison
//! is on raw bit patterns, so `-0.0` vs `+0.0` and differing NaN payloads count
//! as divergences.
#![allow(non_snake_case)]

mod harness;
use harness::*;

/// Iterations per randomized row. Cheap enough to keep the whole suite well
/// under the time budget while sampling every float class many times over.
const N: usize = 4000;

// ---------------------------------------------------------------------------
// assertion helpers
// ---------------------------------------------------------------------------

fn cmp_v(label: &str, ctx: &str, cv: C2v, rv: C2v) {
    assert!(
        same_v(cv, rv),
        "{label} DIVERGED\n  input : {ctx}\n  C     : {}\n  Rust  : {}",
        fmt_v(cv),
        fmt_v(rv)
    );
}

fn cmp_f(label: &str, ctx: &str, cf: f32, rf: f32) {
    assert!(
        same_f(cf, rf),
        "{label} DIVERGED\n  input : {ctx}\n  C     : {:e} / {:#010x}\n  Rust  : {:e} / {:#010x}",
        cf,
        cf.to_bits(),
        rf,
        rf.to_bits()
    );
}

fn cmp_i(label: &str, ctx: &str, ci: i32, ri: i32) {
    assert!(ci == ri, "{label} DIVERGED\n  input : {ctx}\n  C     : {ci}\n  Rust  : {ri}");
}

// ---------------------------------------------------------------------------
// Row 1-2 : c2V
// ---------------------------------------------------------------------------

#[test]
fn cfg_row01_c2v_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        let ctx = format!("#{i} x={:#010x} y={:#010x}", x.to_bits(), y.to_bits());
        cmp_v("c2V", &ctx, (c.c2V)(x, y), (r.c2V)(x, y));
    }
}

#[test]
fn cfg_row02_c2v_boundary_corpus() {
    let (c, r) = both();
    let sp = specials();
    for &x in &sp {
        for &y in &sp {
            let ctx = format!("x={:#010x} y={:#010x}", x.to_bits(), y.to_bits());
            cmp_v("c2V", &ctx, (c.c2V)(x, y), (r.c2V)(x, y));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3-6 : c2Maxv / c2Minv
// ---------------------------------------------------------------------------

#[test]
fn cfg_row03_c2maxv_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        let ctx = format!("#{i} a={} b={}", fmt_v(a), fmt_v(b));
        cmp_v("c2Maxv", &ctx, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
    }
}

#[test]
fn cfg_row04_c2maxv_boundary_cross() {
    let (c, r) = both();
    let sp = specials();
    // Cross every special against every special in BOTH components, so NaN-vs-NaN,
    // +0-vs--0 and inf-vs-inf all appear on each axis.
    for &ax in &sp {
        for &bx in &sp {
            for (i, &ay) in sp.iter().enumerate() {
                let by = sp[(i + 7) % sp.len()];
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: by };
                let ctx = format!("a={} b={}", fmt_v(a), fmt_v(b));
                cmp_v("c2Maxv", &ctx, (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            }
        }
    }
}

#[test]
fn cfg_row05_c2minv_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        let ctx = format!("#{i} a={} b={}", fmt_v(a), fmt_v(b));
        cmp_v("c2Minv", &ctx, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
    }
}

#[test]
fn cfg_row06_c2minv_boundary_cross() {
    let (c, r) = both();
    let sp = specials();
    for &ax in &sp {
        for &bx in &sp {
            for (i, &ay) in sp.iter().enumerate() {
                let by = sp[(i + 7) % sp.len()];
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: by };
                let ctx = format!("a={} b={}", fmt_v(a), fmt_v(b));
                cmp_v("c2Minv", &ctx, (c.c2Minv)(a, b), (r.c2Minv)(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7-8 : c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn cfg_row07_c2clampv_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        // Fully unconstrained: lo > hi (inverted range) occurs naturally, and the
        // C never orders them, so this is a valid input shape.
        let (a, lo, hi) = (rng.any_v(), rng.any_v(), rng.any_v());
        let ctx = format!("#{i} a={} lo={} hi={}", fmt_v(a), fmt_v(lo), fmt_v(hi));
        cmp_v("c2Clampv", &ctx, (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
    }
    // Explicit ordered / inverted / degenerate ranges with finite geometry.
    for i in 0..N {
        let a = rng.finite_v(100.0);
        let p = rng.finite_v(100.0);
        let q = rng.finite_v(100.0);
        let ordered = C2Aabb {
            min: C2v { x: p.x.min(q.x), y: p.y.min(q.y) },
            max: C2v { x: p.x.max(q.x), y: p.y.max(q.y) },
        };
        for (tag, lo, hi) in [
            ("ordered", ordered.min, ordered.max),
            ("inverted", ordered.max, ordered.min),
            ("degenerate", ordered.min, ordered.min),
        ] {
            let ctx = format!("#{i} {tag} a={} lo={} hi={}", fmt_v(a), fmt_v(lo), fmt_v(hi));
            cmp_v("c2Clampv", &ctx, (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
        }
    }
}

#[test]
fn cfg_row08_c2clampv_boundary_cross() {
    let (c, r) = both();
    let sp = specials();
    // Triple cross-product on the x axis (with a rotated y axis so y is covered
    // too): NaN appears in each of the three positions in turn.
    for (i, &ax) in sp.iter().enumerate() {
        for (j, &lox) in sp.iter().enumerate() {
            for (k, &hix) in sp.iter().enumerate() {
                let a = C2v { x: ax, y: sp[(i + 3) % sp.len()] };
                let lo = C2v { x: lox, y: sp[(j + 11) % sp.len()] };
                let hi = C2v { x: hix, y: sp[(k + 5) % sp.len()] };
                let ctx = format!("a={} lo={} hi={}", fmt_v(a), fmt_v(lo), fmt_v(hi));
                cmp_v("c2Clampv", &ctx, (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9-10 : c2Sub
// ---------------------------------------------------------------------------

#[test]
fn cfg_row09_c2sub_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        let ctx = format!("#{i} a={} b={}", fmt_v(a), fmt_v(b));
        cmp_v("c2Sub", &ctx, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
    }
}

#[test]
fn cfg_row10_c2sub_boundary_cross() {
    let (c, r) = both();
    let sp = specials();
    for (i, &ax) in sp.iter().enumerate() {
        for (j, &bx) in sp.iter().enumerate() {
            // Rotate the y lanes so every (y_a, y_b) special pair also occurs.
            let a = C2v { x: ax, y: sp[(j + 1) % sp.len()] };
            let b = C2v { x: bx, y: sp[(i + 13) % sp.len()] };
            let ctx = format!("a={} b={}", fmt_v(a), fmt_v(b));
            cmp_v("c2Sub", &ctx, (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        }
    }
    // Named invalid-operation and over/underflow cases.
    let inf = f32::INFINITY;
    let cases: &[(&str, C2v, C2v)] = &[
        ("inf-inf", C2v { x: inf, y: inf }, C2v { x: inf, y: inf }),
        ("-inf-(-inf)", C2v { x: -inf, y: -inf }, C2v { x: -inf, y: -inf }),
        ("inf-(-inf)", C2v { x: inf, y: inf }, C2v { x: -inf, y: -inf }),
        ("0-0", C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),
        ("-0-0", C2v { x: -0.0, y: -0.0 }, C2v { x: 0.0, y: 0.0 }),
        ("0-(-0)", C2v { x: 0.0, y: 0.0 }, C2v { x: -0.0, y: -0.0 }),
        (
            "overflow",
            C2v { x: f32::MAX, y: f32::MAX },
            C2v { x: -f32::MAX, y: -f32::MAX },
        ),
        (
            "underflow",
            C2v { x: f32::from_bits(0x0080_0001), y: f32::from_bits(0x0080_0000) },
            C2v { x: f32::from_bits(0x0080_0000), y: f32::from_bits(0x0080_0001) },
        ),
    ];
    for (tag, a, b) in cases {
        let ctx = format!("{tag} a={} b={}", fmt_v(*a), fmt_v(*b));
        cmp_v("c2Sub", &ctx, (c.c2Sub)(*a, *b), (r.c2Sub)(*a, *b));
    }
}

// ---------------------------------------------------------------------------
// Row 11-12 : c2Dot
// ---------------------------------------------------------------------------

#[test]
fn cfg_row11_c2dot_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (a, b) = (rng.any_v(), rng.any_v());
        let ctx = format!("#{i} a={} b={}", fmt_v(a), fmt_v(b));
        cmp_f("c2Dot", &ctx, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
    }
    // Finite magnitudes too, where the result is an ordinary number and any
    // operand-order or rounding mistake shows up as a plain value mismatch.
    for i in 0..N {
        let (a, b) = (rng.finite_v(1.0e18), rng.finite_v(1.0e18));
        let ctx = format!("#{i} finite a={} b={}", fmt_v(a), fmt_v(b));
        cmp_f("c2Dot", &ctx, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
    }
}

#[test]
fn cfg_row12_c2dot_boundary_cross() {
    let (c, r) = both();
    let sp = specials();
    for (i, &ax) in sp.iter().enumerate() {
        for (j, &bx) in sp.iter().enumerate() {
            for k in 0..sp.len() {
                let a = C2v { x: ax, y: sp[k] };
                let b = C2v { x: bx, y: sp[(k + i + j) % sp.len()] };
                let ctx = format!("a={} b={}", fmt_v(a), fmt_v(b));
                cmp_f("c2Dot", &ctx, (c.c2Dot)(a, b), (r.c2Dot)(a, b));
            }
        }
    }
    let inf = f32::INFINITY;
    let nan_a = f32::from_bits(0x7FC0_1234);
    let nan_b = f32::from_bits(0x7FDE_ADBE);
    let snan = f32::from_bits(0x7F80_0001);
    let cases: &[(&str, C2v, C2v)] = &[
        // 0 * inf in the first product, then a finite second product.
        ("0*inf,+1", C2v { x: 0.0, y: 1.0 }, C2v { x: inf, y: 1.0 }),
        ("inf*0,+1", C2v { x: inf, y: 1.0 }, C2v { x: 0.0, y: 1.0 }),
        // Both products NaN with DIFFERENT payloads: which one survives is
        // decided purely by the pinned addss operand order.
        ("nanA x nanB", C2v { x: nan_a, y: nan_b }, C2v { x: 1.0, y: 1.0 }),
        ("nanB x nanA", C2v { x: nan_b, y: nan_a }, C2v { x: 1.0, y: 1.0 }),
        // SNaN must be quieted exactly as SSE does.
        ("snan x qnan", C2v { x: snan, y: nan_a }, C2v { x: 1.0, y: 1.0 }),
        ("qnan x snan", C2v { x: nan_a, y: snan }, C2v { x: 1.0, y: 1.0 }),
        // inf + -inf across the two products.
        ("inf + -inf", C2v { x: inf, y: inf }, C2v { x: 1.0, y: -1.0 }),
        ("-inf + inf", C2v { x: inf, y: inf }, C2v { x: -1.0, y: 1.0 }),
        // Overflow / underflow of the products themselves.
        ("overflow", C2v { x: f32::MAX, y: f32::MAX }, C2v { x: f32::MAX, y: f32::MAX }),
        (
            "underflow",
            C2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE },
            C2v { x: f32::MIN_POSITIVE, y: f32::MIN_POSITIVE },
        ),
        // Cancellation to signed zero.
        ("cancel", C2v { x: 1.0, y: 1.0 }, C2v { x: 1.0, y: -1.0 }),
        ("neg zero", C2v { x: -0.0, y: -0.0 }, C2v { x: 0.0, y: 0.0 }),
    ];
    for (tag, a, b) in cases {
        let ctx = format!("{tag} a={} b={}", fmt_v(*a), fmt_v(*b));
        cmp_f("c2Dot", &ctx, (c.c2Dot)(*a, *b), (r.c2Dot)(*a, *b));
    }
}

// ---------------------------------------------------------------------------
// helpers for the pointer-taking `collided` entry point
// ---------------------------------------------------------------------------

/// A 16-byte scratch buffer.
///
/// `collided` reinterprets the same pointer as either a 12-byte `c2Circle` or a
/// 16-byte `c2AABB` depending on the tag, so every buffer handed to it is sized
/// for the LARGER of the two. That keeps the C's unconditional deref in-bounds
/// even when a tag disagrees with how the buffer was filled — which is exactly
/// what rows 29/30 exercise.
#[repr(C)]
#[derive(Copy, Clone)]
struct Buf([u32; 4]);

impl Buf {
    fn circle(c: C2Circle) -> Buf {
        Buf([c.p.x.to_bits(), c.p.y.to_bits(), c.r.to_bits(), 0])
    }
    fn aabb(b: C2Aabb) -> Buf {
        Buf([b.min.x.to_bits(), b.min.y.to_bits(), b.max.x.to_bits(), b.max.y.to_bits()])
    }
    fn as_circle(&self) -> C2Circle {
        C2Circle {
            p: C2v { x: f32::from_bits(self.0[0]), y: f32::from_bits(self.0[1]) },
            r: f32::from_bits(self.0[2]),
        }
    }
    fn as_aabb(&self) -> C2Aabb {
        C2Aabb {
            min: C2v { x: f32::from_bits(self.0[0]), y: f32::from_bits(self.0[1]) },
            max: C2v { x: f32::from_bits(self.0[2]), y: f32::from_bits(self.0[3]) },
        }
    }
    fn ptr(&self) -> *const std::ffi::c_void {
        self.0.as_ptr() as *const std::ffi::c_void
    }
}

/// Calls `collided` on both libraries with the same buffers and compares.
fn cmp_collided(ctx: &str, a: &Buf, ta: i32, b: &Buf, tb: i32) -> i32 {
    let (c, r) = both();
    let cv = unsafe { (c.collided)(a.ptr(), ta, b.ptr(), tb) };
    let rv = unsafe { (r.collided)(a.ptr(), ta, b.ptr(), tb) };
    cmp_i("collided", ctx, cv, rv);
    cv
}

// ---------------------------------------------------------------------------
// Row 13-15 : c2CircletoCircle (direct)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_circle_circle_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (A, B) = (rng.any_circle(), rng.any_circle());
        let ctx = format!("#{i} A={} B={}", fmt_c(A), fmt_c(B));
        cmp_i("c2CircletoCircle", &ctx, (c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));
    }
}

#[test]
fn cfg_row14_circle_circle_random_geometry() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let mut hits = 0usize;
    for i in 0..N {
        let A = rng.finite_circle(100.0, 10.0);
        let B = rng.finite_circle(100.0, 10.0);
        let ctx = format!("#{i} A={} B={}", fmt_c(A), fmt_c(B));
        let v = (c.c2CircletoCircle)(A, B);
        cmp_i("c2CircletoCircle", &ctx, v, (r.c2CircletoCircle)(A, B));
        hits += (v != 0) as usize;
    }
    // Guard against a vacuous row: some pairs must overlap and some must not,
    // otherwise only one branch of the predicate was ever taken.
    assert!(hits > 0 && hits < N, "row 14 exercised only one branch ({hits}/{N} overlaps)");
}

#[test]
fn cfg_row15_circle_circle_boundaries() {
    let (c, r) = both();
    let inf = f32::INFINITY;
    let nan = f32::NAN;
    let v = |x: f32, y: f32| C2v { x, y };
    let cir = |x: f32, y: f32, r: f32| C2Circle { p: v(x, y), r };
    let cases: &[(&str, C2Circle, C2Circle)] = &[
        // d2 == r2 exactly -> the strict `<` must be false.
        ("touching", cir(0.0, 0.0, 1.0), cir(2.0, 0.0, 1.0)),
        ("touching diag", cir(0.0, 0.0, 3.0), cir(3.0, 4.0, 2.0)),
        ("1ulp inside", cir(0.0, 0.0, 1.0), cir(f32::from_bits(0x3FFF_FFFF), 0.0, 1.0)),
        ("1ulp outside", cir(0.0, 0.0, 1.0), cir(f32::from_bits(0x4000_0001), 0.0, 1.0)),
        ("concentric", cir(5.0, 5.0, 1.0), cir(5.0, 5.0, 2.0)),
        ("identical", cir(1.0, 2.0, 3.0), cir(1.0, 2.0, 3.0)),
        ("zero radius both", cir(0.0, 0.0, 0.0), cir(0.0, 0.0, 0.0)),
        ("zero radius apart", cir(0.0, 0.0, 0.0), cir(1.0, 0.0, 0.0)),
        // Negative radii: r2 = (A.r+B.r)^2 is still positive, so the C can report
        // an overlap for negative radii. Reproduce, do not "fix".
        ("negative radius", cir(0.0, 0.0, -1.0), cir(1.0, 0.0, -1.0)),
        ("negative sum", cir(0.0, 0.0, -5.0), cir(1.0, 0.0, 2.0)),
        ("neg cancels", cir(0.0, 0.0, -1.0), cir(0.5, 0.0, 1.0)),
        // Values that push the arithmetic out of range.
        ("radius overflow", cir(0.0, 0.0, f32::MAX), cir(1.0, 0.0, f32::MAX)),
        ("pos overflow", cir(-f32::MAX, 0.0, 1.0), cir(f32::MAX, 0.0, 1.0)),
        ("radius inf", cir(0.0, 0.0, inf), cir(1.0, 0.0, 1.0)),
        ("radius inf + -inf", cir(0.0, 0.0, inf), cir(1.0, 0.0, -inf)),
        ("pos inf", cir(inf, 0.0, 1.0), cir(inf, 0.0, 1.0)),
        ("radius nan", cir(0.0, 0.0, nan), cir(1.0, 0.0, 1.0)),
        ("radius snan", cir(0.0, 0.0, f32::from_bits(0x7F80_0001)), cir(1.0, 0.0, 1.0)),
        ("pos nan", cir(nan, 0.0, 1.0), cir(0.0, 0.0, 1.0)),
        (
            "subnormal",
            cir(0.0, 0.0, f32::from_bits(1)),
            cir(f32::from_bits(1), 0.0, f32::from_bits(1)),
        ),
        ("neg zero pos", cir(-0.0, -0.0, 1.0), cir(0.0, 0.0, 1.0)),
    ];
    for (tag, A, B) in cases {
        let ctx = format!("{tag} A={} B={}", fmt_c(*A), fmt_c(*B));
        cmp_i("c2CircletoCircle", &ctx, (c.c2CircletoCircle)(*A, *B), (r.c2CircletoCircle)(*A, *B));
        // ...and with the arguments swapped, since `c2Sub(B.p, A.p)` and
        // `A.r + B.r` both pin an operand order.
        let ctx = format!("{tag} swapped A={} B={}", fmt_c(*B), fmt_c(*A));
        cmp_i("c2CircletoCircle", &ctx, (c.c2CircletoCircle)(*B, *A), (r.c2CircletoCircle)(*B, *A));
    }
}

// ---------------------------------------------------------------------------
// Row 16-19 : c2CircletoAABB (direct)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row16_circle_aabb_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let A = rng.any_circle();
        let B = rng.any_aabb();
        let ctx = format!("#{i} A={} B={}", fmt_c(A), fmt_b(B));
        cmp_i("c2CircletoAABB", &ctx, (c.c2CircletoAABB)(A, B), (r.c2CircletoAABB)(A, B));
    }
}

#[test]
fn cfg_row17_circle_aabb_random_geometry() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let mut hits = 0usize;
    for i in 0..N {
        let A = rng.finite_circle(100.0, 20.0);
        let B = rng.ordered_aabb(100.0);
        let ctx = format!("#{i} A={} B={}", fmt_c(A), fmt_b(B));
        let v = (c.c2CircletoAABB)(A, B);
        cmp_i("c2CircletoAABB", &ctx, v, (r.c2CircletoAABB)(A, B));
        hits += (v != 0) as usize;
    }
    assert!(hits > 0 && hits < N, "row 17 exercised only one branch ({hits}/{N})");
}

#[test]
fn cfg_row18_circle_aabb_inverted_box() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let A = rng.finite_circle(100.0, 20.0);
        let B = rng.inverted_aabb(100.0);
        let ctx = format!("#{i} A={} B(inverted)={}", fmt_c(A), fmt_b(B));
        cmp_i("c2CircletoAABB", &ctx, (c.c2CircletoAABB)(A, B), (r.c2CircletoAABB)(A, B));
    }
    // Inverted on the x axis only.
    for i in 0..N {
        let A = rng.finite_circle(100.0, 20.0);
        let ok = rng.ordered_aabb(100.0);
        let B = C2Aabb {
            min: C2v { x: ok.max.x, y: ok.min.y },
            max: C2v { x: ok.min.x, y: ok.max.y },
        };
        let ctx = format!("#{i} A={} B(x-inverted)={}", fmt_c(A), fmt_b(B));
        cmp_i("c2CircletoAABB", &ctx, (c.c2CircletoAABB)(A, B), (r.c2CircletoAABB)(A, B));
    }
}

#[test]
fn cfg_row19_circle_aabb_boundaries() {
    let (c, r) = both();
    let inf = f32::INFINITY;
    let nan = f32::NAN;
    let v = |x: f32, y: f32| C2v { x, y };
    let cir = |x: f32, y: f32, rr: f32| C2Circle { p: v(x, y), r: rr };
    let bx = |a: f32, b: f32, cc: f32, d: f32| C2Aabb { min: v(a, b), max: v(cc, d) };
    let unit = bx(-1.0, -1.0, 1.0, 1.0);
    let cases: &[(&str, C2Circle, C2Aabb)] = &[
        ("centre inside", cir(0.0, 0.0, 0.5), unit),
        // Centre inside => clamp is the identity => d2 == 0 => `0 < r2`, which is
        // FALSE for r == 0: a zero-radius circle at the box centre "misses".
        ("centre inside r=0", cir(0.0, 0.0, 0.0), unit),
        ("centre on edge", cir(1.0, 0.0, 0.5), unit),
        ("centre on corner", cir(1.0, 1.0, 0.5), unit),
        ("centre on corner r=0", cir(1.0, 1.0, 0.0), unit),
        ("edge touching", cir(2.0, 0.0, 1.0), unit),
        ("edge 1ulp inside", cir(f32::from_bits(0x3FFF_FFFF), 0.0, 1.0), unit),
        ("edge 1ulp outside", cir(f32::from_bits(0x4000_0001), 0.0, 1.0), unit),
        ("corner touching", cir(4.0, 4.0, 5.0), unit),
        ("far away", cir(100.0, 100.0, 1.0), unit),
        ("negative radius", cir(2.0, 0.0, -1.0), unit),
        ("negative radius inside", cir(0.0, 0.0, -1.0), unit),
        ("zero-area box", cir(0.0, 0.0, 1.0), bx(0.0, 0.0, 0.0, 0.0)),
        ("zero-area box r=0", cir(0.0, 0.0, 0.0), bx(0.0, 0.0, 0.0, 0.0)),
        ("inverted box", cir(0.0, 0.0, 1.0), bx(1.0, 1.0, -1.0, -1.0)),
        ("inverted box far", cir(50.0, 50.0, 1.0), bx(1.0, 1.0, -1.0, -1.0)),
        ("infinite box", cir(0.0, 0.0, 1.0), bx(-inf, -inf, inf, inf)),
        ("empty inf box", cir(0.0, 0.0, 1.0), bx(inf, inf, -inf, -inf)),
        ("inf centre", cir(inf, inf, 1.0), unit),
        ("inf radius", cir(0.0, 0.0, inf), unit),
        ("nan radius", cir(0.0, 0.0, nan), unit),
        ("nan centre x", cir(nan, 0.0, 1.0), unit),
        ("nan box min x", cir(0.0, 0.0, 1.0), bx(nan, -1.0, 1.0, 1.0)),
        ("nan box max x", cir(0.0, 0.0, 1.0), bx(-1.0, -1.0, nan, 1.0)),
        ("nan box min y", cir(0.0, 0.0, 1.0), bx(-1.0, nan, 1.0, 1.0)),
        ("nan box max y", cir(0.0, 0.0, 1.0), bx(-1.0, -1.0, 1.0, nan)),
        ("snan box max y", cir(0.0, 0.0, 1.0), bx(-1.0, -1.0, 1.0, f32::from_bits(0x7F80_0001))),
        ("neg zero box", cir(-0.0, -0.0, 1.0), bx(-0.0, -0.0, 0.0, 0.0)),
        (
            "subnormal box",
            cir(0.0, 0.0, f32::from_bits(1)),
            bx(0.0, 0.0, f32::from_bits(1), f32::from_bits(1)),
        ),
        ("huge box", cir(0.0, 0.0, 1.0), bx(-f32::MAX, -f32::MAX, f32::MAX, f32::MAX)),
        ("overflow dist", cir(f32::MAX, f32::MAX, f32::MAX), bx(-f32::MAX, -f32::MAX, 0.0, 0.0)),
    ];
    for (tag, A, B) in cases {
        let ctx = format!("{tag} A={} B={}", fmt_c(*A), fmt_b(*B));
        cmp_i("c2CircletoAABB", &ctx, (c.c2CircletoAABB)(*A, *B), (r.c2CircletoAABB)(*A, *B));
    }
}

// ---------------------------------------------------------------------------
// Row 20-23 : c2AABBtoAABB (direct)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row20_aabb_aabb_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (A, B) = (rng.any_aabb(), rng.any_aabb());
        let ctx = format!("#{i} A={} B={}", fmt_b(A), fmt_b(B));
        cmp_i("c2AABBtoAABB", &ctx, (c.c2AABBtoAABB)(A, B), (r.c2AABBtoAABB)(A, B));
    }
}

#[test]
fn cfg_row21_aabb_aabb_random_geometry() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let mut hits = 0usize;
    for i in 0..N {
        let (A, B) = (rng.ordered_aabb(100.0), rng.ordered_aabb(100.0));
        let ctx = format!("#{i} A={} B={}", fmt_b(A), fmt_b(B));
        let vv = (c.c2AABBtoAABB)(A, B);
        cmp_i("c2AABBtoAABB", &ctx, vv, (r.c2AABBtoAABB)(A, B));
        hits += (vv != 0) as usize;
    }
    assert!(hits > 0 && hits < N, "row 21 exercised only one branch ({hits}/{N})");
}

#[test]
fn cfg_row22_aabb_aabb_inverted() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        for (tag, A, B) in [
            ("A inverted", rng.inverted_aabb(100.0), rng.ordered_aabb(100.0)),
            ("B inverted", rng.ordered_aabb(100.0), rng.inverted_aabb(100.0)),
            ("both inverted", rng.inverted_aabb(100.0), rng.inverted_aabb(100.0)),
        ] {
            let ctx = format!("#{i} {tag} A={} B={}", fmt_b(A), fmt_b(B));
            cmp_i("c2AABBtoAABB", &ctx, (c.c2AABBtoAABB)(A, B), (r.c2AABBtoAABB)(A, B));
        }
    }
}

#[test]
fn cfg_row23_aabb_aabb_boundaries() {
    let (c, r) = both();
    let inf = f32::INFINITY;
    let nan = f32::NAN;
    let v = |x: f32, y: f32| C2v { x, y };
    let bx = |a: f32, b: f32, cc: f32, d: f32| C2Aabb { min: v(a, b), max: v(cc, d) };
    let unit = bx(0.0, 0.0, 1.0, 1.0);
    let cases: &[(&str, C2Aabb, C2Aabb)] = &[
        ("identical", unit, unit),
        ("contained", unit, bx(0.25, 0.25, 0.75, 0.75)),
        ("overlap partial", unit, bx(0.5, 0.5, 1.5, 1.5)),
        // Edge-touching: `B.max.x < A.min.x` is false at equality, so touching
        // boxes COUNT AS COLLIDING under this predicate.
        ("edge touch +x", unit, bx(1.0, 0.0, 2.0, 1.0)),
        ("edge touch -x", unit, bx(-1.0, 0.0, 0.0, 1.0)),
        ("edge touch +y", unit, bx(0.0, 1.0, 1.0, 2.0)),
        ("edge touch -y", unit, bx(0.0, -1.0, 1.0, 0.0)),
        ("corner touch", unit, bx(1.0, 1.0, 2.0, 2.0)),
        // Separated on exactly one axis at a time: d0, d1, d2, d3 individually.
        ("sep d0 (B.max.x<A.min.x)", unit, bx(-2.0, 0.0, -1.0, 1.0)),
        ("sep d1 (A.max.x<B.min.x)", unit, bx(2.0, 0.0, 3.0, 1.0)),
        ("sep d2 (B.max.y<A.min.y)", unit, bx(0.0, -2.0, 1.0, -1.0)),
        ("sep d3 (A.max.y<B.min.y)", unit, bx(0.0, 2.0, 1.0, 3.0)),
        ("1ulp separated", unit, bx(f32::from_bits(0x3F80_0001), 0.0, 2.0, 1.0)),
        ("zero area both", bx(0.0, 0.0, 0.0, 0.0), bx(0.0, 0.0, 0.0, 0.0)),
        ("zero area apart", bx(0.0, 0.0, 0.0, 0.0), bx(1.0, 1.0, 1.0, 1.0)),
        ("A inverted", bx(1.0, 1.0, 0.0, 0.0), unit),
        ("both inverted", bx(1.0, 1.0, 0.0, 0.0), bx(1.0, 1.0, 0.0, 0.0)),
        ("infinite", bx(-inf, -inf, inf, inf), unit),
        ("empty inf", bx(inf, inf, -inf, -inf), unit),
        ("empty inf both", bx(inf, inf, -inf, -inf), bx(inf, inf, -inf, -inf)),
        // NaN makes every `<` false, so the C reports a COLLISION.
        ("nan A.min.x", bx(nan, 0.0, 1.0, 1.0), unit),
        ("nan A.max.x", bx(0.0, 0.0, nan, 1.0), unit),
        ("nan A.min.y", bx(0.0, nan, 1.0, 1.0), unit),
        ("nan A.max.y", bx(0.0, 0.0, 1.0, nan), unit),
        ("nan B.min.x", unit, bx(nan, 0.0, 1.0, 1.0)),
        ("nan B.max.x", unit, bx(0.0, 0.0, nan, 1.0)),
        ("nan B.min.y", unit, bx(0.0, nan, 1.0, 1.0)),
        ("nan B.max.y", unit, bx(0.0, 0.0, 1.0, nan)),
        ("nan everywhere", bx(nan, nan, nan, nan), bx(nan, nan, nan, nan)),
        ("snan", bx(f32::from_bits(0x7F80_0001), 0.0, 1.0, 1.0), unit),
        ("neg zero", bx(-0.0, -0.0, 0.0, 0.0), bx(0.0, 0.0, -0.0, -0.0)),
        ("subnormal gap", bx(0.0, 0.0, 0.0, 0.0), bx(f32::from_bits(1), 0.0, 1.0, 1.0)),
        ("max range", bx(-f32::MAX, -f32::MAX, f32::MAX, f32::MAX), unit),
    ];
    for (tag, A, B) in cases {
        let ctx = format!("{tag} A={} B={}", fmt_b(*A), fmt_b(*B));
        cmp_i("c2AABBtoAABB", &ctx, (c.c2AABBtoAABB)(*A, *B), (r.c2AABBtoAABB)(*A, *B));
        let ctx = format!("{tag} swapped A={} B={}", fmt_b(*B), fmt_b(*A));
        cmp_i("c2AABBtoAABB", &ctx, (c.c2AABBtoAABB)(*B, *A), (r.c2AABBtoAABB)(*B, *A));
    }
}

// ---------------------------------------------------------------------------
// Row 24 : the composed pipeline, driven through the LOW-LEVEL exports
// ---------------------------------------------------------------------------

/// Reproduces `c2CircletoCircle`'s and `c2CircletoAABB`'s internals by chaining
/// the low-level exports of ONE library, then compares the intermediates across
/// libraries. A per-wrapper test cannot see a divergence that only appears in an
/// intermediate (e.g. a wrong `c2Clampv` result that the final `<` happens to
/// mask), so the intermediates are compared explicitly.
#[test]
fn cfg_row24_composed_pipeline() {
    let (c, r) = both();
    let mut rng = Rng::seeded();

    for i in 0..N {
        // --- circle/circle pipeline: c2Sub -> c2Dot
        let (A, B) = (rng.any_circle(), rng.any_circle());
        let cc = (c.c2Sub)(B.p, A.p);
        let rc = (r.c2Sub)(B.p, A.p);
        let ctx = format!("#{i} c2Sub(B.p,A.p) A={} B={}", fmt_c(A), fmt_c(B));
        cmp_v("pipeline/c2Sub", &ctx, cc, rc);
        let cd2 = (c.c2Dot)(cc, cc);
        let rd2 = (r.c2Dot)(rc, rc);
        cmp_f("pipeline/c2Dot", &ctx, cd2, rd2);
        // ...and the wrapper that composes them.
        cmp_i("pipeline/c2CircletoCircle", &ctx, (c.c2CircletoCircle)(A, B), (r.c2CircletoCircle)(A, B));

        // --- circle/AABB pipeline: c2Clampv -> c2Sub -> c2Dot
        let box_ = rng.any_aabb();
        let cl = (c.c2Clampv)(A.p, box_.min, box_.max);
        let rl = (r.c2Clampv)(A.p, box_.min, box_.max);
        let ctx = format!("#{i} c2Clampv A={} B={}", fmt_c(A), fmt_b(box_));
        cmp_v("pipeline/c2Clampv", &ctx, cl, rl);
        let cab = (c.c2Sub)(A.p, cl);
        let rab = (r.c2Sub)(A.p, rl);
        cmp_v("pipeline/c2Sub(A.p,L)", &ctx, cab, rab);
        cmp_f("pipeline/c2Dot(ab,ab)", &ctx, (c.c2Dot)(cab, cab), (r.c2Dot)(rab, rab));
        cmp_i("pipeline/c2CircletoAABB", &ctx, (c.c2CircletoAABB)(A, box_), (r.c2CircletoAABB)(A, box_));
    }

    // Internal consistency on well-behaved finite geometry: the wrapper's answer
    // must equal `d2 < r*r` recomputed from that SAME library's low-level
    // intermediates. Restricted to finite, moderate magnitudes so the reference
    // `<` in the test is itself unambiguous.
    for i in 0..N {
        let A = rng.finite_circle(50.0, 5.0);
        let B = rng.finite_circle(50.0, 5.0);
        for api in [c, r] {
            let d = (api.c2Sub)(B.p, A.p);
            let d2 = (api.c2Dot)(d, d);
            let s = A.r + B.r;
            let expect = (d2 < s * s) as i32;
            let got = (api.c2CircletoCircle)(A, B);
            assert_eq!(
                got, expect,
                "{} c2CircletoCircle is inconsistent with its own c2Sub/c2Dot at #{i}: \
                 A={} B={} d2={:e} r2={:e}",
                api.name,
                fmt_c(A),
                fmt_c(B),
                d2,
                s * s
            );
        }

        let bb = rng.ordered_aabb(50.0);
        for api in [c, r] {
            let l = (api.c2Clampv)(A.p, bb.min, bb.max);
            let ab = (api.c2Sub)(A.p, l);
            let d2 = (api.c2Dot)(ab, ab);
            let expect = (d2 < A.r * A.r) as i32;
            let got = (api.c2CircletoAABB)(A, bb);
            assert_eq!(
                got, expect,
                "{} c2CircletoAABB is inconsistent with its own c2Clampv/c2Sub/c2Dot at #{i}: \
                 A={} B={}",
                api.name,
                fmt_c(A),
                fmt_b(bb)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25-28 : collided, the 4 valid tag combinations
// ---------------------------------------------------------------------------

#[test]
fn cfg_row25_collided_circle_circle() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (A, B) = (rng.any_circle(), rng.any_circle());
        let (ba, bb) = (Buf::circle(A), Buf::circle(B));
        let ctx = format!("#{i} bits A={} B={}", fmt_c(A), fmt_c(B));
        let v = cmp_collided(&ctx, &ba, C2_TYPE_CIRCLE, &bb, C2_TYPE_CIRCLE);
        // The dispatch must land on c2CircletoCircle(A, B) in that order.
        cmp_i("collided==c2CircletoCircle(A,B)", &ctx, (c.c2CircletoCircle)(A, B), v);
        cmp_i("rust dispatch", &ctx, (r.c2CircletoCircle)(A, B), v);
    }
    let mut hits = 0usize;
    for i in 0..N {
        let A = rng.finite_circle(100.0, 10.0);
        let B = rng.finite_circle(100.0, 10.0);
        let (ba, bb) = (Buf::circle(A), Buf::circle(B));
        let ctx = format!("#{i} geom A={} B={}", fmt_c(A), fmt_c(B));
        hits += (cmp_collided(&ctx, &ba, C2_TYPE_CIRCLE, &bb, C2_TYPE_CIRCLE) != 0) as usize;
    }
    assert!(hits > 0 && hits < N, "row 25 exercised only one branch ({hits}/{N})");
}

#[test]
fn cfg_row26_collided_circle_aabb() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let A = rng.any_circle();
        let B = rng.any_aabb();
        let (ba, bb) = (Buf::circle(A), Buf::aabb(B));
        let ctx = format!("#{i} bits A={} B={}", fmt_c(A), fmt_b(B));
        let v = cmp_collided(&ctx, &ba, C2_TYPE_CIRCLE, &bb, C2_TYPE_AABB);
        // C: `c2CircletoAABB(*(c2Circle*)A, *(c2AABB*)B)` — A is the circle.
        cmp_i("collided==c2CircletoAABB(A,B)", &ctx, (c.c2CircletoAABB)(A, B), v);
        cmp_i("rust dispatch", &ctx, (r.c2CircletoAABB)(A, B), v);
    }
    let mut hits = 0usize;
    for i in 0..N {
        let A = rng.finite_circle(100.0, 20.0);
        for (tag, B) in [("ordered", rng.ordered_aabb(100.0)), ("inverted", rng.inverted_aabb(100.0))] {
            let (ba, bb) = (Buf::circle(A), Buf::aabb(B));
            let ctx = format!("#{i} {tag} A={} B={}", fmt_c(A), fmt_b(B));
            let v = cmp_collided(&ctx, &ba, C2_TYPE_CIRCLE, &bb, C2_TYPE_AABB);
            if tag == "ordered" {
                hits += (v != 0) as usize;
            }
        }
    }
    assert!(hits > 0 && hits < N, "row 26 exercised only one branch ({hits}/{N})");
}

#[test]
fn cfg_row27_collided_aabb_circle() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        // typeA=AABB so `A` is the box and `B` is the circle; the C then calls
        // `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` — the OPERANDS ARE SWAPPED.
        let boxA = rng.any_aabb();
        let cirB = rng.any_circle();
        let (ba, bb) = (Buf::aabb(boxA), Buf::circle(cirB));
        let ctx = format!("#{i} bits A(box)={} B(circle)={}", fmt_b(boxA), fmt_c(cirB));
        let v = cmp_collided(&ctx, &ba, C2_TYPE_AABB, &bb, C2_TYPE_CIRCLE);
        // Confirm the swap is REPRODUCED, not silently "corrected".
        cmp_i("collided==c2CircletoAABB(B,A)", &ctx, (c.c2CircletoAABB)(cirB, boxA), v);
        cmp_i("rust dispatch", &ctx, (r.c2CircletoAABB)(cirB, boxA), v);
    }
    let mut hits = 0usize;
    for i in 0..N {
        let cirB = rng.finite_circle(100.0, 20.0);
        for (tag, boxA) in [("ordered", rng.ordered_aabb(100.0)), ("inverted", rng.inverted_aabb(100.0))] {
            let (ba, bb) = (Buf::aabb(boxA), Buf::circle(cirB));
            let ctx = format!("#{i} {tag} A(box)={} B(circle)={}", fmt_b(boxA), fmt_c(cirB));
            let v = cmp_collided(&ctx, &ba, C2_TYPE_AABB, &bb, C2_TYPE_CIRCLE);
            cmp_i("swap check", &ctx, (c.c2CircletoAABB)(cirB, boxA), v);
            if tag == "ordered" {
                hits += (v != 0) as usize;
            }
        }
    }
    assert!(hits > 0 && hits < N, "row 27 exercised only one branch ({hits}/{N})");
}

#[test]
fn cfg_row28_collided_aabb_aabb() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..N {
        let (A, B) = (rng.any_aabb(), rng.any_aabb());
        let (ba, bb) = (Buf::aabb(A), Buf::aabb(B));
        let ctx = format!("#{i} bits A={} B={}", fmt_b(A), fmt_b(B));
        let v = cmp_collided(&ctx, &ba, C2_TYPE_AABB, &bb, C2_TYPE_AABB);
        cmp_i("collided==c2AABBtoAABB(A,B)", &ctx, (c.c2AABBtoAABB)(A, B), v);
        cmp_i("rust dispatch", &ctx, (r.c2AABBtoAABB)(A, B), v);
    }
    let mut hits = 0usize;
    for i in 0..N {
        for (tag, A, B) in [
            ("ordered", rng.ordered_aabb(100.0), rng.ordered_aabb(100.0)),
            ("inverted", rng.inverted_aabb(100.0), rng.ordered_aabb(100.0)),
        ] {
            let (ba, bb) = (Buf::aabb(A), Buf::aabb(B));
            let ctx = format!("#{i} {tag} A={} B={}", fmt_b(A), fmt_b(B));
            let v = cmp_collided(&ctx, &ba, C2_TYPE_AABB, &bb, C2_TYPE_AABB);
            if tag == "ordered" {
                hits += (v != 0) as usize;
            }
        }
    }
    assert!(hits > 0 && hits < N, "row 28 exercised only one branch ({hits}/{N})");
}

// ---------------------------------------------------------------------------
// Row 29-30 : collided with aliased pointers and with raw byte blobs
// ---------------------------------------------------------------------------

#[test]
fn cfg_row29_collided_aliased_pointers() {
    let (_c, _r) = both();
    let mut rng = Rng::seeded();
    let tags = [C2_TYPE_CIRCLE, C2_TYPE_AABB];
    for i in 0..N {
        // One 16-byte buffer passed as BOTH operands. Legal for the C: whichever
        // tag is used, the read stays inside the 16 bytes.
        let raw = Buf([rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()]);
        for ta in tags {
            for tb in tags {
                let ctx = format!(
                    "#{i} aliased ta={ta} tb={tb} raw=[{:#010x},{:#010x},{:#010x},{:#010x}]",
                    raw.0[0], raw.0[1], raw.0[2], raw.0[3]
                );
                cmp_collided(&ctx, &raw, ta, &raw, tb);
            }
        }
        // Also finite geometry aliased, so the predicates land on real answers
        // rather than almost always on NaN paths.
        let g = Buf::aabb(rng.ordered_aabb(10.0));
        for ta in tags {
            for tb in tags {
                let ctx = format!("#{i} aliased-geom ta={ta} tb={tb} {}", fmt_b(g.as_aabb()));
                cmp_collided(&ctx, &g, ta, &g, tb);
            }
        }
    }
}

#[test]
fn cfg_row30_collided_raw_blob() {
    let (_c, _r) = both();
    let mut rng = Rng::seeded();
    let tags = [C2_TYPE_CIRCLE, C2_TYPE_AABB];
    for i in 0..N {
        // Two independent 16-byte blobs of arbitrary bytes: the struct contents
        // are not "constructed values" at all, which is how a real caller passing
        // deserialised data would look.
        let a = Buf([rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()]);
        let b = Buf([rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()]);
        for ta in tags {
            for tb in tags {
                let ctx = format!(
                    "#{i} blob ta={ta} tb={tb} A={} / {} B={} / {}",
                    fmt_c(a.as_circle()),
                    fmt_b(a.as_aabb()),
                    fmt_c(b.as_circle()),
                    fmt_b(b.as_aabb())
                );
                cmp_collided(&ctx, &a, ta, &b, tb);
            }
        }
    }
}
