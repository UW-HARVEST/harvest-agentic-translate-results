//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`, plus the generic FFI boundary coverage.
//!
//! Each test asserts (a) that C and Rust return the *same* value and (b) where
//! the C source pins an exact sentinel, that the shared value *is* that
//! sentinel — not merely "both failed somehow".

mod common;

use common::{diff, lib, samples, Checker, Rng, BOUNDARY_GRID, INT_MAX, INT_MIN, SEED};

// ===========================================================================
// E1..E4 — the only explicit rejection: `if (v2 == 0) return 0;` (lib.c:4-6)
// ===========================================================================

#[test]
fn e1_v2_zero_v1_zero() {
    // Exact sentinel: 0
    assert_eq!(diff(0, 0).expect("C/Rust diverge on (0,0)"), 0);
}

#[test]
fn e2_v2_zero_v1_positive() {
    let mut ck = Checker::new("E2 v2==0, v1>0");
    for v1 in [1, 2, 3, 1000, 1 << 30, INT_MAX - 1, INT_MAX] {
        ck.check_eq(v1, 0, 0);
    }
    ck.finish();
}

#[test]
fn e3_v2_zero_v1_negative() {
    let mut ck = Checker::new("E3 v2==0, v1<0");
    for v1 in [-1, -2, -3, -1000, -(1 << 30), INT_MIN + 1, INT_MIN] {
        ck.check_eq(v1, 0, 0);
    }
    ck.finish();
}

#[test]
fn e4_v2_zero_random_v1() {
    let mut ck = Checker::new("E4 v2==0, random v1");
    let mut r = Rng::new(SEED ^ 0xE4);
    for _ in 0..(samples() * 5) {
        ck.check_eq(r.next_i32(), 0, 0);
        ck.check_eq(r.nasty_i32(), 0, 0);
    }
    ck.finish();
}

// ===========================================================================
// E5 — lib.c:11 guard `v2 != INT_MIN` with v1 >= 0  =>  q=0, r=v1 (r>=0) => 0
// ===========================================================================

#[test]
fn e5_int_min_v2_nonneg_v1() {
    let mut ck = Checker::new("E5 v1>=0, v2==INT_MIN");
    let mut r = Rng::new(SEED ^ 0xE5);
    for v1 in [0, 1, 2, 12345, 1 << 30, INT_MAX - 1, INT_MAX] {
        ck.check_eq(v1, INT_MIN, 0);
    }
    for _ in 0..samples() {
        ck.check_eq(r.range_i32(0, INT_MAX), INT_MIN, 0);
    }
    ck.finish();
}

// ===========================================================================
// E6 — lib.c:15 guard `v1 != INT_MIN`: v1 == INT_MIN must never be negated.
//      Assert C and Rust agree for v1 == INT_MIN against every kind of v2.
// ===========================================================================

#[test]
fn e6_int_min_v1_guard() {
    let mut ck = Checker::new("E6 v1==INT_MIN vs all v2 kinds");
    let mut r = Rng::new(SEED ^ 0xE6);
    for v2 in BOUNDARY_GRID {
        ck.check(INT_MIN, v2);
    }
    for _ in 0..(samples() * 5) {
        ck.check(INT_MIN, r.next_i32());
        ck.check(INT_MIN, r.nasty_i32());
    }
    ck.finish();
}

// ===========================================================================
// E7 — lib.c:18 guard: v1 < 0 (!= INT_MIN), v2 == INT_MIN
//      => q=1, r = v1 - 1*INT_MIN >= 1 > 0 => returns 1
// ===========================================================================

#[test]
fn e7_int_min_v2_negative_v1() {
    let mut ck = Checker::new("E7 v1<0 (!=INT_MIN), v2==INT_MIN");
    let mut r = Rng::new(SEED ^ 0xE7);
    for v1 in [-1, -2, -3, -(1 << 30), INT_MIN + 2, INT_MIN + 1] {
        ck.check_eq(v1, INT_MIN, 1);
    }
    for _ in 0..samples() {
        ck.check_eq(r.range_i32(INT_MIN + 1, -1), INT_MIN, 1);
    }
    ck.finish();
}

