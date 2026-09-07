//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through `dlopen`/`dlsym` on their
//! respective `.so`s (see `harness`), so the exported ABI is under test too.
//! Every row uses many randomized inputs from a fixed seed.

#![allow(non_snake_case)]

mod harness;
use harness::*;
use std::ffi::c_void;

const N: usize = 4000;

// ===========================================================================
// Rows 1-2 — c2V
// ===========================================================================

#[test]
fn row01_c2v_random_finite() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x01);
    for _ in 0..N {
        let (x, y) = (rng.finite_f32(), rng.finite_f32());
        unsafe {
            assert_v_eq("c2V", &format!("({x:e},{y:e})"), (c.c2V)(x, y), (r.c2V)(x, y));
        }
    }
}

#[test]
fn row02_c2v_specials() {
    let (c, r) = pair();
    for &x in SPECIAL {
        for &y in SPECIAL {
            unsafe {
                assert_v_eq(
                    "c2V",
                    &format!("(0x{:08x},0x{:08x})", x.to_bits(), y.to_bits()),
                    (c.c2V)(x, y),
                    (r.c2V)(x, y),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 3-10 — c2Maxv / c2Minv
// ===========================================================================

fn vv_sweep(name: &str, cf: unsafe extern "C" fn(C2v, C2v) -> C2v, rf: unsafe extern "C" fn(C2v, C2v) -> C2v, mut g: impl FnMut() -> (C2v, C2v), n: usize) {
    for _ in 0..n {
        let (a, b) = g();
        let args = format!(
            "a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits()
        );
        unsafe { assert_v_eq(name, &args, cf(a, b), rf(a, b)) };
    }
}

/// All ordered pairs drawn from `pool`, per component, exhaustively.
fn vv_exhaustive(name: &str, cf: unsafe extern "C" fn(C2v, C2v) -> C2v, rf: unsafe extern "C" fn(C2v, C2v) -> C2v, pool: &[f32]) {
    for &ax in pool {
        for &bx in pool {
            for &ay in pool {
                for &by in pool {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    let args = format!(
                        "a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
                        ax.to_bits(),
                        ay.to_bits(),
                        bx.to_bits(),
                        by.to_bits()
                    );
                    unsafe { assert_v_eq(name, &args, cf(a, b), rf(a, b)) };
                }
            }
        }
    }
}

#[test]
fn row03_maxv_finite_orderings() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x03);
    // random normals
    vv_sweep("c2Maxv", c.c2Maxv, r.c2Maxv, || (rng.v_small(), rng.v_small()), N);
    let mut rng = Rng::new(0x13);
    // deliberately force a==b, a<b, a>b per component
    for _ in 0..N {
        let x = rng.small_f32();
        let y = rng.small_f32();
        for &(dx, dy) in &[(0.0, 0.0), (1.0, -1.0), (-1.0, 1.0)] {
            let (a, b) = (v(x, y), v(x + dx, y + dy));
            unsafe { assert_v_eq("c2Maxv", "forced ordering", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b)) };
        }
    }
}

#[test]
fn row04_maxv_zeros_and_subnormals() {
    let (c, r) = pair();
    let pool = [
        0.0f32,
        -0.0,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::MIN_POSITIVE,
    ];
    vv_exhaustive("c2Maxv", c.c2Maxv, r.c2Maxv, &pool);
}

#[test]
fn row05_maxv_nan_payloads() {
    let (c, r) = pair();
    let pool = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFDE_AD00),
        f32::from_bits(0x7F80_0001),
        1.0,
        -1.0,
    ];
    vv_exhaustive("c2Maxv", c.c2Maxv, r.c2Maxv, &pool);
}

#[test]
fn row06_maxv_infinities() {
    let (c, r) = pair();
    let pool = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        0.0,
        f32::from_bits(0x7FC0_1234),
    ];
    vv_exhaustive("c2Maxv", c.c2Maxv, r.c2Maxv, &pool);
}

#[test]
fn row07_minv_finite_orderings() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x07);
    vv_sweep("c2Minv", c.c2Minv, r.c2Minv, || (rng.v_small(), rng.v_small()), N);
    let mut rng = Rng::new(0x17);
    for _ in 0..N {
        let x = rng.small_f32();
        let y = rng.small_f32();
        for &(dx, dy) in &[(0.0, 0.0), (1.0, -1.0), (-1.0, 1.0)] {
            let (a, b) = (v(x, y), v(x + dx, y + dy));
            unsafe { assert_v_eq("c2Minv", "forced ordering", (c.c2Minv)(a, b), (r.c2Minv)(a, b)) };
        }
    }
}

#[test]
fn row08_minv_zeros_and_subnormals() {
    let (c, r) = pair();
    let pool = [
        0.0f32,
        -0.0,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::MIN_POSITIVE,
    ];
    vv_exhaustive("c2Minv", c.c2Minv, r.c2Minv, &pool);
}

#[test]
fn row09_minv_nan_payloads() {
    let (c, r) = pair();
    let pool = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFDE_AD00),
        f32::from_bits(0x7F80_0001),
        1.0,
        -1.0,
    ];
    vv_exhaustive("c2Minv", c.c2Minv, r.c2Minv, &pool);
}

