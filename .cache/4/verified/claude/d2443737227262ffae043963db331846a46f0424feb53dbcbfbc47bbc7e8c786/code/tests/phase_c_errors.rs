//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejection/guard condition the C code checks,
//! calls BOTH `.so` exports and asserts the SAME value comes back (same
//! sentinel / same wrapped result — not merely "both failed").

mod common;

use common::{assert_same, assert_same_raw, boundary_values, libs, Rng, I32_MAX, I32_MIN};

// E1 — v2 == 0 (the only explicit rejection: `if (v2 == 0) return 0;`)
#[test]
fn e1_div_by_zero_random() {
    let mut rng = Rng::new(0xE001);
    for _ in 0..200_000 {
        let v1 = rng.next_i32();
        let got = assert_same("E1", v1, 0);
        assert_eq!(got, 0, "E1: C sentinel for v2==0 must be 0, got {got}");
    }
}

// E2 — 0 / 0
#[test]
fn e2_zero_over_zero() {
    assert_eq!(assert_same("E2", 0, 0), 0);
}

// E3 — INT_MIN / 0 : the zero rejection precedes every INT_MIN guard
#[test]
fn e3_int_min_over_zero() {
    assert_eq!(assert_same("E3", I32_MIN, 0), 0);
    assert_eq!(assert_same("E3", I32_MIN + 1, 0), 0);
}

// E4 — INT_MAX / 0
#[test]
fn e4_int_max_over_zero() {
    assert_eq!(assert_same("E4", I32_MAX, 0), 0);
    assert_eq!(assert_same("E4", I32_MAX - 1, 0), 0);
}

// E5 — v1 >= 0, v2 == INT_MIN : guard `v2 != -0x7fffffff-1` false -> q=0,r=v1
#[test]
fn e5_nonneg_over_int_min() {
    let mut rng = Rng::new(0xE005);
    for i in 0..50_000 {
        let v1 = match i {
            0 => 0,
            1 => 1,
            2 => I32_MAX,
            3 => I32_MAX - 1,
            _ => rng.range_i32(0, I32_MAX),
        };
        let got = assert_same("E5", v1, I32_MIN);
        assert_eq!(got, 0, "E5: expected 0 for div_euclid({v1}, INT_MIN)");
    }
}

// E6 — v1 < 0 (but != INT_MIN), v2 == INT_MIN -> q=1, r=v1-q*v2 > 0
#[test]
fn e6_neg_over_int_min() {
    let mut rng = Rng::new(0xE006);
    for i in 0..50_000 {
        let v1 = match i {
            0 => -1,
            1 => -2,
            2 => I32_MIN + 1,
            3 => I32_MIN + 2,
            _ => rng.range_i32(I32_MIN + 1, -1),
        };
        let got = assert_same("E6", v1, I32_MIN);
        assert_eq!(got, 1, "E6: expected 1 for div_euclid({v1}, INT_MIN)");
    }
}

// E7 — INT_MIN / INT_MIN -> q=1, r=0
#[test]
fn e7_int_min_over_int_min() {
    assert_eq!(assert_same("E7", I32_MIN, I32_MIN), 1);
}

// E8 — INT_MIN / -1 : C signed overflow (INT_MAX + 1) must wrap identically
#[test]
fn e8_int_min_over_minus_one() {
    let got = assert_same("E8", I32_MIN, -1);
    assert_eq!(
        got, I32_MIN,
        "E8: the compiled C wraps INT_MAX+1 to INT_MIN; got {got}"
    );
}

// E9 — INT_MIN / 1 : quotient at the boundary of representability
#[test]
fn e9_int_min_over_one() {
    let got = assert_same("E9", I32_MIN, 1);
    assert_eq!(got, I32_MIN, "E9: expected INT_MIN, got {got}");
}

// E10 — INT_MIN / positive : `-v1` must never be evaluated (no trap)
#[test]
fn e10_int_min_over_positive_random() {
    let mut rng = Rng::new(0xE010);
    for i in 0..200_000 {
        let v2 = match i {
            0 => 1,
            1 => 2,
            2 => I32_MAX,
            3 => I32_MAX - 1,
            _ => rng.pos_i32(I32_MAX),
        };
        assert_same("E10", I32_MIN, v2);
    }
    // ... and the negative mirror (`v2 != INT_MIN` guard true)
    let mut rng = Rng::new(0xE010_2);
    for i in 0..200_000 {
        let v2 = match i {
            0 => -1,
            1 => -2,
            2 => I32_MIN + 1,
            _ => rng.range_i32(I32_MIN + 1, -1),
        };
        assert_same("E10", I32_MIN, v2);
    }
}

