//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every call goes through `dlopen`/`dlsym` on BOTH shared objects (see
//! `tests/common/mod.rs`); the Rust implementation is never invoked directly.

mod common;

use common::{lib, samples, Checker, Rng, BOUNDARY_GRID, INT_MAX, INT_MIN, SEED};

// ---------------------------------------------------------------------------
// generators for the regions each C path occupies
// ---------------------------------------------------------------------------

fn pos(r: &mut Rng) -> i32 {
    r.range_i32(1, INT_MAX)
}
fn nonneg(r: &mut Rng) -> i32 {
    r.range_i32(0, INT_MAX)
}
fn neg_not_min(r: &mut Rng) -> i32 {
    r.range_i32(INT_MIN + 1, -1)
}

/// Build `v1 = sign1 * (k * m + rem)` with `m > 0`, keeping the magnitude inside
/// `i32`. Returns `None` when the product would not fit.
fn compose(k: i64, m: i64, rem: i64, negate: bool) -> Option<i32> {
    let mag = k.checked_mul(m)?.checked_add(rem)?;
    if negate {
        let v = -mag;
        if v < INT_MIN as i64 {
            return None;
        }
        Some(v as i32)
    } else {
        if mag > INT_MAX as i64 {
            return None;
        }
        Some(mag as i32)
    }
}

/// Powers of two in `[1, 2^30]` — exactly the positive divisors of `2^31`
/// (i.e. of `|INT_MIN|`) that are representable as `i32`.
fn pow2_divisors_of_int_min() -> Vec<i32> {
    (0..31).map(|k| 1i32 << k).collect()
}