#[test]
fn row10_minv_infinities() {
    let (c, r) = pair();
    let pool = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        0.0,
        f32::from_bits(0x7FC0_1234),
    ];
    vv_exhaustive("c2Minv", c.c2Minv, r.c2Minv, &pool);
}

// ===========================================================================
// Rows 11-14 — c2Clampv
// ===========================================================================

fn clamp_check(c: &Impl, r: &Impl, a: C2v, lo: C2v, hi: C2v) {
    let args = format!(
        "a=(0x{:08x},0x{:08x}) lo=(0x{:08x},0x{:08x}) hi=(0x{:08x},0x{:08x})",
        a.x.to_bits(),
        a.y.to_bits(),
        lo.x.to_bits(),
        lo.y.to_bits(),
        hi.x.to_bits(),
        hi.y.to_bits()
    );
    unsafe { assert_v_eq("c2Clampv", &args, (c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi)) };
}

#[test]
fn row11_clampv_ordered_range() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x11);
    for _ in 0..N {
        let l = rng.small_f32();
        let h = l + (rng.next_u32() % 500) as f32;
        let l2 = rng.small_f32();
        let h2 = l2 + (rng.next_u32() % 500) as f32;
        let lo = v(l, l2);
        let hi = v(h, h2);
        // below, inside, above
        for a in [v(l - 10.0, l2 - 10.0), v((l + h) * 0.5, (l2 + h2) * 0.5), v(h + 10.0, h2 + 10.0), rng.v_small()] {
            clamp_check(c, r, a, lo, hi);
        }
    }
}

#[test]
fn row12_clampv_inverted_range() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x12);
    for _ in 0..N {
        let h = rng.small_f32();
        let l = h + (1 + rng.next_u32() % 500) as f32; // lo > hi
        let h2 = rng.small_f32();
        let l2 = h2 + (1 + rng.next_u32() % 500) as f32;
        clamp_check(c, r, rng.v_small(), v(l, l2), v(h, h2));
    }
}

#[test]
fn row13_clampv_nan_placements() {
    let (c, r) = pair();
    let nans = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFDE_AD00),
        f32::from_bits(0x7F80_0001),
    ];
    // every placement of a NaN among the 6 scalar slots, with random finites elsewhere
    let mut rng = Rng::new(0x13);
    for &nan in &nans {
        for slot in 0..6 {
            for _ in 0..300 {
                let mut s = [
                    rng.small_f32(),
                    rng.small_f32(),
                    rng.small_f32(),
                    rng.small_f32(),
                    rng.small_f32(),
                    rng.small_f32(),
                ];
                s[slot] = nan;
                clamp_check(c, r, v(s[0], s[1]), v(s[2], s[3]), v(s[4], s[5]));
            }
        }
    }
    // all-NaN and multi-NaN combinations
    for &n1 in &nans {
        for &n2 in &nans {
            for &n3 in &nans {
                clamp_check(c, r, v(n1, n2), v(n2, n3), v(n3, n1));
            }
        }
    }
    // fully random 32-bit patterns (hits arbitrary NaN payloads)
    let mut rng = Rng::new(0x1313);
    for _ in 0..N {
        clamp_check(c, r, rng.v_any(), rng.v_any(), rng.v_any());
    }
}

#[test]
fn row14_clampv_infinite_and_degenerate_bounds() {
    let (c, r) = pair();
    let pool = [f32::INFINITY, f32::NEG_INFINITY, 0.0f32, -0.0, 1.0, -1.0, f32::MAX, f32::MIN];
    for &a in &pool {
        for &lo in &pool {
            for &hi in &pool {
                clamp_check(c, r, v(a, hi), v(lo, a), v(hi, lo));
                clamp_check(c, r, v(a, a), v(lo, lo), v(lo, lo)); // lo == hi
            }
        }
    }
}

// ===========================================================================
// Rows 15-18 — c2Sub
// ===========================================================================

fn sub_check(c: &Impl, r: &Impl, a: C2v, b: C2v) {
    let args = format!(
        "a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
        a.x.to_bits(),
        a.y.to_bits(),
        b.x.to_bits(),
        b.y.to_bits()
    );
    unsafe { assert_v_eq("c2Sub", &args, (c.c2Sub)(a, b), (r.c2Sub)(a, b)) };
}

#[test]
fn row15_sub_random_finite() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x15);
    for _ in 0..N {
        sub_check(c, r, rng.v_small(), rng.v_small());
    }
    let mut rng = Rng::new(0x1515);
    for _ in 0..N {
        sub_check(c, r, v(rng.finite_f32(), rng.finite_f32()), v(rng.finite_f32(), rng.finite_f32()));
    }
}

#[test]
fn row16_sub_overflow_underflow_signed_zero() {
    let (c, r) = pair();
    let pool = [
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::EPSILON,
        1e30,
        -1e30,
    ];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    sub_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row17_sub_inf_minus_inf() {
    let (c, r) = pair();
    let pool = [f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 0.0, -0.0, 1.0];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    sub_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row18_sub_nan_payload_priority() {
    let (c, r) = pair();
    let pool = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFDE_AD00),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        1.0,
        f32::INFINITY,
    ];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    sub_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
    let mut rng = Rng::new(0x18);
    for _ in 0..N {
        sub_check(c, r, rng.v_any(), rng.v_any());
    }
}

