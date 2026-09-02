//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//! Rows C1..C15 (scalar + `init_result_array` entry points).

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 5000;

// ---------------------------------------------------------------------------
// C1..C3 — the arithmetic operations
// ---------------------------------------------------------------------------

fn op_row(row: &str, pick: fn(&Lib) -> OperationFunc) {
    let (c, r) = both();
    let cf = pick(c);
    let rf = pick(r);
    let mut rng = Rng::seeded();

    // Boundary cross-product first.
    for a in boundary_i32() {
        for b in boundary_i32() {
            for u1 in [0, -1, i32::MAX] {
                for u2 in [0, 1, i32::MIN] {
                    eq_int(
                        &format!("{row} a={a} b={b} u1={u1} u2={u2}"),
                        cf(a, b, u1, u2),
                        rf(a, b, u1, u2),
                    );
                }
            }
        }
    }
    // Randomized bulk.
    for i in 0..N {
        let a = rng.next_i32_spicy();
        let b = rng.next_i32_spicy();
        let u1 = rng.next_i32();
        let u2 = rng.next_i32();
        eq_int(
            &format!("{row} #{i} a={a} b={b}"),
            cf(a, b, u1, u2),
            rf(a, b, u1, u2),
        );
    }
}

#[test]
fn c1_add_operation() {
    op_row("C1 add", |l| l.add_operation);
}

#[test]
fn c2_multiply_operation() {
    op_row("C2 mul", |l| l.multiply_operation);
}

#[test]
fn c3_subtract_operation() {
    op_row("C3 sub", |l| l.subtract_operation);
}

// ---------------------------------------------------------------------------
// C4 — modulo, b != 0, excluding the trapping INT_MIN % -1 (see ERRORS E2)
// ---------------------------------------------------------------------------

#[test]
fn c4_modulo_operation_nonzero_divisor() {
    let (c, r) = both();
    let mut rng = Rng::seeded();

    let interesting: Vec<c_int> = vec![
        1, -1, 2, -2, 3, -3, 7, -7, 10, -10, 100, -100,
        i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];
    for &a in interesting.iter() {
        for &b in interesting.iter() {
            if b == 0 || (a == i32::MIN && b == -1) {
                continue;
            }
            eq_int(
                &format!("C4 a={a} b={b}"),
                (c.modulo_operation)(a, b, 0, 0),
                (r.modulo_operation)(a, b, 0, 0),
            );
        }
    }

    let mut done = 0;
    while done < N {
        let a = rng.next_i32_spicy();
        let mut b = rng.next_i32_spicy();
        if b == 0 {
            b = 1;
        }
        if a == i32::MIN && b == -1 {
            continue;
        }
        eq_int(
            &format!("C4 rnd a={a} b={b}"),
            (c.modulo_operation)(a, b, 0, 0),
            (r.modulo_operation)(a, b, 0, 0),
        );
        done += 1;
    }
}

// ---------------------------------------------------------------------------
// C5 — modulo, b == 0 early-return branch
// ---------------------------------------------------------------------------

#[test]
fn c5_modulo_operation_zero_divisor_branch() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for a in boundary_i32() {
        eq_int(
            &format!("C5 a={a}"),
            (c.modulo_operation)(a, 0, 0, 0),
            (r.modulo_operation)(a, 0, 0, 0),
        );
    }
    for _ in 0..N {
        let a = rng.next_i32_spicy();
        let (u1, u2) = (rng.next_i32(), rng.next_i32());
        eq_int(
            &format!("C5 rnd a={a}"),
            (c.modulo_operation)(a, 0, u1, u2),
            (r.modulo_operation)(a, 0, u1, u2),
        );
    }
}

// ---------------------------------------------------------------------------
// C6..C10 — safe_double_to_int, all four branches
// ---------------------------------------------------------------------------

fn sdti_row(row: &str, gen: &mut dyn FnMut(&mut Rng) -> f64, extra: &[f64]) {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for &d in extra {
        eq_int(
            &format!("{row} d={d} bits={:#018x}", d.to_bits()),
            (c.safe_double_to_int)(d),
            (r.safe_double_to_int)(d),
        );
    }
    for i in 0..N {
        let d = gen(&mut rng);
        eq_int(
            &format!("{row} #{i} d={d} bits={:#018x}", d.to_bits()),
            (c.safe_double_to_int)(d),
            (r.safe_double_to_int)(d),
        );
    }
}

#[test]
fn c6_sdti_truncation_branch() {
    sdti_row(
        "C6",
        &mut |rng| rng.f64_in_int_range(),
        &[
            0.0, -0.0, 0.5, -0.5, 0.9999999, -0.9999999, 1.5, -1.5, -2.5, 2.5,
            1.0, -1.0, 42.7, -42.7, 2147483646.999, -2147483647.999,
            5e-324, -5e-324, f64::MIN_POSITIVE, -f64::MIN_POSITIVE,
        ],
    );
}

#[test]
fn c7_sdti_upper_clamp_branch() {
    sdti_row(
        "C7",
        &mut |rng| {
            let e = (rng.below(300)) as i32;
            INT_MAX_D * (1.0 + (rng.below(1000) as f64)) * 10f64.powi(e / 10)
        },
        &[
            INT_MAX_D,
            2147483647.5,
            2147483648.0,
            4e9,
            1e300,
            f64::INFINITY,
            f64::MAX,
        ],
    );
}

#[test]
fn c8_sdti_lower_clamp_branch() {
    sdti_row(
        "C8",
        &mut |rng| {
            let e = (rng.below(300)) as i32;
            INT_MIN_D * (1.0 + (rng.below(1000) as f64)) * 10f64.powi(e / 10)
        },
        &[
            INT_MIN_D,
            -2147483648.5,
            -2147483649.0,
            -4e9,
            -1e300,
            f64::NEG_INFINITY,
            f64::MIN,
        ],
    );
}