// E11 — (INT_MIN+1) / -1 : (-v1)/(-v2) == INT_MAX, largest in-range result
#[test]
fn e11_int_min_plus_one_over_minus_one() {
    let got = assert_same("E11", I32_MIN + 1, -1);
    assert_eq!(got, I32_MAX, "E11: expected INT_MAX, got {got}");
}

// E12 — INT_MAX / -1
#[test]
fn e12_int_max_over_minus_one() {
    let got = assert_same("E12", I32_MAX, -1);
    assert_eq!(got, -I32_MAX, "E12: expected -2147483647, got {got}");
}

// E13 — FFI boundary: dirty upper 32 bits in the argument registers.
#[test]
fn e13_dirty_high_bits_abi() {
    let l = libs();
    let dirt: [u64; 6] = [
        0x0000_0000_0000_0000,
        0xFFFF_FFFF_0000_0000,
        0x7FFF_FFFF_0000_0000,
        0x8000_0000_0000_0000,
        0xDEAD_BEEF_0000_0000,
        0x0000_0001_0000_0000,
    ];
    let interesting = [0i32, 1, -1, 2, -2, 7, -7, 100, -100, I32_MIN, I32_MAX, I32_MIN + 1];
    for &v1 in &interesting {
        for &v2 in &interesting {
            let clean = (l.c)(v1, v2);
            for &d1 in &dirt {
                for &d2 in &dirt {
                    let a = ((v1 as u32) as u64 | d1) as i64;
                    let b = ((v2 as u32) as u64 | d2) as i64;
                    assert_same_raw("E13", a, b);
                    // both must ignore the upper halves exactly like the C does
                    let c_dirty = (l.c_raw)(a, b) as i32;
                    let r_dirty = (l.rust_raw)(a, b) as i32;
                    assert_eq!(c_dirty, clean, "E13: C changed with dirty high bits");
                    assert_eq!(r_dirty, clean, "E13: Rust changed with dirty high bits");
                }
            }
        }
    }
}

// E14 — "out-of-range enum"-equivalent: the parameters are plain `int`, so every
// 32-bit pattern is in range. Exercise the extreme patterns (0x80000000,
// 0x7fffffff, 0xffffffff, 0x00000000, single-bit and all-but-one-bit patterns)
// plus a large uniform random sample of raw patterns.
#[test]
fn e14_all_bit_patterns_boundary() {
    let mut patterns: Vec<i32> = vec![
        0x0000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0x7FFF_FFFFu32 as i32,
        0x8000_0001u32 as i32,
        0x7FFF_FFFEu32 as i32,
        0xAAAA_AAAAu32 as i32,
        0x5555_5555u32 as i32,
    ];
    for k in 0..32u32 {
        patterns.push((1u32 << k) as i32);
        patterns.push(!(1u32 << k) as i32);
    }
    patterns.extend(boundary_values());
    patterns.sort_unstable();
    patterns.dedup();
    for &v1 in &patterns {
        for &v2 in &patterns {
            assert_same("E14", v1, v2);
        }
    }
    let mut rng = Rng::new(0xE014);
    for _ in 0..500_000 {
        assert_same("E14", rng.next_u32() as i32, rng.next_u32() as i32);
    }
}

// E15 — no pointer / length parameters exist, so there is no null-pointer or
// length rejection to differentiate. Assert that structurally: the public
// header declares exactly one function taking two ints and returning an int,
// and both .so files expose that single symbol.
#[test]
fn e15_no_pointer_or_length_params() {
    let header = std::fs::read_to_string(common::manifest_dir().join("c_src/include/lib.h"))
        .expect("read lib.h");
    assert!(
        !header.contains('*'),
        "E15 assumption broken: lib.h now contains a pointer parameter:\n{header}"
    );
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .collect();
    assert_eq!(
        decls,
        vec!["int div_euclid(int v1, int v2);"],
        "E15: unexpected public API surface in lib.h"
    );
    // and the symbol really is callable with that exact signature through both
    // shared objects
    assert_same("E15", 42, 5);
}