// ===========================================================================
// E8 — lib.c:24 guard: v1 == INT_MIN && v2 == INT_MIN => q=1, r=0 => returns 1
// ===========================================================================

#[test]
fn e8_int_min_both() {
    let mut ck = Checker::new("E8 both INT_MIN");
    ck.check_eq(INT_MIN, INT_MIN, 1);
    ck.finish();
}

// ===========================================================================
// E9 — lib.c:23: v1 == INT_MIN, v2 > 0 (the -(v1+v2) re-association)
// ===========================================================================

#[test]
fn e9_int_min_v1_positive_v2() {
    let mut ck = Checker::new("E9 v1==INT_MIN, v2>0");
    let mut r = Rng::new(SEED ^ 0xE9);
    for v2 in 1..=8192 {
        ck.check(INT_MIN, v2);
    }
    for v2 in (INT_MAX - 8192)..=INT_MAX {
        ck.check(INT_MIN, v2);
    }
    for k in 0..31 {
        ck.check(INT_MIN, 1 << k);
        ck.check(INT_MIN, (1i32 << k).saturating_add(1));
        ck.check(INT_MIN, (1i32 << k) - 1);
    }
    for _ in 0..(samples() * 5) {
        ck.check(INT_MIN, r.range_i32(1, INT_MAX));
    }
    ck.finish();
}

// ===========================================================================
// E10 — lib.c:25: v1 == INT_MIN, v2 < 0 && != INT_MIN (the -(v1-v2) form)
// ===========================================================================

#[test]
fn e10_int_min_v1_negative_v2() {
    let mut ck = Checker::new("E10 v1==INT_MIN, v2<0 (!=INT_MIN)");
    let mut r = Rng::new(SEED ^ 0xEA);
    for v2 in -8192..=-1 {
        ck.check(INT_MIN, v2);
    }
    for v2 in (INT_MIN + 1)..=(INT_MIN + 8192) {
        ck.check(INT_MIN, v2);
    }
    for k in 0..31 {
        ck.check(INT_MIN, -(1i32 << k));
        ck.check(INT_MIN, -((1i32 << k) - 1).max(INT_MIN + 1));
    }
    for _ in 0..(samples() * 5) {
        ck.check(INT_MIN, r.range_i32(INT_MIN + 1, -1));
    }
    ck.finish();
}

// ===========================================================================
// E11 — INT_MIN / -1: the classic trapping input. Must not trap; must agree.
// ===========================================================================

#[test]
fn e11_int_min_over_minus_one() {
    let c_and_rust = diff(INT_MIN, -1).expect("C/Rust diverge on (INT_MIN,-1)");
    // Reference-independent: just record the shared value so a regression that
    // changes it shows up in the log too.
    println!("div_euclid(INT_MIN, -1) = {c_and_rust}");
    // The C reaches lib.c:25 with v2 = -1: t = -(INT_MIN - (-1)) = INT_MAX,
    // q = INT_MAX/1 + 1 = INT_MIN (wrapped), r = -(INT_MAX % 1) = 0 => returns q.
    assert_eq!(c_and_rust, INT_MIN);
}

#[test]
fn e12_int_min_over_one() {
    let v = diff(INT_MIN, 1).expect("C/Rust diverge on (INT_MIN,1)");
    println!("div_euclid(INT_MIN, 1) = {v}");
    // lib.c:23 with v2 = 1: t = -(INT_MIN+1) = INT_MAX, q = -(INT_MAX/1) - 1 = INT_MIN,
    // r = -(INT_MAX % 1) = 0 => returns INT_MIN.
    assert_eq!(v, INT_MIN);
}

// ===========================================================================
// E13/E14 — the negative-r epilogue at lib.c:31 and its sign-dependent
//           adjustment (`v2 > 0 ? -1 : +1`).
// ===========================================================================

