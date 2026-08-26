//! Phase B — valid-path differential tests for the public entry point.
//!
//! Rows 1–15 of `CONFIGS.md`.  Both implementations are loaded from their
//! `.so`s and called through `dlsym`, so the exported `memchra2` wrapper is
//! itself under test.

mod common;

use std::ffi::c_int;

use common::{c_lib, rust_lib, sym, Rng, SEED};

type Memchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn funcs() -> (Memchra2, Memchra2) {
    unsafe {
        (
            sym::<Memchra2>(c_lib(), "memchra2"),
            sym::<Memchra2>(rust_lib(), "memchra2"),
        )
    }
}

#[track_caller]
fn check(label: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let (fc, fr) = funcs();
    let got_c = unsafe { fc(a, b, c, d) };
    let got_r = unsafe { fr(a, b, c, d) };
    assert_eq!(
        got_c, got_r,
        "{label}: memchra2({a}, {b}, {c}, {d}) -> C={got_c} Rust={got_r} \
         (a bits = {:#010x})",
        a as u32
    );
}

/// Row 1 — all args non-negative single digit (3 dashes, subnormal float).
#[test]
fn cfg01_all_positive_small() {
    for a in 0..10 {
        for b in 0..10 {
            for c in 0..10 {
                for d in 0..10 {
                    check("cfg01", a, b, c, d);
                }
            }
        }
    }
}

/// Row 2 — all args negative (7 dashes in the formatted buffer).
#[test]
fn cfg02_all_negative() {
    let mut rng = Rng::new(SEED ^ 2);
    for a in -10..0 {
        for b in -10..0 {
            check("cfg02-small", a, b, -1, -9);
        }
    }
    for _ in 0..5000 {
        let a = -(rng.next_u32() as i64 % 2_147_483_648i64) as i32 - 1;
        let b = -((rng.next_u32() % 1_000_000) as i32) - 1;
        let c = -((rng.next_u32() % 1_000) as i32) - 1;
        let d = -((rng.next_u32() % 100_000_000) as i32) - 1;
        check("cfg02-rand", a, b, c, d);
    }
}

/// Row 3 — every one of the 16 sign patterns of (a, b, c, d).
#[test]
fn cfg03_sign_patterns() {
    let mut rng = Rng::new(SEED ^ 3);
    for pattern in 0..16u32 {
        for _ in 0..500 {
            let mut v = [0i32; 4];
            for (i, slot) in v.iter_mut().enumerate() {
                // magnitude in [1, 2^31-1] so the sign is unambiguous
                let mag = (rng.next_u32() % 0x7FFF_FFFF) as i32 + 1;
                *slot = if pattern & (1 << i) != 0 { -mag } else { mag };
            }
            check("cfg03", v[0], v[1], v[2], v[3]);
        }
    }
}