// ===========================================================================
// Rows 19-22 — c2Dot
// ===========================================================================

fn dot_check(c: &Impl, r: &Impl, a: C2v, b: C2v) {
    let args = format!(
        "a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
        a.x.to_bits(),
        a.y.to_bits(),
        b.x.to_bits(),
        b.y.to_bits()
    );
    unsafe { assert_f_eq("c2Dot", &args, (c.c2Dot)(a, b), (r.c2Dot)(a, b)) };
}

#[test]
fn row19_dot_random_finite() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x19);
    for _ in 0..N {
        dot_check(c, r, rng.v_small(), rng.v_small());
    }
    let mut rng = Rng::new(0x1919);
    for _ in 0..N {
        dot_check(c, r, v(rng.finite_f32(), rng.finite_f32()), v(rng.finite_f32(), rng.finite_f32()));
    }
}

#[test]
fn row20_dot_overflow_and_inf_cancellation() {
    let (c, r) = pair();
    let pool = [
        f32::MAX,
        f32::MIN,
        1e30,
        -1e30,
        1e20,
        -1e20,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
        -1.0,
    ];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    dot_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row21_dot_zero_times_inf_and_underflow() {
    let (c, r) = pair();
    let pool = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE,
        1e-30,
        -1e-30,
        1.0,
    ];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    dot_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row22_dot_nan_priority_in_products_and_sum() {
    let (c, r) = pair();
    let pool = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFDE_AD00),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        0.0,
        f32::INFINITY,
        1.0,
    ];
    for &ax in &pool {
        for &bx in &pool {
            for &ay in &pool {
                for &by in &pool {
                    dot_check(c, r, v(ax, ay), v(bx, by));
                }
            }
        }
    }
    let mut rng = Rng::new(0x22);
    for _ in 0..(N * 4) {
        dot_check(c, r, rng.v_any(), rng.v_any());
    }
    let mut rng = Rng::new(0x2222);
    for _ in 0..(N * 4) {
        dot_check(c, r, rng.v_interesting(), rng.v_interesting());
    }
}

// ===========================================================================
// Rows 23-29 — c2CircletoCircle
// ===========================================================================

fn cc_check(c: &Impl, r: &Impl, A: C2Circle, B: C2Circle) {
    let args = format!("A={A:?} B={B:?}");
    unsafe {
        assert_i_eq(
            "c2CircletoCircle",
            &args,
            (c.c2CircletoCircle)(A, B),
            (r.c2CircletoCircle)(A, B),
        )
    };
}

#[test]
fn row23_cc_overlapping() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x23);
    let mut overlaps = 0;
    for _ in 0..N {
        let p = rng.v_small();
        let ra = 1.0 + (rng.next_u32() % 100) as f32;
        let rb = 1.0 + (rng.next_u32() % 100) as f32;
        // place B's centre inside the combined radius
        let frac = (rng.next_u32() % 1000) as f32 / 1000.0 * 0.99;
        let ang = (rng.next_u32() % 6283) as f32 / 1000.0;
        let d = (ra + rb) * frac;
        let B = circle(p.x + d * ang.cos(), p.y + d * ang.sin(), rb);
        let A = C2Circle { p, r: ra };
        unsafe {
            if (c.c2CircletoCircle)(A, B) == 1 {
                overlaps += 1;
            }
        }
        cc_check(c, r, A, B);
    }
    assert!(overlaps > N / 2, "expected mostly overlaps, got {overlaps}");
}

#[test]
fn row24_cc_disjoint() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x24);
    let mut disjoint = 0;
    for _ in 0..N {
        let p = rng.v_small();
        let ra = 1.0 + (rng.next_u32() % 100) as f32;
        let rb = 1.0 + (rng.next_u32() % 100) as f32;
        let d = (ra + rb) * (1.01 + (rng.next_u32() % 1000) as f32 / 100.0);
        let ang = (rng.next_u32() % 6283) as f32 / 1000.0;
        let A = C2Circle { p, r: ra };
        let B = circle(p.x + d * ang.cos(), p.y + d * ang.sin(), rb);
        unsafe {
            if (c.c2CircletoCircle)(A, B) == 0 {
                disjoint += 1;
            }
        }
        cc_check(c, r, A, B);
    }
    assert!(disjoint > N / 2, "expected mostly disjoint, got {disjoint}");
}

#[test]
fn row25_cc_exact_touch_boundary() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x25);
    for _ in 0..N {
        let x = (rng.next_u32() % 1000) as f32;
        let ra = (rng.next_u32() % 50) as f32;
        let rb = (rng.next_u32() % 50) as f32;
        let A = circle(x, 0.0, ra);
        // exactly touching along x: distance == ra + rb
        let B = circle(x + ra + rb, 0.0, rb);
        cc_check(c, r, A, B);
        // one ULP closer / farther
        let d = ra + rb;
        for delta in [-1i32, 1] {
            let dd = f32::from_bits((d.to_bits() as i32 + delta) as u32);
            cc_check(c, r, A, circle(x + dd, 0.0, rb));
        }
    }
}

