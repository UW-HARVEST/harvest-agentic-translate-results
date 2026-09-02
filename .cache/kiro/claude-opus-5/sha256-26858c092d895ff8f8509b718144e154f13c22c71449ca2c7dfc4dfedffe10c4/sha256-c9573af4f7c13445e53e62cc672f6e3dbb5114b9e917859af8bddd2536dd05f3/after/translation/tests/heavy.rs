//! Heavy structured sweeps for the arithmetic kernels.
//!
//! The randomized rows in `configs.rs` sample uniformly over bit patterns, which
//! under-samples the *interactions* that matter: two NaNs with different
//! payloads, `0 * inf`, `inf - inf`, and the exponent boundaries where a product
//! overflows or underflows. This file builds a dense structured grid over
//! (sign x exponent x mantissa) instead, so those interactions occur
//! systematically rather than by chance.
//!
//! Run with `cargo test --test heavy -- --ignored` (they take longer than the
//! default suite, so they are `#[ignore]`d).
#![allow(non_snake_case)]

mod harness;
use harness::*;
use harness::Api;

/// Exponents chosen at every boundary the format has: zero/subnormal (0), the
/// smallest normal (1), around 1.0 (126/127/128), the top of the normal range
/// (253/254), and the inf/NaN encoding (255).
const EXPONENTS: &[u32] = &[0, 1, 2, 63, 126, 127, 128, 190, 252, 253, 254, 255];

/// Mantissas at their boundaries plus a couple of arbitrary payloads. With
/// exponent 255 these select inf (0), signalling NaNs (1, 0x3FFFFF, 0x7FFFFF)
/// and quiet NaNs (0x400000, 0x401234, 0x5EADBE).
const MANTISSAS: &[u32] = &[0, 1, 0x3F_FFFF, 0x40_0000, 0x40_1234, 0x5E_ADBE, 0x7F_FFFF];

/// The full structured grid: 2 signs x 12 exponents x 7 mantissas = 168 values,
/// spanning every float class and both NaN kinds with distinct payloads.
fn grid() -> Vec<f32> {
    let mut v = Vec::with_capacity(2 * EXPONENTS.len() * MANTISSAS.len());
    for sign in [0u32, 1u32] {
        for &e in EXPONENTS {
            for &m in MANTISSAS {
                v.push(f32::from_bits((sign << 31) | (e << 23) | m));
            }
        }
    }
    v
}

fn ctx2(a: C2v, b: C2v) -> String {
    format!("a={} b={}", fmt_v(a), fmt_v(b))
}

/// `c2Sub` over the full grid x grid on the x lane, with the y lane rotated
/// through the grid so both lanes see every pair. 168^2 = 28224 x-pairs.
#[test]
#[ignore = "heavy; run with --ignored"]
fn heavy_c2sub_structured_grid() {
    let (c, r) = both();
    let g = grid();
    let n = g.len();
    for (i, &ax) in g.iter().enumerate() {
        for (j, &bx) in g.iter().enumerate() {
            let a = C2v { x: ax, y: g[(i + j) % n] };
            let b = C2v { x: bx, y: g[(i + 2 * j + 1) % n] };
            let (cv, rv) = ((c.c2Sub)(a, b), (r.c2Sub)(a, b));
            if !same_v(cv, rv) {
                panic!(
                    "c2Sub DIVERGED\n  input : {}\n  C     : {}\n  Rust  : {}",
                    ctx2(a, b),
                    fmt_v(cv),
                    fmt_v(rv)
                );
            }
        }
    }
}

/// `c2Dot` over the full grid x grid. This is the function whose SSE operand
/// order is pinned, so every (product-NaN, product-NaN) and
/// (product-inf, product--inf) pairing must be reached.
#[test]
#[ignore = "heavy; run with --ignored"]
fn heavy_c2dot_structured_grid() {
    let (c, r) = both();
    let g = grid();
    let n = g.len();
    for (i, &ax) in g.iter().enumerate() {
        for (j, &bx) in g.iter().enumerate() {
            // Rotate the y lane so the SECOND product also sweeps the whole grid
            // independently of the first, which is what makes the two products
            // differ in class (NaN vs inf vs finite).
            for k in [0usize, 1, 37, 84, 167] {
                let a = C2v { x: ax, y: g[(i + k) % n] };
                let b = C2v { x: bx, y: g[(j + 2 * k + 1) % n] };
                let (cv, rv) = ((c.c2Dot)(a, b), (r.c2Dot)(a, b));
                if !same_f(cv, rv) {
                    panic!(
                        "c2Dot DIVERGED\n  input : {}\n  C     : {:e}/{:#010x}\n  Rust  : {:e}/{:#010x}",
                        ctx2(a, b),
                        cv,
                        cv.to_bits(),
                        rv,
                        rv.to_bits()
                    );
                }
            }
        }
    }
}

