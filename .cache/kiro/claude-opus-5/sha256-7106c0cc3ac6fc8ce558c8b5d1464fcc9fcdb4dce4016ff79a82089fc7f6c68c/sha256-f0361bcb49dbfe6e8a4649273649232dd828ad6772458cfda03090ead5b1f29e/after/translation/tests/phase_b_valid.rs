//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `libloading` into the C `.so` and the Rust `.so`.

mod common;

use common::*;

// ===========================================================================
// safe_double_to_int  (C1 .. C7)
// ===========================================================================

fn sdti_row(row: &'static str, values: impl IntoIterator<Item = f64>) {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new(row);
    for v in values {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("d={v:?} bits={:#018x}", v.to_bits()), cv, rv);
    }
    d.finish();
}

#[test]
fn c01_safe_double_to_int_in_range_positive() {
    let mut rng = Rng::new(0xC001);
    let mut v = Vec::new();
    for _ in 0..20_000 {
        // Uniform over the whole positive in-range span, with a fractional part.
        let whole = (rng.next_u64() % 2_147_483_647) as f64;
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        v.push(whole + frac);
    }
    sdti_row("C1", v);
}

#[test]
fn c02_safe_double_to_int_in_range_negative() {
    let mut rng = Rng::new(0xC002);
    let mut v = Vec::new();
    for _ in 0..20_000 {
        let whole = (rng.next_u64() % 2_147_483_648) as f64;
        let frac = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        v.push(-(whole + frac));
    }
    sdti_row("C2", v);
}

#[test]
fn c03_safe_double_to_int_zeros_and_subnormals() {
    let mut v = vec![
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        f64::from_bits(0x800F_FFFF_FFFF_FFFF),
        0.5,
        -0.5,
        0.9999999999999999,
        -0.9999999999999999,
        1.0,
        -1.0,
    ];
    let mut rng = Rng::new(0xC003);
    for _ in 0..2000 {
        // random subnormal
        v.push(f64::from_bits(rng.next_u64() & 0x800F_FFFF_FFFF_FFFF));
    }
    sdti_row("C3", v);
}

#[test]
fn c04_safe_double_to_int_clamp_boundaries() {
    let imax = 2147483647.0f64;
    let imin = -2147483648.0f64;
    let mut v = Vec::new();
    for base in [imax, imin] {
        let mut x = base;
        for _ in 0..64 {
            v.push(x);
            x = f64::from_bits(x.to_bits().wrapping_sub(1));
        }
        let mut x = base;
        for _ in 0..64 {
            v.push(x);
            x = f64::from_bits(x.to_bits().wrapping_add(1));
        }
        for k in -8..=8 {
            v.push(base + k as f64);
            v.push(base + k as f64 * 0.5);
            v.push(base - k as f64 * 0.25);
        }
    }
    v.push(2147483646.9999998);
    v.push(-2147483647.9999998);
    v.push(2147483648.0);
    v.push(-2147483649.0);
    sdti_row("C4", v);
}

#[test]
fn c05_safe_double_to_int_out_of_range_finite() {
    let mut v = vec![
        1e10,
        -1e10,
        1e15,
        -1e15,
        1e100,
        -1e100,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        2.0f64.powi(31),
        -(2.0f64.powi(31)) - 1.0,
        2.0f64.powi(52),
        2.0f64.powi(63),
        -(2.0f64.powi(63)),
        2.0f64.powi(1023),
    ];
    let mut rng = Rng::new(0xC005);
    for _ in 0..5000 {
        let e = rng.range_i32(31, 1023);
        let m = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 + 1.0;
        let x = m * 2f64.powi(e);
        v.push(x);
        v.push(-x);
    }
    sdti_row("C5", v);
}

#[test]
fn c06_safe_double_to_int_non_finite() {
    let mut v = vec![
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ];
    let mut rng = Rng::new(0xC006);
    for _ in 0..2000 {
        // random NaN payload, both signs
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        if payload == 0 {
            continue;
        }
        v.push(f64::from_bits(0x7FF0_0000_0000_0000 | payload));
        v.push(f64::from_bits(0xFFF0_0000_0000_0000 | payload));
    }
    sdti_row("C6", v);
}