#[test]
fn row26_cc_identical_and_zero_radius() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x26);
    for _ in 0..N {
        let p = rng.v_small();
        let a0 = C2Circle { p, r: 0.0 };
        cc_check(c, r, a0, a0); // d2 == r2 == 0 -> 0
        cc_check(c, r, a0, C2Circle { p, r: -0.0 });
        let rr = rng.small_f32();
        let a = C2Circle { p, r: rr };
        cc_check(c, r, a, a);
        cc_check(c, r, a, C2Circle { p, r: 0.0 });
        cc_check(c, r, C2Circle { p, r: 0.0 }, a);
    }
}

#[test]
fn row27_cc_negative_radii() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x27);
    for _ in 0..N {
        let A = circle(rng.small_f32(), rng.small_f32(), -((rng.next_u32() % 100) as f32));
        let B = circle(rng.small_f32(), rng.small_f32(), -((rng.next_u32() % 100) as f32));
        cc_check(c, r, A, B);
        // mixed signs, incl. exact cancellation A.r + B.r == 0
        let m = (rng.next_u32() % 100) as f32;
        cc_check(c, r, circle(0.0, 0.0, m), circle(rng.small_f32(), 0.0, -m));
        cc_check(c, r, circle(0.0, 0.0, -m), circle(rng.small_f32(), 0.0, m));
    }
}

#[test]
fn row28_cc_overflow_to_inf() {
    let (c, r) = pair();
    let pool = [f32::MAX, f32::MIN, 1e30, -1e30, 1e20, 3.4e38, -3.4e38, 1.0, 0.0];
    for &px in &pool {
        for &rr in &pool {
            for &qx in &pool {
                let A = circle(px, 0.0, rr);
                let B = circle(qx, px, rr);
                cc_check(c, r, A, B);
            }
        }
    }
}

#[test]
fn row29_cc_nan_and_inf() {
    let (c, r) = pair();
    let pool = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        0.0,
        1.0,
        -1.0,
    ];
    for &a in &pool {
        for &b in &pool {
            for &d in &pool {
                cc_check(c, r, circle(a, b, d), circle(d, a, b));
                cc_check(c, r, circle(1.0, 1.0, a), circle(2.0, 2.0, b));
                cc_check(c, r, circle(a, 1.0, 1.0), circle(b, 2.0, 2.0));
            }
        }
    }
    let mut rng = Rng::new(0x29);
    for _ in 0..(N * 2) {
        cc_check(c, r, rng.circle_interesting(), rng.circle_interesting());
    }
}

// ===========================================================================
// Rows 30-37 — c2CircletoAABB
// ===========================================================================

fn ca_check(c: &Impl, r: &Impl, A: C2Circle, B: C2Aabb) {
    let args = format!("A={A:?} B={B:?}");
    unsafe {
        assert_i_eq("c2CircletoAABB", &args, (c.c2CircletoAABB)(A, B), (r.c2CircletoAABB)(A, B))
    };
}

fn rand_box(rng: &mut Rng) -> C2Aabb {
    let x0 = rng.small_f32();
    let y0 = rng.small_f32();
    aabb(x0, y0, x0 + (rng.next_u32() % 200) as f32, y0 + (rng.next_u32() % 200) as f32)
}

#[test]
fn row30_ca_centre_inside_box() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x30);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let fx = (rng.next_u32() % 1001) as f32 / 1000.0;
        let fy = (rng.next_u32() % 1001) as f32 / 1000.0;
        let p = v(b.min.x + (b.max.x - b.min.x) * fx, b.min.y + (b.max.y - b.min.y) * fy);
        for rr in [0.0, 1e-30, 0.5, 100.0, -3.0] {
            ca_check(c, r, C2Circle { p, r: rr }, b);
        }
    }
}

#[test]
fn row31_ca_outside_all_eight_directions_overlapping() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x31);
    let dirs: [(f32, f32); 8] =
        [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
    let mut hits = 0;
    for _ in 0..(N / 4) {
        let b = rand_box(&mut rng);
        for &(dx, dy) in &dirs {
            let rr = 1.0 + (rng.next_u32() % 50) as f32;
            let off = rr * 0.5;
            let p = v(
                if dx < 0.0 { b.min.x - off } else if dx > 0.0 { b.max.x + off } else { (b.min.x + b.max.x) * 0.5 },
                if dy < 0.0 { b.min.y - off } else if dy > 0.0 { b.max.y + off } else { (b.min.y + b.max.y) * 0.5 },
            );
            let A = C2Circle { p, r: rr };
            unsafe {
                if (c.c2CircletoAABB)(A, b) == 1 {
                    hits += 1;
                }
            }
            ca_check(c, r, A, b);
        }
    }
    assert!(hits > 0, "expected some overlaps");
}

#[test]
fn row32_ca_outside_disjoint() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x32);
    let mut misses = 0;
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let rr = 1.0 + (rng.next_u32() % 20) as f32;
        let p = v(b.max.x + rr * 4.0, b.max.y + rr * 4.0);
        let A = C2Circle { p, r: rr };
        unsafe {
            if (c.c2CircletoAABB)(A, b) == 0 {
                misses += 1;
            }
        }
        ca_check(c, r, A, b);
    }
    assert!(misses > N / 2, "expected mostly disjoint, got {misses}");
}