/// Row 4 — `a == 0` → `f == 0.0f`, so the float branch is skipped.
#[test]
fn cfg04_a_zero() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..2000 {
        check(
            "cfg04",
            0,
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
}

/// Row 5 — `a ∈ [1, 0x3F7FFFFF]` → `0 < f < 1` → `(int)f == 0`.
#[test]
fn cfg05_a_float_lt_one() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..4000 {
        let a = (1 + rng.next_u32() % 0x3F7F_FFFF) as i32;
        check(
            "cfg05",
            a,
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
    // exact boundaries
    check("cfg05-min", 1, 1, 1, 1);
    check("cfg05-max", 0x3F7F_FFFF, 1, 2, 3);
}

/// Row 6 — `a ∈ [0x3F800000, 0x4479FFFF]` → `1 <= f < 1000` → non-zero
/// `(int)f` contribution.
#[test]
fn cfg06_a_float_in_range() {
    let mut rng = Rng::new(SEED ^ 6);
    let lo = 0x3F80_0000u32;
    let hi = 0x4479_FFFFu32;
    for _ in 0..6000 {
        let a = (lo + rng.next_u32() % (hi - lo + 1)) as i32;
        check(
            "cfg06",
            a,
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
    for a in [lo, lo + 1, hi - 1, hi, 0x4479_FFFE] {
        check("cfg06-edge", a as i32, -7, 0, 255);
    }
}

/// Row 7 — `a ∈ [0x447A0000, 0x7F7FFFFF]` → `f >= 1000` → branch skipped.
#[test]
fn cfg07_a_float_ge_1000() {
    let mut rng = Rng::new(SEED ^ 7);
    let lo = 0x447A_0000u32;
    let hi = 0x7F7F_FFFFu32;
    for _ in 0..4000 {
        let a = (lo + rng.next_u32() % (hi - lo + 1)) as i32;
        check(
            "cfg07",
            a,
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
    for a in [lo, lo + 1, hi, 0x447A_0001] {
        check("cfg07-edge", a as i32, 3, -4, 5);
    }
}

/// Row 8 — `a` is a float special: ±inf, quiet/signalling NaN.
#[test]
fn cfg08_a_float_special() {
    let specials: [u32; 8] = [
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // quiet NaN
        0x7F80_0001, // signalling NaN
        0xFFC0_0000, // negative NaN
        0x8000_0000, // -0.0
        0x0000_0001, // smallest subnormal
        0x8000_0001, // negative subnormal
    ];
    let mut rng = Rng::new(SEED ^ 8);
    for &bits in &specials {
        for _ in 0..200 {
            check(
                "cfg08",
                bits as i32,
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
                rng.next_i32_interesting(),
            );
        }
    }
}

/// Row 9 — low byte of b, c, d is zero (multiples of 256).
#[test]
fn cfg09_low_bytes_zero() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..3000 {
        let b = (rng.next_i32() / 256) * 256;
        let c = (rng.next_i32() / 256) * 256;
        let d = (rng.next_i32() / 256) * 256;
        check("cfg09", rng.next_i32_interesting(), b, c, d);
    }
    check("cfg09-zero", 0, 0, 0, 0);
    check("cfg09-256", 256, 256, -256, 65536);
}

/// Row 10 — low byte of b, c, d is 0xFF → `interpreted == 0x00FFFFFF`.
#[test]
fn cfg10_low_bytes_ff() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..3000 {
        let mk = |r: &mut Rng| ((r.next_i32() as u32 & 0xFFFF_FF00) | 0xFF) as i32;
        let b = mk(&mut rng);
        let c = mk(&mut rng);
        let d = mk(&mut rng);
        check("cfg10", rng.next_i32_interesting(), b, c, d);
    }
    check("cfg10-neg1", 0, -1, -1, -1);
    check("cfg10-255", 1, 255, 255, 255);
}

/// Row 11 — decimal-width sweep (1…10 digits per argument).
#[test]
fn cfg11_digit_widths() {
    let mut rng = Rng::new(SEED ^ 11);
    let widths: [i64; 10] = [
        1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000,
    ];
    for &wa in &widths {
        for &wb in &widths {
            for &wc in &widths {
                for &wd in &widths {
                    let pick = |base: i64, r: &mut Rng| -> i32 {
                        let span = base.max(1);
                        let v = base + (r.next_u32() as i64 % span);
                        let v = v.min(i32::MAX as i64) as i32;
                        if r.next_u64() & 1 == 0 {
                            v
                        } else {
                            -v
                        }
                    };
                    let a = pick(wa, &mut rng);
                    let b = pick(wb, &mut rng);
                    let c = pick(wc, &mut rng);
                    let d = pick(wd, &mut rng);
                    check("cfg11", a, b, c, d);
                }
            }
        }
    }
}

/// Row 12 — boundary matrix over {INT_MIN, INT_MAX, 0, -1, 1} (625 combos).
#[test]
fn cfg12_boundary_matrix() {
    let vals: [i32; 5] = [i32::MIN, i32::MAX, 0, -1, 1];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    check("cfg12", a, b, c, d);
                }
            }
        }
    }
}

/// Row 13 — `a+b+c+d` wraps around (signed overflow in `safe_sum_array`).
#[test]
fn cfg13_sum_overflow() {
    let cases: [[i32; 4]; 8] = [
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, 1, 0, 0],
        [i32::MIN, -1, 0, 0],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [2_000_000_000, 2_000_000_000, 1, -1],
        [-2_000_000_000, -2_000_000_000, -1, 1],
        [i32::MAX / 2, i32::MAX / 2, i32::MAX / 2, i32::MAX / 2],
    ];
    for case in &cases {
        check("cfg13", case[0], case[1], case[2], case[3]);
    }
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..3000 {
        let a = 2_000_000_000 - (rng.next_u32() % 1000) as i32;
        let b = 2_000_000_000 - (rng.next_u32() % 1000) as i32;
        let c = rng.next_i32();
        let d = rng.next_i32();
        check("cfg13-rand", a, b, c, d);
    }
}

/// Row 14 — full-range random fuzz.
#[test]
fn cfg14_random_full_range() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..20_000 {
        check(
            "cfg14",
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
    // and the "interesting value" distribution
    let mut rng = Rng::new(SEED ^ 0xF14);
    for _ in 0..20_000 {
        check(
            "cfg14-interesting",
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
            rng.next_i32_interesting(),
        );
    }
}

/// Row 15 — repeated / identical arguments.
#[test]
fn cfg15_repeated_args() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..3000 {
        let v = rng.next_i32_interesting();
        check("cfg15-all-same", v, v, v, v);
        let n = v.wrapping_neg();
        check("cfg15-pairs", v, v, n, n);
    }
    for v in [0, 1, -1, 7, 11, 111_111_111, -111_111_111, i32::MIN, i32::MAX] {
        check("cfg15-fixed", v, v, v, v);
    }
}