#[test]
fn c07_safe_double_to_int_random_bit_patterns() {
    let mut rng = Rng::new(0xC007);
    let mut v = Vec::new();
    for _ in 0..50_000 {
        v.push(rng.next_f64_bits());
    }
    for _ in 0..20_000 {
        v.push(rng.next_finite_f64());
    }
    sdti_row("C7", v);
}

// ===========================================================================
// process_array_reverse  (C8 .. C13)
// ===========================================================================

#[test]
fn c08_process_array_reverse_count_zero() {
    let (c, r) = process_array_reverse();
    let mut d = Diff::new("C8");
    let mut buf = [1i32, 2, 3, 4];
    let ptrs: [*mut i32; 4] = [
        std::ptr::null_mut(),
        buf.as_mut_ptr(),
        unsafe { buf.as_mut_ptr().add(3) },
        0x1usize as *mut i32,
    ];
    for p in ptrs {
        let cv = unsafe { c(p, 0) };
        let rv = unsafe { r(p, 0) };
        d.check(format!("end={p:?} count=0"), cv, rv);
    }
    d.finish();
}

#[test]
fn c09_process_array_reverse_count_one() {
    let (c, r) = process_array_reverse();
    let mut rng = Rng::new(0xC009);
    let mut d = Diff::new("C9");
    for _ in 0..20_000 {
        let mut v = [rng.next_i32()];
        let end = v.as_mut_ptr();
        let cv = unsafe { c(end, 1) };
        let rv = unsafe { r(end, 1) };
        d.check(format!("v={:?}", v[0]), cv, rv);
    }
    d.finish();
}

#[test]
fn c10_process_array_reverse_many_from_last() {
    let (c, r) = process_array_reverse();
    let mut rng = Rng::new(0xC010);
    let mut d = Diff::new("C10");
    for _ in 0..5000 {
        let n = rng.range_i32(2, 64);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let end = unsafe { v.as_mut_ptr().add(n as usize - 1) };
        let cv = unsafe { c(end, n) };
        let rv = unsafe { r(end, n) };
        d.check(format!("n={n} v={:?}", &v[..v.len().min(6)]), cv, rv);
    }
    d.finish();
}

#[test]
fn c11_process_array_reverse_interior_window() {
    let (c, r) = process_array_reverse();
    let mut rng = Rng::new(0xC011);
    let mut d = Diff::new("C11");
    for _ in 0..5000 {
        let n = rng.range_i32(4, 96);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        // Pick a window [start ..= endidx] of length `count` inside the buffer.
        let endidx = rng.range_i32(1, n - 1);
        let count = rng.range_i32(1, endidx + 1);
        let endp = unsafe { v.as_mut_ptr().add(endidx as usize) };
        let cv = unsafe { c(endp, count) };
        let rv = unsafe { r(endp, count) };
        d.check(format!("n={n} endidx={endidx} count={count}"), cv, rv);
    }
    d.finish();
}

#[test]
fn c12_process_array_reverse_overflowing_sum() {
    let (c, r) = process_array_reverse();
    let mut rng = Rng::new(0xC012);
    let mut d = Diff::new("C12");
    let extremes = [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2, -1, 1];
    for _ in 0..5000 {
        let n = rng.range_i32(2, 40);
        let mut v: Vec<i32> = (0..n)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    extremes[(rng.next_u64() as usize) % extremes.len()]
                } else {
                    rng.next_i32()
                }
            })
            .collect();
        let end = unsafe { v.as_mut_ptr().add(n as usize - 1) };
        let cv = unsafe { c(end, n) };
        let rv = unsafe { r(end, n) };
        d.check(format!("n={n} v={:?}", &v[..v.len().min(6)]), cv, rv);
    }
    d.finish();
}

#[test]
fn c13_process_array_reverse_negative_count() {
    let (c, r) = process_array_reverse();
    let mut rng = Rng::new(0xC013);
    let mut d = Diff::new("C13");
    let mut buf = [7i32, 8, 9, 10];
    let end = unsafe { buf.as_mut_ptr().add(3) };
    let mut counts = vec![-1, -2, -100, i32::MIN, i32::MIN + 1];
    for _ in 0..2000 {
        counts.push(-(rng.range_i32(1, i32::MAX - 1)));
    }
    for n in counts {
        let cv = unsafe { c(end, n) };
        let rv = unsafe { r(end, n) };
        d.check(format!("count={n}"), cv, rv);
        let cv = unsafe { c(std::ptr::null_mut(), n) };
        let rv = unsafe { r(std::ptr::null_mut(), n) };
        d.check(format!("NULL count={n}"), cv, rv);
    }
    d.finish();
}