#[test]
fn row33_ca_exact_touch_and_corner() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x33);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let rr = 1.0 + (rng.next_u32() % 50) as f32;
        // side touch: distance from centre to clamped point == rr exactly
        for A in [
            circle(b.min.x - rr, (b.min.y + b.max.y) * 0.5, rr),
            circle(b.max.x + rr, (b.min.y + b.max.y) * 0.5, rr),
            circle((b.min.x + b.max.x) * 0.5, b.min.y - rr, rr),
            circle((b.min.x + b.max.x) * 0.5, b.max.y + rr, rr),
            // corner touch along a 3-4-5 triangle so the distance is exact
            circle(b.min.x - 3.0, b.min.y - 4.0, 5.0),
            circle(b.max.x + 3.0, b.max.y + 4.0, 5.0),
        ] {
            ca_check(c, r, A, b);
            // +/- 1 ULP on the radius, straddling the strict `<`
            for delta in [-1i32, 1] {
                let rp = f32::from_bits((A.r.to_bits() as i32 + delta) as u32);
                ca_check(c, r, C2Circle { p: A.p, r: rp }, b);
            }
        }
    }
}

#[test]
fn row34_ca_degenerate_box_and_zero_radius() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x34);
    for _ in 0..N {
        let p = rng.v_small();
        let pt = C2Aabb { min: p, max: p };
        for rr in [0.0f32, -0.0, 1e-30, 1.0, -1.0, f32::MIN_POSITIVE] {
            ca_check(c, r, C2Circle { p, r: rr }, pt);
            ca_check(c, r, circle(p.x + 1.0, p.y, rr), pt);
        }
        let b = rand_box(&mut rng);
        ca_check(c, r, C2Circle { p: b.min, r: 0.0 }, b);
        ca_check(c, r, C2Circle { p: b.max, r: 0.0 }, b);
        // zero-area box on one axis only
        ca_check(c, r, C2Circle { p, r: 1.0 }, aabb(b.min.x, b.min.y, b.min.x, b.max.y));
        ca_check(c, r, C2Circle { p, r: 1.0 }, aabb(b.min.x, b.min.y, b.max.x, b.min.y));
    }
}

#[test]
fn row35_ca_inverted_box() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x35);
    for _ in 0..N {
        let x0 = rng.small_f32();
        let y0 = rng.small_f32();
        let dx = (1 + rng.next_u32() % 200) as f32;
        let dy = (1 + rng.next_u32() % 200) as f32;
        let inv_both = aabb(x0 + dx, y0 + dy, x0, y0);
        let inv_x = aabb(x0 + dx, y0, x0, y0 + dy);
        let inv_y = aabb(x0, y0 + dy, x0 + dx, y0);
        for b in [inv_both, inv_x, inv_y] {
            for rr in [0.0f32, 1.0, 50.0, 1e6, -7.0] {
                ca_check(c, r, C2Circle { p: rng.v_small(), r: rr }, b);
                ca_check(c, r, circle(x0, y0, rr), b);
                ca_check(c, r, circle(x0 + dx, y0 + dy, rr), b);
            }
        }
    }
}

#[test]
fn row36_ca_negative_radius_and_overflow() {
    let (c, r) = pair();
    let pool = [f32::MAX, f32::MIN, 1e30, -1e30, 1e20, 0.0, -0.0, 1.0, -1.0, 3.4e38];
    for &a in &pool {
        for &b in &pool {
            for &d in &pool {
                ca_check(c, r, circle(a, b, d), aabb(b, d, a, b));
                ca_check(c, r, circle(a, a, -d), aabb(-a, -a, a, a));
            }
        }
    }
    let mut rng = Rng::new(0x36);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        ca_check(c, r, circle(rng.small_f32(), rng.small_f32(), -rng.small_f32().abs()), b);
    }
}

#[test]
fn row37_ca_nan_and_inf_all_placements() {
    let (c, r) = pair();
    let nans = [
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut rng = Rng::new(0x37);
    for &nan in &nans {
        for slot in 0..7 {
            for _ in 0..200 {
                let mut s = [1.0f32; 7];
                for x in s.iter_mut() {
                    *x = rng.small_f32();
                }
                s[slot] = nan;
                ca_check(
                    c,
                    r,
                    circle(s[0], s[1], s[2]),
                    aabb(s[3], s[4], s[5], s[6]),
                );
            }
        }
    }
    for _ in 0..(N * 2) {
        ca_check(c, r, rng.circle_interesting(), rng.aabb_interesting());
    }
    let mut rng = Rng::new(0x3737);
    for _ in 0..(N * 2) {
        ca_check(
            c,
            r,
            circle(rng.any_f32(), rng.any_f32(), rng.any_f32()),
            aabb(rng.any_f32(), rng.any_f32(), rng.any_f32(), rng.any_f32()),
        );
    }
}

// ===========================================================================
// Rows 38-43 — c2AABBtoAABB
// ===========================================================================

fn aa_check(c: &Impl, r: &Impl, A: C2Aabb, B: C2Aabb) {
    let args = format!("A={A:?} B={B:?}");
    unsafe { assert_i_eq("c2AABBtoAABB", &args, (c.c2AABBtoAABB)(A, B), (r.c2AABBtoAABB)(A, B)) };
}

#[test]
fn row38_aa_overlapping() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x38);
    let mut hits = 0;
    for _ in 0..N {
        let a = rand_box(&mut rng);
        let fx = (rng.next_u32() % 1001) as f32 / 1000.0;
        let fy = (rng.next_u32() % 1001) as f32 / 1000.0;
        let px = a.min.x + (a.max.x - a.min.x) * fx;
        let py = a.min.y + (a.max.y - a.min.y) * fy;
        let b = aabb(px, py, px + (rng.next_u32() % 100) as f32, py + (rng.next_u32() % 100) as f32);
        unsafe {
            if (c.c2AABBtoAABB)(a, b) == 1 {
                hits += 1;
            }
        }
        aa_check(c, r, a, b);
    }
    assert!(hits > N / 2, "expected mostly overlaps, got {hits}");
}

