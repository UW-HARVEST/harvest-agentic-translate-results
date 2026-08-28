//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! Every test constructs the exact triggering condition, calls **both** `.so`s,
//! and asserts they return the same value. Where the C source dictates a
//! specific sentinel (e.g. the `v2 == 0` guard's `return 0`) the test also pins
//! that exact value, so "both agree on the wrong thing" cannot pass.

mod common;

use common::{check, check_eq, Cmp, Pcg32, BOUNDARIES, I32_MAX, I32_MIN};

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Row 1 — `if (v2 == 0) return 0;`  (lib.c:4-6) divide-by-zero rejection
// ---------------------------------------------------------------------------

#[test]
fn err_row01_v2_zero_any_v1() {
    // Sentinel is a literal 0 for *every* v1, including both extremes.
    for &v1 in BOUNDARIES {
        check_eq(v1, 0, 0);
    }
    check_eq(I32_MIN, 0, 0);
    check_eq(I32_MAX, 0, 0);
    check_eq(0, 0, 0);

    let mut rng = Pcg32::new(0xC001_0001);
    let mut cmp = Cmp::new("row01 v2 == 0");
    for _ in 0..N {
        let v1 = rng.i32_any();
        assert_eq!(check(v1, 0), 0, "v2==0 guard must return 0 for v1={v1}");
        cmp.feed(v1, 0);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 2 — `v2 != INT_MIN` range check FAILS with v1 >= 0 (lib.c:11 -> :14)
//         q = 0, r = v1 >= 0 -> returns 0
// ---------------------------------------------------------------------------

#[test]
fn err_row02_v1_nonneg_v2_intmin() {
    check_eq(0, I32_MIN, 0);
    check_eq(1, I32_MIN, 0);
    check_eq(2, I32_MIN, 0);
    check_eq(I32_MAX, I32_MIN, 0);
    check_eq(I32_MAX - 1, I32_MIN, 0);
    check_eq(1 << 30, I32_MIN, 0);

    let mut rng = Pcg32::new(0xC001_0002);
    let mut cmp = Cmp::new("row02 v1>=0, v2==INT_MIN");
    for _ in 0..N {
        let v1 = rng.nonneg();
        assert_eq!(check(v1, I32_MIN), 0, "L3 must return 0 for v1={v1}");
        cmp.feed(v1, I32_MIN);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 3 — `v1 != INT_MIN` range check FAILS (lib.c:15) so `-v1` is never
//         evaluated; control reaches the :22/:24/:26 ladder instead.
// ---------------------------------------------------------------------------

#[test]
fn err_row03_v1_intmin_guard() {
    // If the guard were mistranslated (e.g. the ordinary `-v1` paths taken),
    // these three anchors would differ. They are the three outcomes of the
    // INT_MIN ladder.
    check_eq(I32_MIN, 1, I32_MIN); // :23 branch, r == 0
    check_eq(I32_MIN, -1, I32_MIN); // :25 branch, overflow sub-case
    check_eq(I32_MIN, I32_MIN, 1); // :27 branch

    // and agreement across the whole v2 axis for the pinned v1 == INT_MIN
    let mut rng = Pcg32::new(0xC001_0003);
    let mut cmp = Cmp::new("row03 v1 == INT_MIN, all v2");
    for &v2 in BOUNDARIES {
        cmp.feed(I32_MIN, v2);
    }
    for _ in 0..N {
        cmp.feed(I32_MIN, rng.i32_any());
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 4 — `v2 != INT_MIN` FAILS with v1 < 0 && v1 != INT_MIN (lib.c:18 -> :21)
//         q = 1, r = v1 - INT_MIN > 0 -> returns 1
// ---------------------------------------------------------------------------

#[test]
fn err_row04_v1_neg_nonmin_v2_intmin() {
    check_eq(-1, I32_MIN, 1);
    check_eq(-2, I32_MIN, 1);
    check_eq(I32_MIN + 1, I32_MIN, 1);
    check_eq(I32_MIN + 2, I32_MIN, 1);
    check_eq(-(1 << 30), I32_MIN, 1);

    let mut rng = Pcg32::new(0xC001_0004);
    let mut cmp = Cmp::new("row04 v1<0 non-MIN, v2==INT_MIN");
    for _ in 0..N {
        let v1 = rng.neg_nonmin();
        assert_eq!(check(v1, I32_MIN), 1, "L6 must return 1 for v1={v1}");
        cmp.feed(v1, I32_MIN);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 5 — `v2 != INT_MIN` FAILS with v1 == INT_MIN (lib.c:24 -> :27)
//         q = 1, r = 0 -> returns 1
// ---------------------------------------------------------------------------

#[test]
fn err_row05_both_intmin() {
    check_eq(I32_MIN, I32_MIN, 1);
}

// ---------------------------------------------------------------------------
// Row 6 — v1 == INT_MIN, v2 >= 1: the `-(v1 + v2)` INT_MIN-safe rewrite (:23)
// ---------------------------------------------------------------------------

#[test]
fn err_row06_v1_intmin_v2_pos() {
    // Values derived by hand from lib.c:23 + the :28 tail.
    check_eq(I32_MIN, 1, I32_MIN); // t=INT_MAX, q=-INT_MAX-1, r=0
    check_eq(I32_MIN, 2, -1073741824); // exact
    check_eq(I32_MIN, 3, -715827883); // r=-2 <0, v2>0 -> q-1
    check_eq(I32_MIN, 4, -536870912); // exact
    check_eq(I32_MIN, 1 << 30, -2); // exact
    check_eq(I32_MIN, I32_MAX, -2); // r<0 -> q-1
    check_eq(I32_MIN, I32_MAX - 1, -2);

    let mut rng = Pcg32::new(0xC001_0006);
    let mut cmp = Cmp::new("row06 v1==INT_MIN, v2>0");
    for _ in 0..N {
        cmp.feed(I32_MIN, rng.pos());
    }
    for v2 in 1..=2000i32 {
        cmp.feed(I32_MIN, v2);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 7 — v1 == INT_MIN, v2 < 0 && v2 != INT_MIN: the `-(v1 - v2)` rewrite (:25)
// ---------------------------------------------------------------------------

#[test]
fn err_row07_v1_intmin_v2_neg_nonmin() {
    check_eq(I32_MIN, -2, 1073741824); // exact
    check_eq(I32_MIN, -3, 715827883); // r=-2 <0, v2<0 -> q+1
    check_eq(I32_MIN, -4, 536870912);
    check_eq(I32_MIN, -(1 << 30), 2);
    check_eq(I32_MIN, I32_MIN + 1, 2); // r<0 -> q+1

    let mut rng = Pcg32::new(0xC001_0007);
    let mut cmp = Cmp::new("row07 v1==INT_MIN, v2<0 non-MIN");
    for _ in 0..N {
        cmp.feed(I32_MIN, rng.neg_nonmin());
    }
    for v2 in -2000..=-1i32 {
        cmp.feed(I32_MIN, v2);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 8 — v1 == INT_MIN, v2 == -1: signed overflow `q = INT_MAX + 1`
// ---------------------------------------------------------------------------

#[test]
fn err_row08_v1_intmin_v2_minus_one_overflow() {
    // -(v1-v2) = INT_MAX; INT_MAX/1 = INT_MAX; q = INT_MAX + 1 overflows and
    // the -O0 C build wraps it to INT_MIN; r = -(INT_MAX % 1) = 0 -> return q.
    check_eq(I32_MIN, -1, I32_MIN);

    // Neighbours, to prove the wrap is confined to exactly this input.
    check_eq(I32_MIN + 1, -1, I32_MAX);
    check_eq(I32_MIN, -2, 1073741824);
    check_eq(I32_MAX, -1, -I32_MAX);
}

// ---------------------------------------------------------------------------
// Row 9 — tail `if (r >= 0) return q;` (lib.c:28-29)
// ---------------------------------------------------------------------------

#[test]
fn err_row09_tail_r_nonneg() {
    // r == 0 reachable from every leaf that reaches the tail: L2, L3, L4, L5,
    // L6, L7, L8, L9. One exact-multiple representative each.
    check(84, -12); // L2, r = 84 % 12 = 0
    check_eq(5, I32_MIN, 0); // L3, r = 5 >= 0
    check(-84, 12); // L4, r = -(84 % 12) = 0
    check(-84, -12); // L5, r = 0
    check_eq(-5, I32_MIN, 1); // L6, r = -5 - INT_MIN > 0
    check_eq(I32_MIN, 2, -1073741824); // L7, r = 0
    check_eq(I32_MIN, -2, 1073741824); // L8, r = 0
    check_eq(I32_MIN, I32_MIN, 1); // L9, r = 0

    // r > 0 (strictly) is only produced by L2 and L3/L6.
    check(85, -12); // L2, r = 1 > 0
    check(1, -3); // L2, r = 1 > 0

    let mut rng = Pcg32::new(0xC001_0009);
    let mut cmp = Cmp::new("row09 tail r >= 0 (exact multiples)");
    for _ in 0..N {
        // build exact multiples in all four sign combinations
        let m = rng.i32_in(1, 1 << 15);
        let k = rng.i32_in(0, 1 << 15);
        let p = k * m;
        for &(a, b) in &[(p, m), (p, -m), (-p, m), (-p, -m)] {
            cmp.feed(a, b);
        }
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 10 — tail `r < 0` with `v2 > 0` -> `q + (-1)` (lib.c:31 true arm)
// ---------------------------------------------------------------------------

#[test]
fn err_row10_tail_r_neg_v2_pos() {
    // Reachable from L4 (v1<0 non-MIN, v2>0) and L7 (v1==INT_MIN, v2>0).
    check_eq(-85, 12, -8); // L4: q = -(85/12) = -7, r = -1 -> -8
    check_eq(-1, 2, -1); // L4: q = 0, r = -1 -> -1
    check_eq(I32_MIN, 3, -715827883); // L7
    check_eq(I32_MIN, I32_MAX, -2); // L7

    let mut rng = Pcg32::new(0xC001_000a);
    let mut cmp = Cmp::new("row10 tail r<0, v2>0");
    for _ in 0..N {
        let v2 = rng.i32_in(2, I32_MAX);
        let v1 = rng.i32_in(I32_MIN + 1, -1);
        cmp.feed(v1, v2);
    }
    for v2 in 2..=500i32 {
        for v1 in -500..=-1i32 {
            cmp.feed(v1, v2);
        }
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 11 — tail `r < 0` with `v2 < 0` -> `q + 1` (lib.c:31 false arm)
// ---------------------------------------------------------------------------

#[test]
fn err_row11_tail_r_neg_v2_neg() {
    // Reachable from L5 (v1<0 non-MIN, v2<0 non-MIN) and L8 (v1==INT_MIN, v2<0).
    check_eq(-85, -12, 8); // L5: q = 85/12 = 7, r = -1 -> 8
    check_eq(-1, -2, 1); // L5: q = 0, r = -1 -> 1
    check_eq(I32_MIN, -3, 715827883); // L8
    check_eq(I32_MIN, I32_MIN + 1, 2); // L8

    let mut rng = Pcg32::new(0xC001_000b);
    let mut cmp = Cmp::new("row11 tail r<0, v2<0");
    for _ in 0..N {
        let v2 = rng.i32_in(I32_MIN + 1, -2);
        let v1 = rng.i32_in(I32_MIN + 1, -1);
        cmp.feed(v1, v2);
    }
    for v2 in -500..=-2i32 {
        for v1 in -500..=-1i32 {
            cmp.feed(v1, v2);
        }
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// Row 12 — v1 == 0 with any v2 != 0 -> `return 0`
// ---------------------------------------------------------------------------

#[test]
fn err_row12_v1_zero() {
    for &v2 in BOUNDARIES {
        check_eq(0, v2, 0);
    }
    let mut rng = Pcg32::new(0xC001_000c);
    let mut cmp = Cmp::new("row12 v1 == 0");
    for _ in 0..N {
        let v2 = rng.i32_any();
        assert_eq!(check(0, v2), 0, "div_euclid(0, {v2}) must be 0");
        cmp.feed(0, v2);
    }
    cmp.finish(N as u64);
}

// ---------------------------------------------------------------------------
// G1..G7 — generic FFI-boundary classes (see ERRORS.md)
// ---------------------------------------------------------------------------

#[test]
fn boundary_g1_g7_generic_ffi_edges() {
    // G1: no pointer parameter exists (`int div_euclid(int, int)`), so there is
    //     no null-pointer case; the loaded signature proves the shape.
    let (c, r) = common::funcs();
    let _: unsafe extern "C" fn(i32, i32) -> i32 = c;
    let _: unsafe extern "C" fn(i32, i32) -> i32 = r;

    // G2: "zero length / empty" == the scalar zeros, for both arguments.
    check_eq(0, 0, 0);
    check_eq(0, 1, 0);
    check_eq(1, 0, 0);

    // G3/G4: the extremes and one step either side of every range the C tests
    //        (`x >= 0` and `x != INT_MIN`), for both arguments.
    let edges = [
        I32_MIN,
        I32_MIN + 1,
        I32_MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        I32_MAX - 2,
        I32_MAX - 1,
        I32_MAX,
    ];
    for &v1 in &edges {
        for &v2 in &edges {
            check(v1, v2);
        }
    }

    // G5: an `int` bit pattern with no matching case is impossible because the
    //     C ladder is total over `int`. Probe "enum-like" out-of-range ints,
    //     i.e. values far outside any plausible small-enum domain, plus raw
    //     bit patterns reinterpreted from u32.
    for &raw in &[
        0x8000_0000u32,
        0x7fff_ffff,
        0xffff_ffff,
        0x0000_0000,
        0xdead_beef,
        0xcafe_babe,
        0x5555_5555,
        0xaaaa_aaaa,
        0x0000_00ff,
        0xffff_ff00,
        99,
        100,
        255,
        256,
        65535,
        65536,
    ] {
        let v = raw as i32;
        check(v, v);
        check(v, 1);
        check(1, v);
        check(v, -1);
        check(-1, v);
        check(v, 0);
        check(0, v);
        check(v, I32_MIN);
        check(I32_MIN, v);
        check(v, I32_MAX);
        check(I32_MAX, v);
    }

    // G6: the only overflow-capable expressions.
    check_eq(I32_MIN, -1, I32_MIN);
    check_eq(I32_MIN, 1, I32_MIN);
    check_eq(I32_MIN, I32_MIN, 1);

    // G7: none of the above crashed, confirming no SIGFPE path is reachable.
}