// ===========================================================================
// foreach_sum  (C14 .. C19)
// ===========================================================================

#[test]
fn c14_foreach_sum_count_zero() {
    let (c, r) = foreach_sum();
    let mut d = Diff::new("C14");
    let mut buf = [1i32, 2, 3];
    for p in [std::ptr::null_mut(), buf.as_mut_ptr(), 0x1usize as *mut i32] {
        let cv = unsafe { c(p, 0) };
        let rv = unsafe { r(p, 0) };
        d.check(format!("array={p:?} count=0"), cv, rv);
    }
    d.finish();
}

#[test]
fn c15_foreach_sum_count_one() {
    let (c, r) = foreach_sum();
    let mut rng = Rng::new(0xC015);
    let mut d = Diff::new("C15");
    for _ in 0..20_000 {
        let mut v = [rng.next_i32()];
        let cv = unsafe { c(v.as_mut_ptr(), 1) };
        let rv = unsafe { r(v.as_mut_ptr(), 1) };
        d.check(format!("v={}", v[0]), cv, rv);
    }
    d.finish();
}

#[test]
fn c16_foreach_sum_many() {
    let (c, r) = foreach_sum();
    let mut rng = Rng::new(0xC016);
    let mut d = Diff::new("C16");
    for _ in 0..5000 {
        let n = rng.range_i32(2, 64);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let cv = unsafe { c(v.as_mut_ptr(), n) };
        let rv = unsafe { r(v.as_mut_ptr(), n) };
        d.check(format!("n={n} v={:?}", &v[..v.len().min(6)]), cv, rv);
    }
    d.finish();
}

#[test]
fn c17_foreach_sum_overflowing() {
    let (c, r) = foreach_sum();
    let mut rng = Rng::new(0xC017);
    let mut d = Diff::new("C17");
    let extremes = [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2, -1, 1];
    for _ in 0..5000 {
        let n = rng.range_i32(2, 40);
        let mut v: Vec<i32> = (0..n)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    extremes[(rng.next_u64() as usize) % extremes.len()]
                } else {
                    rng.next_i32()
                }
            })
            .collect();
        let cv = unsafe { c(v.as_mut_ptr(), n) };
        let rv = unsafe { r(v.as_mut_ptr(), n) };
        d.check(format!("n={n} v={:?}", &v[..v.len().min(6)]), cv, rv);
    }
    d.finish();
}