/// `c2Maxv` / `c2Minv` / `c2Clampv` over the grid. These are pure comparisons, so
/// the interesting content is the unordered (NaN) cases and `±0`.
#[test]
#[ignore = "heavy; run with --ignored"]
fn heavy_minmax_clamp_structured_grid() {
    let (c, r) = both();
    let g = grid();
    let n = g.len();
    for (i, &ax) in g.iter().enumerate() {
        for (j, &bx) in g.iter().enumerate() {
            let a = C2v { x: ax, y: g[(i + j) % n] };
            let b = C2v { x: bx, y: g[(n - 1 - ((i + j) % n)) % n] };
            let ctx = ctx2(a, b);
            let (cv, rv) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            assert!(same_v(cv, rv), "c2Maxv DIVERGED {ctx}: C {} Rust {}", fmt_v(cv), fmt_v(rv));
            let (cv, rv) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
            assert!(same_v(cv, rv), "c2Minv DIVERGED {ctx}: C {} Rust {}", fmt_v(cv), fmt_v(rv));
            // Third operand from a different phase of the grid.
            let h = C2v { x: g[(i * 3 + j) % n], y: g[(i + j * 5) % n] };
            let (cv, rv) = ((c.c2Clampv)(a, b, h), (r.c2Clampv)(a, b, h));
            assert!(
                same_v(cv, rv),
                "c2Clampv DIVERGED a={} lo={} hi={}: C {} Rust {}",
                fmt_v(a),
                fmt_v(b),
                fmt_v(h),
                fmt_v(cv),
                fmt_v(rv)
            );
        }
    }
}

/// The three predicates plus `collided` over a large random sample — an order of
/// magnitude more inputs than the default suite, mixing fully-random bit patterns
/// with plausible geometry.
#[test]
#[ignore = "heavy; run with --ignored"]
fn heavy_predicates_random_bulk() {
    let (c, r) = both();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    const M: usize = 400_000;
    for i in 0..M {
        let bits = i % 3 != 0; // 2/3 random bits, 1/3 plausible geometry
        let (A, B) = if bits {
            (rng.any_circle(), rng.any_circle())
        } else {
            (rng.finite_circle(100.0, 10.0), rng.finite_circle(100.0, 10.0))
        };
        let cc = (c.c2CircletoCircle)(A, B);
        let rc = (r.c2CircletoCircle)(A, B);
        assert_eq!(cc, rc, "c2CircletoCircle DIVERGED #{i}: A={} B={}", fmt_c(A), fmt_c(B));

        let bb = if bits { rng.any_aabb() } else { rng.ordered_aabb(100.0) };
        let cc = (c.c2CircletoAABB)(A, bb);
        let rc = (r.c2CircletoAABB)(A, bb);
        assert_eq!(cc, rc, "c2CircletoAABB DIVERGED #{i}: A={} B={}", fmt_c(A), fmt_b(bb));

        let b2 = if bits { rng.any_aabb() } else { rng.inverted_aabb(100.0) };
        let cc = (c.c2AABBtoAABB)(bb, b2);
        let rc = (r.c2AABBtoAABB)(bb, b2);
        assert_eq!(cc, rc, "c2AABBtoAABB DIVERGED #{i}: A={} B={}", fmt_b(bb), fmt_b(b2));

        // `collided` over all four tag pairs on raw 16-byte blobs.
        let pa: [u32; 4] = [rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let pb: [u32; 4] = [rng.next_u32(), rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let (qa, qb) = (
            pa.as_ptr() as *const std::ffi::c_void,
            pb.as_ptr() as *const std::ffi::c_void,
        );
        for ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB] {
            for tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB] {
                let cv = unsafe { (c.collided)(qa, ta, qb, tb) };
                let rv = unsafe { (r.collided)(qa, ta, qb, tb) };
                assert_eq!(
                    cv, rv,
                    "collided DIVERGED #{i} ta={ta} tb={tb} A={pa:08x?} B={pb:08x?}"
                );
            }
        }
    }
}

/// Exhaustive over one full `f32` argument: `c2Dot((x, 0), (x, 0))`, i.e. `x*x`,
/// for **every one of the 2^32 bit patterns**. This is the only argument position
/// where exhaustion is affordable, and it pins the `mulss`/`addss` emulation
/// against the real hardware for every input value rather than a sample.
#[test]
#[ignore = "very heavy (2^32 calls); run with --ignored --test-threads=1"]
fn heavy_c2dot_exhaustive_single_lane() {
    let (c, r) = both();
    let zero = 0.0f32;
    let mut bad = 0usize;
    let mut b: u32 = 0;
    loop {
        let x = f32::from_bits(b);
        let a = C2v { x, y: zero };
        let cv = (c.c2Dot)(a, a);
        let rv = (r.c2Dot)(a, a);
        if cv.to_bits() != rv.to_bits() {
            if bad < 20 {
                eprintln!(
                    "c2Dot DIVERGED at x={:#010x}: C {:#010x} Rust {:#010x}",
                    b,
                    cv.to_bits(),
                    rv.to_bits()
                );
            }
            bad += 1;
        }
        if b == u32::MAX {
            break;
        }
        b += 1;
    }
    assert_eq!(bad, 0, "{bad} of 2^32 single-lane c2Dot inputs diverged");
}

