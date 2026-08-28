//! Differential tests: every exported function of the C `.so` is compared
//! against the same symbol of the Rust `.so`, both reached via `libloading`.
//!
//! Ordered lowest-level first (`safe_double_to_int`) up to the public entry
//! point (`fallcalc`).

mod common;

use common::{assert_int_eq, load_pair};
use std::ffi::c_int;

/// A spread of `double` bit patterns, including the boundary values that
/// `safe_double_to_int` special-cases.
fn interesting_doubles() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,  // smallest subnormal
        -5e-324,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        -0.1,
        0.1,
        // Boundaries around (double)INT_MAX / (double)INT_MIN.
        2147483647.0,
        2147483646.9999998,
        2147483647.0000002,
        2147483648.0,
        2147483646.0,
        -2147483648.0,
        -2147483647.9999998,
        -2147483648.0000005,
        -2147483649.0,
        -2147483647.0,
        // Well beyond range.
        1e18,
        -1e18,
        1e300,
        -1e300,
    ];
    for e in -30i32..=40 {
        let m = 2f64.powi(e);
        v.push(m);
        v.push(-m);
        v.push(m + 0.5);
        v.push(-m - 0.5);
        v.push(m - 1.0);
        v.push(1.0 - m);
    }
    for k in -2000i32..=2000 {
        v.push(k as f64 * 1_048_576.7);
    }
    v
}

#[test]
fn safe_double_to_int_matches() {
    let p = load_pair();
    for d in interesting_doubles() {
        let c = unsafe { (p.c.safe_double_to_int)(d) };
        let r = unsafe { (p.rs.safe_double_to_int)(d) };
        assert_int_eq(&format!("safe_double_to_int({d:?} / {:#018x})", d.to_bits()), c, r);
    }
}

/// Reads backwards from the last element, so the buffer is passed as
/// `ptr + len - 1`.
#[test]
fn process_array_reverse_matches() {
    let p = load_pair();

    let buffers: Vec<Vec<c_int>> = vec![
        vec![],
        vec![0],
        vec![7],
        vec![-1],
        vec![1, 2, 3, 4, 5],
        vec![-5, -4, -3, -2, -1],
        vec![c_int::MAX, 1, 0, -1, c_int::MIN],
        vec![c_int::MAX, c_int::MAX, c_int::MAX],
        vec![c_int::MIN, c_int::MIN, c_int::MIN],
        (0..64).collect(),
        (0..64).map(|i| i * -12345).collect(),
        (0..37).map(|i: c_int| i.wrapping_mul(0x1234_5678)).collect(),
    ];

    for buf in &buffers {
        let mut b = buf.clone();
        let len = b.len() as c_int;
        // Counts that stay within the buffer, plus the no-op counts.
        let mut counts: Vec<c_int> = vec![0, -1, -7, c_int::MIN];
        for n in 1..=len {
            counts.push(n);
        }
        for &count in &counts {
            // For count <= 0 the C never dereferences, so any pointer is fine;
            // use the buffer start to stay inside the allocation.
            let end = if b.is_empty() {
                std::ptr::null_mut()
            } else if count <= 0 {
                b.as_mut_ptr()
            } else {
                unsafe { b.as_mut_ptr().add(b.len() - 1) }
            };
            if b.is_empty() && count > 0 {
                continue;
            }
            let c = unsafe { (p.c.process_array_reverse)(end, count) };
            let r = unsafe { (p.rs.process_array_reverse)(end, count) };
            assert_int_eq(&format!("process_array_reverse({buf:?}, {count})"), c, r);
        }
    }
}

#[test]
fn switch_fallthrough_calculator_matches() {
    let p = load_pair();

    let mut values: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        8,
        63,
        64,
        0o777,
        0o1000,
        0o100,
        0o200,
        255,
        256,
        1000,
        -1000,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        0x4000_0000,
        -0x4000_0000,
        0x2AAA_AAAA,
        0x5555_5555,
        -0x5555_5555,
    ];
    for k in -300..=300 {
        values.push(k);
    }
    for s in 0..31 {
        values.push(1 << s);
        values.push(-(1 << s));
        values.push((1 << s) - 1);
    }

    // Operations well beyond the labelled cases, and negatives, to cover
    // the `default` arm.
    let mut ops: Vec<c_int> = (-12..=12).collect();
    ops.extend([c_int::MAX, c_int::MIN, 100, -100]);

    for &v in &values {
        for &op in &ops {
            let c = unsafe { (p.c.switch_fallthrough_calculator)(v, op) };
            let r = unsafe { (p.rs.switch_fallthrough_calculator)(v, op) };
            assert_int_eq(
                &format!("switch_fallthrough_calculator({v}, {op})"),
                c,
                r,
            );
        }
    }
}