#[test]
fn c18_foreach_sum_negative_count() {
    let (c, r) = foreach_sum();
    let mut rng = Rng::new(0xC018);
    let mut d = Diff::new("C18");
    let mut buf = [4i32, 5, 6];
    let mut counts = vec![-1, -3, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..2000 {
        counts.push(-(rng.range_i32(1, i32::MAX - 1)));
    }
    for n in counts {
        let cv = unsafe { c(buf.as_mut_ptr(), n) };
        let rv = unsafe { r(buf.as_mut_ptr(), n) };
        d.check(format!("count={n}"), cv, rv);
        let cv = unsafe { c(std::ptr::null_mut(), n) };
        let rv = unsafe { r(std::ptr::null_mut(), n) };
        d.check(format!("NULL count={n}"), cv, rv);
    }
    d.finish();
}

#[test]
fn c19_foreach_and_reverse_same_buffer() {
    let (fc, fr) = foreach_sum();
    let (pc, pr) = process_array_reverse();
    let mut rng = Rng::new(0xC019);
    let mut d = Diff::new("C19");
    for _ in 0..5000 {
        let n = rng.range_i32(1, 48);
        let mut v: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
        let f_c = unsafe { fc(v.as_mut_ptr(), n) };
        let f_r = unsafe { fr(v.as_mut_ptr(), n) };
        d.check(format!("foreach n={n}"), f_c, f_r);
        let end = unsafe { v.as_mut_ptr().add(n as usize - 1) };
        let p_c = unsafe { pc(end, n) };
        let p_r = unsafe { pr(end, n) };
        d.check(format!("reverse n={n}"), p_c, p_r);
        // Cross-check: both C functions must agree with each other too.
        d.check(format!("C-internal n={n}"), f_c, p_c);
    }
    d.finish();
}

// ===========================================================================
// switch_fallthrough_calculator  (C20 .. C25)
// ===========================================================================

fn switch_row(row: &'static str, op_gen: impl Fn(&mut Rng) -> i32, seed: u64) {
    let (c, r) = switch_fallthrough_calculator();
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1,
        0,
        1,
        511,
        512,
        128,
        64,
        -511,
        -512,
        i32::MAX / 8,
        i32::MIN / 8,
        i32::MAX / 3,
        i32::MIN / 3,
    ];
    for &v in &extremes {
        let op = op_gen(&mut rng);
        let cv = unsafe { c(v, op) };
        let rv = unsafe { r(v, op) };
        d.check(format!("value={v} op={op}"), cv, rv);
    }
    for _ in 0..20_000 {
        let v = if rng.next_u64() % 4 == 0 {
            extremes[(rng.next_u64() as usize) % extremes.len()]
                .wrapping_add(rng.range_i32(-4, 4))
        } else {
            rng.next_i32()
        };
        let op = op_gen(&mut rng);
        let cv = unsafe { c(v, op) };
        let rv = unsafe { r(v, op) };
        d.check(format!("value={v} op={op}"), cv, rv);
    }
    d.finish();
}

#[test]
fn c20_switch_op0() {
    switch_row("C20", |_| 0, 0xC020);
}

#[test]
fn c21_switch_op1() {
    switch_row("C21", |_| 1, 0xC021);
}

#[test]
fn c22_switch_op2() {
    switch_row("C22", |_| 2, 0xC022);
}

#[test]
fn c23_switch_op3() {
    switch_row("C23", |_| 3, 0xC023);
}

#[test]
fn c24_switch_op4() {
    switch_row("C24", |_| 4, 0xC024);
}

#[test]
fn c25_switch_op_default() {
    switch_row(
        "C25",
        |rng| loop {
            let op = if rng.next_u64() % 3 == 0 {
                rng.range_i32(-16, 16)
            } else {
                rng.next_i32()
            };
            if !(0..=4).contains(&op) {
                return op;
            }
        },
        0xC025,
    );
}

// ===========================================================================
// allocate_and_compute  (C26 .. C35)
// ===========================================================================

fn alloc_row(
    row: &'static str,
    cases: impl IntoIterator<Item = (i32, f64)>,
) {
    let (c, r) = allocate_and_compute();
    let mut d = Diff::new(row);
    for (size, mult) in cases {
        let cv = unsafe { c(size, mult) };
        let rv = unsafe { r(size, mult) };
        d.check(format!("size={size} mult={mult:?}"), cv, rv);
    }
    d.finish();
}

#[test]
fn c26_allocate_size_zero() {
    let mut rng = Rng::new(0xC026);
    let mut cases = vec![
        (0, 0.0),
        (0, 1.5),
        (0, -1.5),
        (0, f64::NAN),
        (0, f64::INFINITY),
        (0, f64::NEG_INFINITY),
        (0, f64::MAX),
    ];
    for _ in 0..5000 {
        cases.push((0, rng.next_finite_f64()));
    }
    alloc_row("C26", cases);
}

#[test]
fn c27_allocate_size_one() {
    let mut rng = Rng::new(0xC027);
    let mut cases = vec![
        (1, 0.0),
        (1, -0.0),
        (1, 1.5),
        (1, -1.5),
        (1, f64::NAN),
        (1, f64::INFINITY),
        (1, f64::NEG_INFINITY),
        (1, f64::MAX),
        (1, f64::MIN),
    ];
    for _ in 0..5000 {
        cases.push((1, rng.next_finite_f64()));
    }
    for _ in 0..2000 {
        cases.push((1, rng.next_f64_bits()));
    }
    alloc_row("C27", cases);
}