#[test]
fn row39_aa_disjoint_axes() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x39);
    for _ in 0..N {
        let a = rand_box(&mut rng);
        let w = a.max.x - a.min.x + 1.0;
        let h = a.max.y - a.min.y + 1.0;
        // disjoint on x only
        aa_check(c, r, a, aabb(a.max.x + 1.0, a.min.y, a.max.x + w, a.max.y));
        aa_check(c, r, a, aabb(a.min.x - w, a.min.y, a.min.x - 1.0, a.max.y));
        // disjoint on y only
        aa_check(c, r, a, aabb(a.min.x, a.max.y + 1.0, a.max.x, a.max.y + h));
        aa_check(c, r, a, aabb(a.min.x, a.min.y - h, a.max.x, a.min.y - 1.0));
        // disjoint on both
        aa_check(c, r, a, aabb(a.max.x + 1.0, a.max.y + 1.0, a.max.x + w, a.max.y + h));
        // fully random pairs
        aa_check(c, r, rand_box(&mut rng), rand_box(&mut rng));
    }
}

#[test]
fn row40_aa_edge_touching() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x40);
    for _ in 0..N {
        let a = rand_box(&mut rng);
        // B.min.x == A.max.x  -> touching, `<` false -> collision
        aa_check(c, r, a, aabb(a.max.x, a.min.y, a.max.x + 5.0, a.max.y));
        // B.max.x == A.min.x
        aa_check(c, r, a, aabb(a.min.x - 5.0, a.min.y, a.min.x, a.max.y));
        aa_check(c, r, a, aabb(a.min.x, a.max.y, a.max.x, a.max.y + 5.0));
        aa_check(c, r, a, aabb(a.min.x, a.min.y - 5.0, a.max.x, a.min.y));
        // corner-only touch
        aa_check(c, r, a, aabb(a.max.x, a.max.y, a.max.x + 5.0, a.max.y + 5.0));
        // one ULP past touching
        let nx = f32::from_bits(a.max.x.to_bits() + 1);
        aa_check(c, r, a, aabb(nx, a.min.y, nx + 5.0, a.max.y));
    }
}

#[test]
fn row41_aa_degenerate_inverted_signed_zero() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x41);
    for _ in 0..N {
        let p = rng.v_small();
        let q = rng.v_small();
        aa_check(c, r, C2Aabb { min: p, max: p }, C2Aabb { min: p, max: p });
        aa_check(c, r, C2Aabb { min: p, max: p }, C2Aabb { min: q, max: q });
        // inverted
        aa_check(c, r, C2Aabb { min: q, max: p }, C2Aabb { min: p, max: q });
        aa_check(c, r, C2Aabb { min: q, max: p }, rand_box(&mut rng));
        aa_check(c, r, rand_box(&mut rng), C2Aabb { min: q, max: p });
    }
    let zpool = [0.0f32, -0.0, f32::from_bits(0x0000_0001), f32::from_bits(0x8000_0001)];
    for &a in &zpool {
        for &b in &zpool {
            for &d in &zpool {
                for &e in &zpool {
                    aa_check(c, r, aabb(a, b, d, e), aabb(e, d, b, a));
                    aa_check(c, r, aabb(a, a, b, b), aabb(d, d, e, e));
                }
            }
        }
    }
}

#[test]
fn row42_aa_infinite_corners() {
    let (c, r) = pair();
    let pool = [f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 0.0, 1.0];
    for &a in &pool {
        for &b in &pool {
            for &d in &pool {
                for &e in &pool {
                    aa_check(c, r, aabb(a, b, d, e), aabb(e, d, b, a));
                    aa_check(
                        c,
                        r,
                        aabb(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY),
                        aabb(a, b, d, e),
                    );
                }
            }
        }
    }
}

#[test]
fn row43_aa_nan_in_every_slot() {
    let (c, r) = pair();
    let nans = [
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFDE_AD00),
    ];
    let mut rng = Rng::new(0x43);
    for &nan in &nans {
        for slot in 0..8 {
            for _ in 0..200 {
                let mut s = [0.0f32; 8];
                for x in s.iter_mut() {
                    *x = rng.small_f32();
                }
                s[slot] = nan;
                aa_check(c, r, aabb(s[0], s[1], s[2], s[3]), aabb(s[4], s[5], s[6], s[7]));
            }
        }
    }
    // all-NaN boxes
    for &n in &nans {
        let b = aabb(n, n, n, n);
        aa_check(c, r, b, b);
        aa_check(c, r, b, rand_box(&mut rng));
        aa_check(c, r, rand_box(&mut rng), b);
    }
    for _ in 0..(N * 2) {
        aa_check(c, r, rng.aabb_interesting(), rng.aabb_interesting());
    }
}