#[test]
fn foreach_sum_matches() {
    let p = load_pair();

    let buffers: Vec<Vec<c_int>> = vec![
        vec![],
        vec![0],
        vec![42],
        vec![-42],
        vec![1, 2, 3, 4, 5],
        vec![c_int::MAX, 1],
        vec![c_int::MIN, -1],
        vec![c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX],
        (0..128).collect(),
        (0..128).map(|i| i * -7919).collect(),
        (0..53).map(|i: c_int| i.wrapping_mul(0x0BAD_F00D)).collect(),
    ];

    for buf in &buffers {
        let mut b = buf.clone();
        let len = b.len() as c_int;
        let ptr = if b.is_empty() {
            std::ptr::null_mut()
        } else {
            b.as_mut_ptr()
        };
        let mut counts: Vec<c_int> = vec![0, -1, -9, c_int::MIN];
        for n in 1..=len {
            counts.push(n);
        }
        for &count in &counts {
            if b.is_empty() && count > 0 {
                continue;
            }
            let c = unsafe { (p.c.foreach_sum)(ptr, count) };
            let r = unsafe { (p.rs.foreach_sum)(ptr, count) };
            assert_int_eq(&format!("foreach_sum({buf:?}, {count})"), c, r);
        }
    }
}

#[test]
fn allocate_and_compute_matches() {
    let p = load_pair();

    // Sizes kept modest: huge sizes would make the C `malloc` outcome depend on
    // the host's memory state rather than on the translation.
    let mut sizes: Vec<c_int> = (-40..=200).collect();
    sizes.extend([
        512, 1000, 1024, 4095, 4096, 10_000, 65_536, 100_000, -1000, -65_536,
    ]);

    let multipliers: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        1.5,
        -1.5,
        0.25,
        -0.25,
        1e-8,
        1e8,
        1e18,
        -1e18,
        3.7,
        2.3,
        -0.5,
        1e300,
        -1e300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        123456.789,
    ];

    for &size in &sizes {
        for &m in &multipliers {
            let c = unsafe { (p.c.allocate_and_compute)(size, m) };
            let r = unsafe { (p.rs.allocate_and_compute)(size, m) };
            assert_int_eq(&format!("allocate_and_compute({size}, {m:?})"), c, r);
        }
    }
}

#[test]
fn fallcalc_matches_small_grid() {
    let p = load_pair();

    let vals: Vec<c_int> = (-12..=12).collect();
    for &a in &vals {
        for &b in &vals {
            for &c3 in &vals {
                for &d in &vals {
                    let c = unsafe { (p.c.fallcalc)(a, b, c3, d) };
                    let r = unsafe { (p.rs.fallcalc)(a, b, c3, d) };
                    assert_int_eq(&format!("fallcalc({a}, {b}, {c3}, {d})"), c, r);
                }
            }
        }
    }
}

#[test]
fn fallcalc_matches_extremes() {
    let p = load_pair();

    let vals: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        4,
        5,
        -5,
        9,
        10,
        -10,
        127,
        128,
        129,
        -128,
        0o177,
        0o200,
        0o201,
        0o777,
        0o1000,
        255,
        256,
        1000,
        -1000,
        65_535,
        65_536,
        -65_536,
        0x00FF_FFFF,
        0x0100_0000,
        0x4000_0000,
        -0x4000_0000,
        0x7FFF_FFFE,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        0x5555_5555,
        -0x5555_5555,
        0x2AAA_AAAB,
        21_474_836,
        -21_474_836,
        1_000_000,
        -1_000_000,
    ];

    for &a in &vals {
        for &b in &vals {
            for &c3 in &vals {
                for &d in &vals {
                    let c = unsafe { (p.c.fallcalc)(a, b, c3, d) };
                    let r = unsafe { (p.rs.fallcalc)(a, b, c3, d) };
                    assert_int_eq(&format!("fallcalc({a}, {b}, {c3}, {d})"), c, r);
                }
            }
        }
    }
}

/// Deterministic pseudo-random sweep over the whole `int` range.
#[test]
fn fallcalc_matches_random() {
    let p = load_pair();

    let mut state: u64 = 0x243F_6A88_85A3_08D3;
    let mut next = || -> c_int {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 16) as u32 as c_int
    };

    for _ in 0..200_000 {
        let (a, b, c3, d) = (next(), next(), next(), next());
        let c = unsafe { (p.c.fallcalc)(a, b, c3, d) };
        let r = unsafe { (p.rs.fallcalc)(a, b, c3, d) };
        assert_int_eq(&format!("fallcalc({a}, {b}, {c3}, {d})"), c, r);
    }
}

/// Every dynamic symbol the C `.so` exports must also be exported, under the
/// exact same name, by the Rust `.so`.
#[test]
fn exported_symbols_match() {
    let c_syms = common::dynamic_symbols(&common::c_so_path());
    let rs_syms = common::dynamic_symbols(&common::rust_so_path());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports: {c_syms:?}\nRust exports: {rs_syms:?}"
    );
    assert!(!c_syms.is_empty(), "nm reported no C symbols; check the build");
}