#[test]
fn c28_allocate_typical_sizes() {
    let mut rng = Rng::new(0xC028);
    let mut cases = Vec::new();
    for _ in 0..8000 {
        let size = rng.range_i32(2, 64);
        cases.push((size, rng.next_finite_f64()));
    }
    for size in 2..=64 {
        for m in [0.0, 1.0, 1.5, -1.5, 2.5, -2.5, 1e-8, 1e8, -1e8] {
            cases.push((size, m));
        }
    }
    alloc_row("C28", cases);
}

#[test]
fn c29_allocate_saturating_positive() {
    let mut rng = Rng::new(0xC029);
    let mut cases = vec![
        (10_000, 1.5),
        (10_000, 1.0),
        (5_000, 2.0),
        (2_000, 100.0),
        (100_000, 1.5),
    ];
    for _ in 0..300 {
        cases.push((rng.range_i32(1000, 20_000), rng.range_i32(1, 1000) as f64));
    }
    alloc_row("C29", cases);
}

#[test]
fn c30_allocate_saturating_negative() {
    let mut rng = Rng::new(0xC030);
    let mut cases = vec![
        (10_000, -1.5),
        (10_000, -1.0),
        (5_000, -2.0),
        (2_000, -100.0),
        (100_000, -1.5),
    ];
    for _ in 0..300 {
        cases.push((rng.range_i32(1000, 20_000), -(rng.range_i32(1, 1000) as f64)));
    }
    alloc_row("C30", cases);
}

#[test]
fn c31_allocate_zero_multiplier() {
    let mut cases = Vec::new();
    for size in [0, 1, 2, 3, 7, 16, 64, 1000, 10_000] {
        cases.push((size, 0.0));
        cases.push((size, -0.0));
    }
    alloc_row("C31", cases);
}

#[test]
fn c32_allocate_infinite_multiplier() {
    let mut cases = Vec::new();
    for size in [0, 1, 2, 3, 4, 8, 33, 100, 1000] {
        cases.push((size, f64::INFINITY));
        cases.push((size, f64::NEG_INFINITY));
    }
    alloc_row("C32", cases);
}

#[test]
fn c33_allocate_nan_multiplier() {
    let mut cases = Vec::new();
    for size in [0, 1, 2, 3, 4, 8, 33, 100, 1000] {
        cases.push((size, f64::NAN));
        cases.push((size, -f64::NAN));
        cases.push((size, f64::from_bits(0x7FF0_0000_0000_0001)));
    }
    alloc_row("C33", cases);
}

#[test]
fn c34_allocate_extreme_finite_multiplier() {
    let mut cases = Vec::new();
    for size in [0, 1, 2, 3, 4, 8, 33, 100] {
        for m in [1e300, -1e300, f64::MAX, f64::MIN, 1e-300, -1e-300, 1e160, -1e160] {
            cases.push((size, m));
        }
    }
    alloc_row("C34", cases);
}

#[test]
fn c35_allocate_negative_size() {
    let mut rng = Rng::new(0xC035);
    let mut cases = vec![
        (-1, 1.5),
        (-2, 1.5),
        (-16, -1.5),
        (i32::MIN, 1.5),
        (i32::MIN + 1, f64::NAN),
        (-1_000_000, f64::INFINITY),
    ];
    for _ in 0..2000 {
        cases.push((-(rng.range_i32(1, i32::MAX - 1)), rng.next_finite_f64()));
    }
    alloc_row("C35", cases);
}

// ===========================================================================
// fallcalc  (C36 .. C52)
// ===========================================================================

fn fallcalc_row(row: &'static str, cases: impl IntoIterator<Item = (i32, i32, i32, i32)>) {
    let (c, r) = fallcalc();
    let mut d = Diff::new(row);
    for (a, b, cc, dd) in cases {
        let cv = unsafe { c(a, b, cc, dd) };
        let rv = unsafe { r(a, b, cc, dd) };
        d.check(format!("({a}, {b}, {cc}, {dd})"), cv, rv);
    }
    d.finish();
}

/// Random param4 whose `% 10 + 1` is > 0 (i.e. `param4 % 10 != -1`).
fn p4_positive_size(rng: &mut Rng) -> i32 {
    loop {
        let v = rng.next_i32();
        let m = v % 10;
        if m >= 0 {
            return v;
        }
    }
}