// ===========================================================================
// Rows 44-50 — collided (the public header entry point)
// ===========================================================================

/// Calls `collided` in both libraries with the given tags over raw byte
/// buffers, so the tag alone decides how the bytes are reinterpreted.
fn collided_check(c: &Impl, r: &Impl, ba: &[u8], ta: i32, bb: &[u8], tb: i32) {
    let pa = ba.as_ptr() as *const c_void;
    let pb = bb.as_ptr() as *const c_void;
    let args = format!("A={ba:02x?} typeA={ta} B={bb:02x?} typeB={tb}");
    unsafe {
        assert_i_eq("collided", &args, (c.collided)(pa, ta, pb, tb), (r.collided)(pa, ta, pb, tb))
    };
}

fn circle_bytes(x: C2Circle) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&x.p.x.to_le_bytes());
    b[4..8].copy_from_slice(&x.p.y.to_le_bytes());
    b[8..12].copy_from_slice(&x.r.to_le_bytes());
    b
}

fn aabb_bytes(x: C2Aabb) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&x.min.x.to_le_bytes());
    b[4..8].copy_from_slice(&x.min.y.to_le_bytes());
    b[8..12].copy_from_slice(&x.max.x.to_le_bytes());
    b[12..16].copy_from_slice(&x.max.y.to_le_bytes());
    b
}

#[test]
fn row44_collided_circle_circle() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x44);
    for _ in 0..(N * 2) {
        let A = rng.circle_interesting();
        let B = rng.circle_interesting();
        collided_check(c, r, &circle_bytes(A), C2_TYPE_CIRCLE, &circle_bytes(B), C2_TYPE_CIRCLE);
        // and agreement with the low-level function through the C .so
        unsafe {
            let via_wrapper = (r.collided)(
                circle_bytes(A).as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
                circle_bytes(B).as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
            );
            let direct = (c.c2CircletoCircle)(A, B);
            assert_i_eq("collided(CIRCLE,CIRCLE) vs c2CircletoCircle", "", direct, via_wrapper);
        }
    }
    // geometric sweep: reuse the overlapping/disjoint generators
    let mut rng = Rng::new(0x4444);
    for _ in 0..N {
        let p = rng.v_small();
        let ra = 1.0 + (rng.next_u32() % 100) as f32;
        let rb = 1.0 + (rng.next_u32() % 100) as f32;
        let d = (ra + rb) * (rng.next_u32() % 300) as f32 / 100.0;
        let A = C2Circle { p, r: ra };
        let B = circle(p.x + d, p.y, rb);
        collided_check(c, r, &circle_bytes(A), C2_TYPE_CIRCLE, &circle_bytes(B), C2_TYPE_CIRCLE);
    }
}

#[test]
fn row45_collided_circle_aabb() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x45);
    for _ in 0..(N * 2) {
        let A = rng.circle_interesting();
        let B = rng.aabb_interesting();
        collided_check(c, r, &circle_bytes(A), C2_TYPE_CIRCLE, &aabb_bytes(B), C2_TYPE_AABB);
    }
    let mut rng = Rng::new(0x4545);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let A = circle(rng.small_f32(), rng.small_f32(), (rng.next_u32() % 200) as f32);
        collided_check(c, r, &circle_bytes(A), C2_TYPE_CIRCLE, &aabb_bytes(b), C2_TYPE_AABB);
    }
}

#[test]
fn row46_collided_aabb_circle_swapped_dispatch() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x46);
    for _ in 0..(N * 2) {
        // typeA == AABB, typeB == CIRCLE: C reads *B* as the circle and *A* as the box
        let boxx = rng.aabb_interesting();
        let circ = rng.circle_interesting();
        collided_check(c, r, &aabb_bytes(boxx), C2_TYPE_AABB, &circle_bytes(circ), C2_TYPE_CIRCLE);
    }
    let mut rng = Rng::new(0x4646);
    for _ in 0..N {
        let b = rand_box(&mut rng);
        let circ = circle(rng.small_f32(), rng.small_f32(), (rng.next_u32() % 200) as f32);
        collided_check(c, r, &aabb_bytes(b), C2_TYPE_AABB, &circle_bytes(circ), C2_TYPE_CIRCLE);
        // asymmetry check: (AABB,CIRCLE) must equal (CIRCLE,AABB) with operands swapped
        unsafe {
            let x = (r.collided)(
                aabb_bytes(b).as_ptr() as *const c_void,
                C2_TYPE_AABB,
                circle_bytes(circ).as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
            );
            let y = (c.collided)(
                circle_bytes(circ).as_ptr() as *const c_void,
                C2_TYPE_CIRCLE,
                aabb_bytes(b).as_ptr() as *const c_void,
                C2_TYPE_AABB,
            );
            assert_i_eq("collided swap symmetry", "", y, x);
        }
    }
}