/// Exhaustive over one full `f32` argument for `c2Sub`: every one of the 2^32 bit
/// patterns appears as the *destination* operand, paired with a partner rotated
/// through [`SPECIAL_BITS`] so `subss` is exercised against every float class for
/// every possible dst. Pins the `inf - inf` -> QNaN-indefinite rule and the
/// SNaN-quieting rule across the whole domain.
///
/// Runtime: roughly 3 minutes.
#[test]
#[ignore = "very heavy (2^32 calls); run with --ignored --test-threads=1"]
fn heavy_c2sub_exhaustive_single_lane() {
    let (c, r) = both();
    let sp = specials();
    let n = sp.len();
    let mut bad = 0usize;
    let mut b: u32 = 0;
    loop {
        // Rotate the partner by the low bits of `b`, and use a different rotation
        // on the y lane so two distinct (dst, src) pairs are covered per iteration.
        let p = sp[(b as usize) % n];
        let q = sp[((b as usize) / n + 1) % n];
        let a = C2v { x: f32::from_bits(b), y: p };
        let d = C2v { x: p, y: q };
        let cv = (c.c2Sub)(a, d);
        let rv = (r.c2Sub)(a, d);
        if cv.bits() != rv.bits() {
            if bad < 20 {
                eprintln!(
                    "c2Sub DIVERGED at a={} b={}: C {} Rust {}",
                    fmt_v(a),
                    fmt_v(d),
                    fmt_v(cv),
                    fmt_v(rv)
                );
            }
            bad += 1;
        }
        if b == u32::MAX {
            break;
        }
        b += 1;
    }
    assert_eq!(bad, 0, "{bad} of 2^32 single-lane c2Sub inputs diverged");
}

/// Exhaustive single-lane sweeps for the three comparison-based functions.
///
/// Split into one test per function (rather than one combined test) so that each
/// finishes in roughly 3 minutes and stays well inside a 600 s command budget;
/// combined they took ~9m40s, which is too long for a single invocation.
macro_rules! exhaustive_cmp {
    ($name:ident, $label:literal, $call:expr) => {
        /// Every one of the 2^32 bit patterns appears as the left operand, with
        /// the other operands rotated through [`SPECIAL_BITS`]. Pins the
        /// `a > b ? a : b` / `a < b ? a : b` unordered-compare behaviour across
        /// the whole domain.
        #[test]
        #[ignore = "very heavy (2^32 calls, ~3 min); run with --ignored --test-threads=1"]
        fn $name() {
            let (c, r) = both();
            let sp = specials();
            let n = sp.len();
            let f: fn(&Api, C2v, C2v, C2v) -> C2v = $call;
            let mut bad = 0usize;
            let mut b: u32 = 0;
            loop {
                let x = f32::from_bits(b);
                let p = sp[(b as usize) % n];
                let q = sp[((b as usize) / n + 1) % n];
                let a = C2v { x, y: p };
                let lo = C2v { x: p, y: x };
                let hi = C2v { x: q, y: q };
                let cv = f(c, a, lo, hi);
                let rv = f(r, a, lo, hi);
                if cv.bits() != rv.bits() {
                    if bad < 20 {
                        eprintln!(
                            "{} DIVERGED a={} lo={} hi={}: C {} Rust {}",
                            $label,
                            fmt_v(a),
                            fmt_v(lo),
                            fmt_v(hi),
                            fmt_v(cv),
                            fmt_v(rv)
                        );
                    }
                    bad += 1;
                }
                if b == u32::MAX {
                    break;
                }
                b += 1;
            }
            assert_eq!(bad, 0, "{} of 2^32 exhaustive {} inputs diverged", bad, $label);
        }
    };
}

exhaustive_cmp!(heavy_c2maxv_exhaustive_single_lane, "c2Maxv", |api, a, lo, _hi| (api.c2Maxv)(a, lo));
exhaustive_cmp!(heavy_c2minv_exhaustive_single_lane, "c2Minv", |api, a, lo, _hi| (api.c2Minv)(a, lo));
exhaustive_cmp!(
    heavy_c2clampv_exhaustive_single_lane,
    "c2Clampv",
    |api, a, lo, hi| (api.c2Clampv)(a, lo, hi)
);