fn fallcalc_params_with_p3_mod(rng: &mut Rng, want: i32, n: usize) -> Vec<(i32, i32, i32, i32)> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let p3 = rng.next_i32();
        if p3 % 5 != want {
            continue;
        }
        out.push((rng.next_i32(), rng.next_i32(), p3, rng.next_i32()));
    }
    out
}

#[test]
fn c36_fallcalc_switch_case0() {
    let mut rng = Rng::new(0xC036);
    fallcalc_row("C36", fallcalc_params_with_p3_mod(&mut rng, 0, 6000));
}

#[test]
fn c37_fallcalc_switch_case1() {
    let mut rng = Rng::new(0xC037);
    fallcalc_row("C37", fallcalc_params_with_p3_mod(&mut rng, 1, 6000));
}

#[test]
fn c38_fallcalc_switch_case2() {
    let mut rng = Rng::new(0xC038);
    fallcalc_row("C38", fallcalc_params_with_p3_mod(&mut rng, 2, 6000));
}

#[test]
fn c39_fallcalc_switch_case3() {
    let mut rng = Rng::new(0xC039);
    fallcalc_row("C39", fallcalc_params_with_p3_mod(&mut rng, 3, 6000));
}

#[test]
fn c40_fallcalc_switch_case4() {
    let mut rng = Rng::new(0xC040);
    fallcalc_row("C40", fallcalc_params_with_p3_mod(&mut rng, 4, 6000));
}

#[test]
fn c41_fallcalc_switch_default_negative_p3() {
    let mut rng = Rng::new(0xC041);
    let mut cases = Vec::new();
    while cases.len() < 8000 {
        let p3 = -(rng.range_i32(1, i32::MAX - 1));
        if p3 % 5 == 0 {
            continue;
        }
        cases.push((rng.next_i32(), rng.next_i32(), p3, rng.next_i32()));
    }
    for m in 1..=4 {
        for k in 0..40 {
            let p3 = -(m + 5 * k);
            cases.push((k, -k, p3, k * 7));
        }
    }
    fallcalc_row("C41", cases);
}

#[test]
fn c42_fallcalc_inner_alloc_positive() {
    let mut rng = Rng::new(0xC042);
    let mut cases = Vec::new();
    for _ in 0..8000 {
        let p4 = p4_positive_size(&mut rng);
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), p4));
    }
    for p4 in 0..40 {
        cases.push((1, 2, 3, p4));
    }
    fallcalc_row("C42", cases);
}

#[test]
fn c43_fallcalc_inner_alloc_zero() {
    // param4 % 10 == -1  =>  size 0  =>  malloc(0) path
    let mut rng = Rng::new(0xC043);
    let mut cases = Vec::new();
    for k in 0..200 {
        cases.push((k, k * 3, k * 5, -1 - 10 * k));
    }
    while cases.len() < 4000 {
        let p4 = -(1 + 10 * rng.range_i32(0, 214_748_363));
        if p4 % 10 != -1 {
            continue;
        }
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), p4));
    }
    fallcalc_row("C43", cases);
}

#[test]
fn c44_fallcalc_inner_alloc_negative() {
    // param4 % 10 in {-9..-2}  =>  size < 0  =>  malloc failure  =>  -1
    let mut rng = Rng::new(0xC044);
    let mut cases = Vec::new();
    for m in 2..=9 {
        for k in 0..50 {
            cases.push((k, -k * 2, k * 3, -(m + 10 * k)));
        }
    }
    while cases.len() < 4000 {
        let p4 = -(rng.range_i32(1, i32::MAX - 1));
        let r = p4 % 10;
        if !(-9..=-2).contains(&r) {
            continue;
        }
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), p4));
    }
    fallcalc_row("C44", cases);
}

#[test]
fn c45_fallcalc_flag_taken() {
    let mut rng = Rng::new(0xC045);
    let mut cases = vec![(0, 0, 129, 0), (1, 1, 130, 1), (0, 0, i32::MAX, 0)];
    for _ in 0..8000 {
        let p3 = rng.range_i32(129, i32::MAX);
        cases.push((rng.next_i32(), rng.next_i32(), p3, rng.next_i32()));
    }
    fallcalc_row("C45", cases);
}