#[test]
fn e13_epilogue_negative_r_positive_v2() {
    // P4 with a non-zero remainder is the canonical r<0 && v2>0 case.
    let mut ck = Checker::new("E13 epilogue r<0, v2>0");
    let mut r = Rng::new(SEED ^ 0x13);
    // hand-picked: -7 / 3 -> q = -2, r = -1 -> q-1 = -3
    ck.check_eq(-7, 3, -3);
    ck.check_eq(-1, 2, -1);
    ck.check_eq(-1, INT_MAX, -1);
    for _ in 0..samples() {
        let v2 = r.range_i32(2, 1 << 29);
        let rem = r.range_i32(1, v2 - 1);
        let k = r.range(0, (INT_MAX as i64 - rem as i64) / v2 as i64);
        let mag = k * v2 as i64 + rem as i64;
        if mag >= 1 && mag <= INT_MAX as i64 {
            let v1 = -(mag as i32);
            // expected: floor division
            let expected = (v1 as i64).div_euclid(v2 as i64) as i32;
            ck.check_eq(v1, v2, expected);
        }
    }
    ck.finish();
}

#[test]
fn e14_epilogue_negative_r_negative_v2() {
    // P5 with a non-zero remainder is the canonical r<0 && v2<0 case.
    let mut ck = Checker::new("E14 epilogue r<0, v2<0");
    let mut r = Rng::new(SEED ^ 0x14);
    // -7 / -3 -> q = 2, r = -1 -> q+1 = 3
    ck.check_eq(-7, -3, 3);
    ck.check_eq(-1, -2, 1);
    for _ in 0..samples() {
        let m = r.range_i32(2, 1 << 29);
        let rem = r.range_i32(1, m - 1);
        let k = r.range(0, (INT_MAX as i64 - rem as i64) / m as i64);
        let mag = k * m as i64 + rem as i64;
        if mag >= 1 && mag <= INT_MAX as i64 {
            let v1 = -(mag as i32);
            let v2 = -m;
            // Euclidean division rounds the quotient towards +inf for v2<0.
            let expected = -((v1 as i64).div_euclid(m as i64)) as i32;
            ck.check_eq(v1, v2, expected);
        }
    }
    ck.finish();
}

// ===========================================================================
// E15 — the `q + adjustment` addition at lib.c:31 can itself overflow.
//       Search the extreme grid for q at the representable limits.
// ===========================================================================

#[test]
fn e15_epilogue_adjust_overflow() {
    let mut ck = Checker::new("E15 epilogue adjustment overflow");
    // q == INT_MIN with v2 > 0 (needs q-1 -> wraps to INT_MAX)
    // q == INT_MAX with v2 < 0 (needs q+1 -> wraps to INT_MIN)
    // Reachable candidates all live at |v1| or |v2| == 1 / INT_MIN / INT_MAX.
    let extremes = [
        INT_MIN,
        INT_MIN + 1,
        INT_MIN + 2,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        INT_MAX - 2,
        INT_MAX - 1,
        INT_MAX,
    ];
    for a in extremes {
        for b in extremes {
            ck.check(a, b);
        }
    }
    // v2 == -1 over the whole negative half sweeps q = -v1 up to INT_MAX.
    let mut r = Rng::new(SEED ^ 0x15);
    for _ in 0..samples() {
        ck.check(r.next_i32(), -1);
        ck.check(r.next_i32(), 1);
        ck.check(-1, r.next_i32());
        ck.check(1, r.next_i32());
    }
    ck.finish();
}

// ===========================================================================
// Generic boundary coverage required for every C API
// ===========================================================================

#[test]
fn generic_boundaries() {
    let mut ck = Checker::new("generic boundaries");

    // Full cross product of the 9-value extreme grid.
    for a in BOUNDARY_GRID {
        for b in BOUNDARY_GRID {
            ck.check(a, b);
        }
    }

    // "One step past" the only rejected value (v2 == 0) on both sides.
    for a in BOUNDARY_GRID {
        ck.check(a, -1);
        ck.check(a, 0);
        ck.check(a, 1);
    }

    // Every representable value of one operand, in the immediate neighbourhood
    // of each extreme, against every extreme of the other.
    for &pin in &BOUNDARY_GRID {
        for d in -64i64..=64 {
            for &base in &[INT_MIN as i64, -1, 0, 1, INT_MAX as i64] {
                let v = base.saturating_add(d).clamp(INT_MIN as i64, INT_MAX as i64) as i32;
                ck.check(v, pin);
                ck.check(pin, v);
            }
        }
    }
    ck.finish();
}