#[test]
fn c9_sdti_nan_branch() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let mut nans: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::INFINITY * 0.0,
        f64::INFINITY - f64::INFINITY,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_DEAD_BEEF_1234),
    ];
    for _ in 0..N {
        // random payload NaN, random sign
        let sign = (rng.next_u64() & 1) << 63;
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let bits = sign | 0x7FF0_0000_0000_0000 | payload.max(1);
        nans.push(f64::from_bits(bits));
    }
    for (i, &d) in nans.iter().enumerate() {
        assert!(d.is_nan(), "generator produced a non-NaN at #{i}");
        eq_int(
            &format!("C9 #{i} bits={:#018x}", d.to_bits()),
            (c.safe_double_to_int)(d),
            (r.safe_double_to_int)(d),
        );
    }
}

#[test]
fn c10_sdti_random_bit_patterns() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for i in 0..(N * 8) {
        let d = rng.f64_bits();
        eq_int(
            &format!("C10 #{i} bits={:#018x}", d.to_bits()),
            (c.safe_double_to_int)(d),
            (r.safe_double_to_int)(d),
        );
    }
    // Also sweep every exponent with a fixed mantissa, both signs.
    for sign in [0u64, 1u64] {
        for exp in 0u64..=0x7FF {
            for mant in [0u64, 1, 0x8_0000_0000_0000, 0xF_FFFF_FFFF_FFFF] {
                let bits = (sign << 63) | (exp << 52) | mant;
                let d = f64::from_bits(bits);
                eq_int(
                    &format!("C10 sweep bits={bits:#018x}"),
                    (c.safe_double_to_int)(d),
                    (r.safe_double_to_int)(d),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11..C12 — compute_scaled_value
// ---------------------------------------------------------------------------

#[test]
fn c11_compute_scaled_value_in_range() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for base in boundary_i32() {
        for s in [0.0, 1.0, -1.0, 0.5, -0.5, 1.5, 0.333, 0.75, 0.8] {
            eq_int(
                &format!("C11 base={base} s={s}"),
                (c.compute_scaled_value)(base, s),
                (r.compute_scaled_value)(base, s),
            );
        }
    }
    for i in 0..N {
        let base = rng.next_i32_spicy();
        let s = rng.f64_in_int_range() / 2147483647.0 * 4.0; // ~[-4,4]
        eq_int(
            &format!("C11 #{i} base={base} s={s}"),
            (c.compute_scaled_value)(base, s),
            (r.compute_scaled_value)(base, s),
        );
    }
}

#[test]
fn c12_compute_scaled_value_extreme_scales() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let scales = extreme_scales();
    for base in boundary_i32() {
        for &s in scales.iter() {
            eq_int(
                &format!("C12 base={base} s_bits={:#018x}", s.to_bits()),
                (c.compute_scaled_value)(base, s),
                (r.compute_scaled_value)(base, s),
            );
        }
    }
    for i in 0..N {
        let base = rng.next_i32_spicy();
        let s = scales[(rng.below(scales.len() as u64)) as usize];
        eq_int(
            &format!("C12 #{i} base={base} s_bits={:#018x}", s.to_bits()),
            (c.compute_scaled_value)(base, s),
            (r.compute_scaled_value)(base, s),
        );
    }
    // Fully random bit-pattern scale factors.
    for i in 0..N {
        let base = rng.next_i32_spicy();
        let s = rng.f64_bits();
        eq_int(
            &format!("C12 rnd #{i} base={base} s_bits={:#018x}", s.to_bits()),
            (c.compute_scaled_value)(base, s),
            (r.compute_scaled_value)(base, s),
        );
    }
}

// ---------------------------------------------------------------------------
// C13..C15 — init_result_array
// ---------------------------------------------------------------------------

fn init_both(values: &[c_int], count: c_int) -> (ResultArray, ResultArray) {
    let (c, r) = both();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    let mut cv: Vec<c_int> = values.to_vec();
    let mut rv: Vec<c_int> = values.to_vec();
    (c.init_result_array)(&mut ca, cv.as_mut_ptr(), count);
    (r.init_result_array)(&mut ra, rv.as_mut_ptr(), count);
    assert_eq!(cv, rv, "init_result_array must not modify `values`");
    (ca, ra)
}

#[test]
fn c13_init_result_array_counts_0_to_10() {
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        for i in 0..500 {
            let values: Vec<c_int> = (0..10).map(|_| rng.next_i32_spicy()).collect();
            let (ca, ra) = init_both(&values, count);
            eq_struct(&format!("C13 count={count} #{i}"), &ca, &ra);
        }
    }
}

#[test]
fn c14_init_result_array_count_clamped_above_10() {
    let mut rng = Rng::seeded();
    for count in [11i32, 12, 17, 64, 1000, i32::MAX - 1, i32::MAX] {
        for i in 0..300 {
            let values: Vec<c_int> = (0..16).map(|_| rng.next_i32_spicy()).collect();
            let (ca, ra) = init_both(&values, count);
            assert_eq!(ca.count, 10, "C clamp broken");
            eq_struct(&format!("C14 count={count} #{i}"), &ca, &ra);
        }
    }
}

#[test]
fn c15_init_result_array_one_and_few() {
    for count in [1i32, 2] {
        for &v in boundary_i32().iter() {
            for &w in boundary_i32().iter() {
                let values = vec![v, w, 0, 0, 0, 0, 0, 0, 0, 0];
                let (ca, ra) = init_both(&values, count);
                eq_struct(&format!("C15 count={count} v={v} w={w}"), &ca, &ra);
            }
        }
    }
}