#[test]
fn c46_fallcalc_flag_not_taken() {
    let mut rng = Rng::new(0xC046);
    let mut cases = vec![(0, 0, 128, 0), (1, 1, 127, 1), (0, 0, i32::MIN, 0)];
    for _ in 0..8000 {
        let p3 = rng.range_i32(i32::MIN, 128);
        cases.push((rng.next_i32(), rng.next_i32(), p3, rng.next_i32()));
    }
    fallcalc_row("C46", cases);
}

#[test]
fn c47_fallcalc_param1_overflow() {
    let mut rng = Rng::new(0xC047);
    let mut cases = Vec::new();
    let seeds = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 64,
        i32::MIN / 64,
        1 << 25,
        -(1 << 25),
        33_554_432,
    ];
    for &p1 in &seeds {
        for k in -4..=4 {
            for p4 in [0, 1, 5, 9, -1, -3] {
                cases.push((p1.wrapping_add(k), 7, 3, p4));
            }
        }
    }
    for _ in 0..6000 {
        let p1 = if rng.next_u64() & 1 == 0 {
            rng.next_i32()
        } else {
            seeds[(rng.next_u64() as usize) % seeds.len()].wrapping_add(rng.range_i32(-64, 64))
        };
        cases.push((p1, rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    fallcalc_row("C47", cases);
}

#[test]
fn c48_fallcalc_param2_overflow() {
    let mut rng = Rng::new(0xC048);
    let mut cases = Vec::new();
    let seeds = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 8,
        i32::MIN / 8,
        i32::MAX / 3,
        i32::MIN / 3,
    ];
    for &p2 in &seeds {
        for k in -4..=4 {
            for p3 in [0, 1, 2, 3, 4, 5, -1, -2, -3, -4] {
                cases.push((3, p2.wrapping_add(k), p3, 4));
            }
        }
    }
    for _ in 0..6000 {
        let p2 = if rng.next_u64() & 1 == 0 {
            rng.next_i32()
        } else {
            seeds[(rng.next_u64() as usize) % seeds.len()].wrapping_add(rng.range_i32(-64, 64))
        };
        cases.push((rng.next_i32(), p2, rng.next_i32(), rng.next_i32()));
    }
    fallcalc_row("C48", cases);
}

#[test]
fn c49_fallcalc_floating_saturation() {
    let mut rng = Rng::new(0xC049);
    let extremes = [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1];
    let mut cases = Vec::new();
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                for d in [0, 1, 9, -1, -5] {
                    cases.push((a, b, c, d));
                }
            }
        }
    }
    for _ in 0..4000 {
        let pick = |rng: &mut Rng| extremes[(rng.next_u64() as usize) % extremes.len()];
        let a = pick(&mut rng).wrapping_add(rng.range_i32(-3, 3));
        let b = pick(&mut rng).wrapping_add(rng.range_i32(-3, 3));
        let c = pick(&mut rng).wrapping_add(rng.range_i32(-3, 3));
        cases.push((a, b, c, rng.next_i32()));
    }
    fallcalc_row("C49", cases);
}

#[test]
fn c50_fallcalc_corner_grid() {
    let corners = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        128,
        129,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut cases = Vec::new();
    for &a in &corners {
        for &b in &corners {
            for &c in &corners {
                for &d in &corners {
                    cases.push((a, b, c, d));
                }
            }
        }
    }
    assert_eq!(cases.len(), 9 * 9 * 9 * 9);
    fallcalc_row("C50", cases);
}

#[test]
fn c51_fallcalc_uniform_random() {
    let mut rng = Rng::new(0xC051);
    let mut cases = Vec::new();
    for _ in 0..40_000 {
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    fallcalc_row("C51", cases);
}

#[test]
fn c52_fallcalc_small_magnitude() {
    let mut rng = Rng::new(0xC052);
    let mut cases = Vec::new();
    for _ in 0..40_000 {
        cases.push((
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
        ));
    }
    for a in -6..=6 {
        for b in -6..=6 {
            for c in -6..=6 {
                for d in -6..=6 {
                    cases.push((a, b, c, d));
                }
            }
        }
    }
    fallcalc_row("C52", cases);
}