// ---------------------------------------------------------------------------
// C1 — P0: v2 == 0
// ---------------------------------------------------------------------------
#[test]
fn c1_p0_divisor_zero() {
    let mut ck = Checker::new("C1 P0 v2==0");
    let mut r = Rng::new(SEED ^ 1);
    for v1 in BOUNDARY_GRID {
        ck.check(v1, 0);
    }
    for _ in 0..samples() {
        ck.check(r.next_i32(), 0);
        ck.check(r.nasty_i32(), 0);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C2 — P1: v1 > 0, v2 > 0, v1 < v2 (quotient 0)
// ---------------------------------------------------------------------------
#[test]
fn c2_p1_quotient_zero() {
    let mut ck = Checker::new("C2 P1 v1<v2");
    let mut r = Rng::new(SEED ^ 2);
    for _ in 0..samples() {
        let v2 = r.range_i32(2, INT_MAX);
        let v1 = r.range_i32(1, v2 - 1);
        ck.check(v1, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C3 — P1: v1 > v2 > 0, exactly divisible
// ---------------------------------------------------------------------------
#[test]
fn c3_p1_exact() {
    let mut ck = Checker::new("C3 P1 divisible");
    let mut r = Rng::new(SEED ^ 3);
    let mut n = 0u64;
    while n < samples() {
        let v2 = r.range_i32(2, 46_341) as i64;
        let kmax = (INT_MAX as i64) / v2;
        if kmax < 2 {
            continue;
        }
        let k = r.range(2, kmax);
        if let Some(v1) = compose(k, v2, 0, false) {
            ck.check(v1, v2 as i32);
            n += 1;
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C4 — P1: v1 > v2 > 0, not divisible
// ---------------------------------------------------------------------------
#[test]
fn c4_p1_inexact() {
    let mut ck = Checker::new("C4 P1 not divisible");
    let mut r = Rng::new(SEED ^ 4);
    let mut n = 0u64;
    while n < samples() {
        let v2 = r.range_i32(2, 1 << 30) as i64;
        let kmax = (INT_MAX as i64) / v2;
        if kmax < 1 {
            continue;
        }
        let k = r.range(1, kmax);
        let rem = r.range(1, v2 - 1);
        if let Some(v1) = compose(k, v2, rem, false) {
            ck.check(v1, v2 as i32);
            n += 1;
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C5 — P1 boundary shapes
// ---------------------------------------------------------------------------
#[test]
fn c5_p1_boundaries() {
    let mut ck = Checker::new("C5 P1 boundaries");
    let mut r = Rng::new(SEED ^ 5);
    for _ in 0..samples() {
        let v = pos(&mut r);
        ck.check(0, v); // v1 == 0
        ck.check(v, v); // v1 == v2
        ck.check(v, 1); // v2 == 1
        ck.check(INT_MAX, v);
        ck.check(v, INT_MAX);
        ck.check(nonneg(&mut r), INT_MAX);
    }
    for &a in &[0, 1, 2, 3, INT_MAX - 1, INT_MAX] {
        for &b in &[1, 2, 3, INT_MAX - 1, INT_MAX] {
            ck.check(a, b);
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C6 — P2: v1 > 0, v2 < 0 (!= INT_MIN), divisible
// ---------------------------------------------------------------------------
#[test]
fn c6_p2_exact() {
    let mut ck = Checker::new("C6 P2 divisible");
    let mut r = Rng::new(SEED ^ 6);
    let mut n = 0u64;
    while n < samples() {
        let m = r.range(1, 46_341); // m = -v2
        let kmax = (INT_MAX as i64) / m;
        if kmax < 1 {
            continue;
        }
        let k = r.range(1, kmax);
        if let Some(v1) = compose(k, m, 0, false) {
            ck.check(v1, -(m as i32));
            n += 1;
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C7 — P2: v1 > 0, v2 < 0 (!= INT_MIN), not divisible
// ---------------------------------------------------------------------------
#[test]
fn c7_p2_inexact() {
    let mut ck = Checker::new("C7 P2 not divisible");
    let mut r = Rng::new(SEED ^ 7);
    let mut n = 0u64;
    while n < samples() {
        let m = r.range(2, 1 << 30);
        let kmax = (INT_MAX as i64) / m;
        if kmax < 1 {
            continue;
        }
        let k = r.range(1, kmax);
        let rem = r.range(1, m - 1);
        if let Some(v1) = compose(k, m, rem, false) {
            ck.check(v1, -(m as i32));
            n += 1;
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C8 — P2 boundary shapes
// ---------------------------------------------------------------------------
#[test]
fn c8_p2_boundaries() {
    let mut ck = Checker::new("C8 P2 boundaries");
    let mut r = Rng::new(SEED ^ 8);
    for _ in 0..samples() {
        let v2 = neg_not_min(&mut r);
        ck.check(0, v2);
        ck.check(nonneg(&mut r), -1);
        ck.check(INT_MAX, v2);
        ck.check(nonneg(&mut r), INT_MIN + 1);
        // |v1| < |v2| => quotient 0
        let m = (v2 as i64).unsigned_abs() as i64;
        if m >= 2 {
            let v1 = r.range(0, m - 1) as i32;
            ck.check(v1, v2);
        }
    }
    for &a in &[0, 1, 2, INT_MAX - 1, INT_MAX] {
        for &b in &[-1, -2, -3, INT_MIN + 2, INT_MIN + 1] {
            ck.check(a, b);
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C9 — P3: v1 >= 0, v2 == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn c9_p3_v2_int_min_nonneg_v1() {
    let mut ck = Checker::new("C9 P3");
    let mut r = Rng::new(SEED ^ 9);
    for v1 in [0, 1, 2, 3, INT_MAX - 1, INT_MAX] {
        ck.check(v1, INT_MIN);
    }
    for _ in 0..samples() {
        ck.check(nonneg(&mut r), INT_MIN);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C10 — P4: v1 < 0 (!= INT_MIN), v2 > 0, divisible
// ---------------------------------------------------------------------------
#[test]
fn c10_p4_exact() {
    let mut ck = Checker::new("C10 P4 divisible");
    let mut r = Rng::new(SEED ^ 10);
    let mut n = 0u64;
    while n < samples() {
        let v2 = r.range(1, 46_341);
        let kmax = (INT_MAX as i64) / v2;
        if kmax < 1 {
            continue;
        }
        let k = r.range(1, kmax);
        if let Some(v1) = compose(k, v2, 0, true) {
            if v1 != INT_MIN && v1 < 0 {
                ck.check(v1, v2 as i32);
                n += 1;
            }
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C11 — P4: v1 < 0 (!= INT_MIN), v2 > 0, not divisible (epilogue q-1)
// ---------------------------------------------------------------------------
#[test]
fn c11_p4_inexact() {
    let mut ck = Checker::new("C11 P4 not divisible");
    let mut r = Rng::new(SEED ^ 11);
    let mut n = 0u64;
    while n < samples() {
        let v2 = r.range(2, 1 << 30);
        let kmax = (INT_MAX as i64) / v2;
        if kmax < 1 {
            continue;
        }
        let k = r.range(0, kmax);
        let rem = r.range(1, v2 - 1);
        if let Some(v1) = compose(k, v2, rem, true) {
            if v1 != INT_MIN && v1 < 0 {
                ck.check(v1, v2 as i32);
                n += 1;
            }
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C12 — P4 boundary shapes
// ---------------------------------------------------------------------------
#[test]
fn c12_p4_boundaries() {
    let mut ck = Checker::new("C12 P4 boundaries");
    let mut r = Rng::new(SEED ^ 12);
    for _ in 0..samples() {
        let v1 = neg_not_min(&mut r);
        ck.check(v1, 1);
        ck.check(v1, INT_MAX);
        ck.check(INT_MIN + 1, pos(&mut r));
        // |v1| < v2 => quotient 0, r < 0
        let v2 = r.range_i32(2, INT_MAX);
        ck.check(-r.range(1, (v2 as i64) - 1) as i32, v2);
    }
    for &a in &[-1, -2, -3, INT_MIN + 2, INT_MIN + 1] {
        for &b in &[1, 2, 3, INT_MAX - 1, INT_MAX] {
            ck.check(a, b);
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C13 — P5: v1 < 0 (!= INT_MIN), v2 < 0 (!= INT_MIN), divisible
// ---------------------------------------------------------------------------
#[test]
fn c13_p5_exact() {
    let mut ck = Checker::new("C13 P5 divisible");
    let mut r = Rng::new(SEED ^ 13);
    let mut n = 0u64;
    while n < samples() {
        let m = r.range(1, 46_341);
        let kmax = (INT_MAX as i64) / m;
        if kmax < 1 {
            continue;
        }
        let k = r.range(1, kmax);
        if let Some(v1) = compose(k, m, 0, true) {
            if v1 != INT_MIN && v1 < 0 {
                ck.check(v1, -(m as i32));
                n += 1;
            }
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C14 — P5: v1 < 0 (!= INT_MIN), v2 < 0 (!= INT_MIN), not divisible (q+1)
// ---------------------------------------------------------------------------
#[test]
fn c14_p5_inexact() {
    let mut ck = Checker::new("C14 P5 not divisible");
    let mut r = Rng::new(SEED ^ 14);
    let mut n = 0u64;
    while n < samples() {
        let m = r.range(2, 1 << 30);
        let kmax = (INT_MAX as i64) / m;
        if kmax < 1 {
            continue;
        }
        let k = r.range(0, kmax);
        let rem = r.range(1, m - 1);
        if let Some(v1) = compose(k, m, rem, true) {
            if v1 != INT_MIN && v1 < 0 {
                ck.check(v1, -(m as i32));
                n += 1;
            }
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C15 — P5 boundary shapes (incl. v2 == -1 near-overflow negation)
// ---------------------------------------------------------------------------
#[test]
fn c15_p5_boundaries() {
    let mut ck = Checker::new("C15 P5 boundaries");
    let mut r = Rng::new(SEED ^ 15);
    for _ in 0..samples() {
        let v1 = neg_not_min(&mut r);
        ck.check(v1, -1);
        ck.check(v1, INT_MIN + 1);
        ck.check(INT_MIN + 1, neg_not_min(&mut r));
        let v2 = r.range_i32(INT_MIN + 1, -2);
        let m = (v2 as i64).unsigned_abs() as i64;
        ck.check(-r.range(1, m - 1) as i32, v2);
    }
    for &a in &[-1, -2, -3, INT_MIN + 2, INT_MIN + 1] {
        for &b in &[-1, -2, -3, INT_MIN + 2, INT_MIN + 1] {
            ck.check(a, b);
        }
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C16 — P6: v1 < 0 (!= INT_MIN), v2 == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn c16_p6_v2_int_min_negative_v1() {
    let mut ck = Checker::new("C16 P6");
    let mut r = Rng::new(SEED ^ 16);
    for v1 in [-1, -2, -3, INT_MIN + 2, INT_MIN + 1] {
        ck.check(v1, INT_MIN);
    }
    for _ in 0..samples() {
        ck.check(neg_not_min(&mut r), INT_MIN);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C17 — P7: v1 == INT_MIN, v2 > 0, divisible
// ---------------------------------------------------------------------------
#[test]
fn c17_p7_exact() {
    let mut ck = Checker::new("C17 P7 divisible");
    for v2 in pow2_divisors_of_int_min() {
        ck.check(INT_MIN, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C18 — P7: v1 == INT_MIN, v2 > 0, not divisible
// ---------------------------------------------------------------------------
#[test]
fn c18_p7_inexact() {
    let mut ck = Checker::new("C18 P7 not divisible");
    let mut r = Rng::new(SEED ^ 18);
    for _ in 0..samples() {
        let v2 = r.range_i32(1, INT_MAX);
        ck.check(INT_MIN, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C19 — P7 boundary divisors
// ---------------------------------------------------------------------------
#[test]
fn c19_p7_boundaries() {
    let mut ck = Checker::new("C19 P7 boundaries");
    for v2 in [1, 2, 3, 4, 5, 7, INT_MAX - 2, INT_MAX - 1, INT_MAX] {
        ck.check(INT_MIN, v2);
    }
    // the whole low end plus the whole high end of the positive divisor range
    for v2 in 1..=4096 {
        ck.check(INT_MIN, v2);
    }
    for v2 in (INT_MAX - 4096)..=INT_MAX {
        ck.check(INT_MIN, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C20 — P8: v1 == INT_MIN, v2 < 0 (!= INT_MIN), divisible
// ---------------------------------------------------------------------------
#[test]
fn c20_p8_exact() {
    let mut ck = Checker::new("C20 P8 divisible");
    for m in pow2_divisors_of_int_min() {
        ck.check(INT_MIN, -m);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C21 — P8: v1 == INT_MIN, v2 < 0 (!= INT_MIN), not divisible
// ---------------------------------------------------------------------------
#[test]
fn c21_p8_inexact() {
    let mut ck = Checker::new("C21 P8 not divisible");
    let mut r = Rng::new(SEED ^ 21);
    for _ in 0..samples() {
        let v2 = r.range_i32(INT_MIN + 1, -1);
        ck.check(INT_MIN, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C22 — P8 boundary divisors, incl. the INT_MIN / -1 trap input
// ---------------------------------------------------------------------------
#[test]
fn c22_p8_boundaries() {
    let mut ck = Checker::new("C22 P8 boundaries");
    for v2 in [-1, -2, -3, -4, -5, -7, INT_MIN + 1, INT_MIN + 2, INT_MIN + 3] {
        ck.check(INT_MIN, v2);
    }
    for v2 in -4096..=-1 {
        ck.check(INT_MIN, v2);
    }
    for v2 in (INT_MIN + 1)..=(INT_MIN + 4096) {
        ck.check(INT_MIN, v2);
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C23 — P9: v1 == INT_MIN, v2 == INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn c23_p9_both_int_min() {
    let mut ck = Checker::new("C23 P9");
    ck.check(INT_MIN, INT_MIN);
    ck.finish();
}

// ---------------------------------------------------------------------------
// C24 — unconstrained uniform random over the whole i32 x i32 space
// ---------------------------------------------------------------------------
#[test]
fn c24_uniform_random() {
    let mut ck = Checker::new("C24 uniform random");
    let mut r = Rng::new(SEED ^ 24);
    for _ in 0..(samples() * 25) {
        ck.check(r.next_i32(), r.next_i32());
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C25 — boundary-biased random over the whole space
// ---------------------------------------------------------------------------
#[test]
fn c25_nasty_random() {
    let mut ck = Checker::new("C25 boundary-biased random");
    let mut r = Rng::new(SEED ^ 25);
    for _ in 0..(samples() * 25) {
        ck.check(r.nasty_i32(), r.nasty_i32());
    }
    ck.finish();
}

// ---------------------------------------------------------------------------
// C26 — full 9x9 boundary grid cross product
// ---------------------------------------------------------------------------
#[test]
fn c26_boundary_grid_cross_product() {
    let mut ck = Checker::new("C26 boundary grid 9x9");
    for a in BOUNDARY_GRID {
        for b in BOUNDARY_GRID {
            ck.check(a, b);
        }
    }
    assert_eq!(ck.cases(), 81);
    ck.finish();
}

// ---------------------------------------------------------------------------
// harness self-check: confirm both .so files were really loaded
// ---------------------------------------------------------------------------
#[test]
fn c0_harness_loads_both_shared_objects() {
    let l = lib();
    assert!(l.c_path.exists(), "C .so missing: {}", l.c_path.display());
    assert!(
        l.rust_path.exists(),
        "Rust .so missing: {}",
        l.rust_path.display()
    );
    assert_ne!(l.c_path, l.rust_path);
    println!("C   .so: {}", l.c_path.display());
    println!("Rust.so: {}", l.rust_path.display());
    // sanity: a value both must agree on
    assert_eq!(l.c(7, 3), l.rust(7, 3));
}