/// `div_euclid`'s signature is `int (int, int)` — no pointers, no lengths, no
/// enums. Documented here so the absence of null-pointer / length tests is a
/// derived fact rather than an omission.
#[test]
fn no_pointer_arguments_in_api() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    assert!(
        !header.contains('*'),
        "the public header gained a pointer parameter; null-pointer differential \
         tests must be added: {header}"
    );
    assert!(
        header.contains("int div_euclid(int v1, int v2);"),
        "unexpected public header contents: {header}"
    );
}

/// Out-of-range "enum-like" ints: any 32-bit value is a legal `c_int` argument.
/// Every operand value is a valid input, so this asserts agreement on a dense
/// random sample drawn with no regard for validity.
#[test]
fn out_of_range_integer_arguments() {
    let mut ck = Checker::new("arbitrary/out-of-range int arguments");
    let mut r = Rng::new(SEED ^ 0xDEAD);
    for _ in 0..(samples() * 25) {
        ck.check(r.next_i32(), r.next_i32());
    }
    // and the u32-reinterpreted extremes
    for &u in &[
        0u32,
        1,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
        0xffff_fffe,
        0xdead_beef,
        0xcafe_babe,
    ] {
        for &w in &[
            0u32,
            1,
            0x7fff_ffff,
            0x8000_0000,
            0x8000_0001,
            0xffff_ffff,
            0xdead_beef,
        ] {
            ck.check(u as i32, w as i32);
        }
    }
    ck.finish();
}

#[test]
fn shared_objects_are_distinct_files() {
    let l = lib();
    assert!(l.c_path.exists());
    assert!(l.rust_path.exists());
    assert_ne!(
        std::fs::canonicalize(&l.c_path).unwrap(),
        std::fs::canonicalize(&l.rust_path).unwrap()
    );
}

// ===========================================================================
// Independent oracle. Established by mutation testing (see PHASES.md): the C
// function is *exactly* wrapping Euclidean division, with `0` for `v2 == 0`.
// Asserting C, Rust AND this third independent oracle all agree makes the
// differential suite non-vacuous even where two implementations share a bug.
// ===========================================================================

#[inline]
fn oracle(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        0
    } else {
        v1.wrapping_div_euclid(v2)
    }
}

#[test]
fn independent_oracle_agrees_with_both() {
    let l = lib();
    let mut bad: Vec<String> = Vec::new();
    let mut n = 0u64;
    let mut probe = |v1: i32, v2: i32| {
        let c = l.c(v1, v2);
        let r = l.rust(v1, v2);
        let o = oracle(v1, v2);
        n += 1;
        if !(c == r && r == o) && bad.len() < 25 {
            bad.push(format!("({v1},{v2}): C={c} Rust={r} oracle={o}"));
        }
    };
    for v1 in -400..=400 {
        for v2 in -400..=400 {
            probe(v1, v2);
        }
    }
    for &p in &[
        INT_MIN,
        INT_MIN + 1,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        INT_MAX - 1,
        INT_MAX,
    ] {
        let mut u: u64 = 0;
        while u <= u32::MAX as u64 {
            let v = (u as u32).wrapping_add(0x8000_0000) as i32;
            probe(p, v);
            probe(v, p);
            u += 4093;
        }
    }
    assert!(
        bad.is_empty(),
        "{} three-way mismatch(es) out of {} probes:\n{}",
        bad.len(),
        n,
        bad.join("\n")
    );
    println!("[oracle] OK - {n} probes: C == Rust == wrapping Euclidean division");
}