#[test]
fn row47_collided_aabb_aabb() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x47);
    for _ in 0..(N * 2) {
        let A = rng.aabb_interesting();
        let B = rng.aabb_interesting();
        collided_check(c, r, &aabb_bytes(A), C2_TYPE_AABB, &aabb_bytes(B), C2_TYPE_AABB);
    }
    let mut rng = Rng::new(0x4747);
    for _ in 0..N {
        collided_check(
            c,
            r,
            &aabb_bytes(rand_box(&mut rng)),
            C2_TYPE_AABB,
            &aabb_bytes(rand_box(&mut rng)),
            C2_TYPE_AABB,
        );
    }
}

#[test]
fn row48_collided_aliased_pointer() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x48);
    for _ in 0..N {
        let buf = circle_bytes(rng.circle_interesting());
        for &(ta, tb) in
            &[(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, C2_TYPE_AABB), (C2_TYPE_AABB, C2_TYPE_CIRCLE), (C2_TYPE_AABB, C2_TYPE_AABB)]
        {
            let p = buf.as_ptr() as *const c_void;
            let args = format!("aliased {buf:02x?} {ta} {tb}");
            unsafe {
                assert_i_eq("collided", &args, (c.collided)(p, ta, p, tb), (r.collided)(p, ta, p, tb))
            };
        }
    }
}

#[test]
fn row49_collided_tag_decides_reinterpretation() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x49);
    for _ in 0..(N * 2) {
        // one shared 16-byte pattern read as circle (12 B) or box (16 B)
        let mut ba = [0u8; 16];
        let mut bb = [0u8; 16];
        for i in 0..4 {
            ba[i * 4..i * 4 + 4].copy_from_slice(&rng.interesting_f32().to_le_bytes());
            bb[i * 4..i * 4 + 4].copy_from_slice(&rng.interesting_f32().to_le_bytes());
        }
        for &(ta, tb) in
            &[(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, C2_TYPE_AABB), (C2_TYPE_AABB, C2_TYPE_CIRCLE), (C2_TYPE_AABB, C2_TYPE_AABB)]
        {
            collided_check(c, r, &ba, ta, &bb, tb);
        }
    }
}

#[test]
fn row50_collided_special_payloads() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x50);
    for _ in 0..(N * 2) {
        let mut ba = [0u8; 16];
        let mut bb = [0u8; 16];
        for i in 0..4 {
            ba[i * 4..i * 4 + 4].copy_from_slice(&SPECIAL[rng.below(SPECIAL.len() as u32) as usize].to_le_bytes());
            bb[i * 4..i * 4 + 4].copy_from_slice(&SPECIAL[rng.below(SPECIAL.len() as u32) as usize].to_le_bytes());
        }
        for &(ta, tb) in
            &[(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, C2_TYPE_AABB), (C2_TYPE_AABB, C2_TYPE_CIRCLE), (C2_TYPE_AABB, C2_TYPE_AABB)]
        {
            collided_check(c, r, &ba, ta, &bb, tb);
        }
    }
    // fully random bytes
    let mut rng = Rng::new(0x5050);
    for _ in 0..(N * 2) {
        let mut ba = [0u8; 16];
        let mut bb = [0u8; 16];
        for i in 0..4 {
            ba[i * 4..i * 4 + 4].copy_from_slice(&rng.next_u32().to_le_bytes());
            bb[i * 4..i * 4 + 4].copy_from_slice(&rng.next_u32().to_le_bytes());
        }
        for &(ta, tb) in
            &[(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, C2_TYPE_AABB), (C2_TYPE_AABB, C2_TYPE_CIRCLE), (C2_TYPE_AABB, C2_TYPE_AABB)]
        {
            collided_check(c, r, &ba, ta, &bb, tb);
        }
    }
}

// ===========================================================================
// Row 51 — full end-to-end sweep over all ten exports at once
// ===========================================================================

#[test]
fn row51_full_sweep_all_exports() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..(N * 3) {
        let a = rng.v_interesting();
        let b = rng.v_interesting();
        let ci = rng.v_interesting();
        unsafe {
            assert_v_eq("c2V", "sweep", (c.c2V)(a.x, a.y), (r.c2V)(a.x, a.y));
            assert_v_eq("c2Maxv", "sweep", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            assert_v_eq("c2Minv", "sweep", (c.c2Minv)(a, b), (r.c2Minv)(a, b));
            assert_v_eq("c2Clampv", "sweep", (c.c2Clampv)(a, b, ci), (r.c2Clampv)(a, b, ci));
            assert_v_eq("c2Sub", "sweep", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
            assert_f_eq("c2Dot", "sweep", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        }
        let A = C2Circle { p: a, r: rng.interesting_f32() };
        let B = C2Circle { p: b, r: rng.interesting_f32() };
        let bx = C2Aabb { min: b, max: ci };
        let by = C2Aabb { min: ci, max: a };
        cc_check(c, r, A, B);
        ca_check(c, r, A, bx);
        aa_check(c, r, bx, by);
        for &(ta, tb) in
            &[(C2_TYPE_CIRCLE, C2_TYPE_CIRCLE), (C2_TYPE_CIRCLE, C2_TYPE_AABB), (C2_TYPE_AABB, C2_TYPE_CIRCLE), (C2_TYPE_AABB, C2_TYPE_AABB)]
        {
            collided_check(c, r, &aabb_bytes(bx), ta, &aabb_bytes(by), tb);
        }
    }
}
