//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded from their `.so` via `libloading`; the Rust
//! crate is never called directly.

mod common;

use std::ffi::{c_char, c_int};

use common::{apis, assert_f64_bits_eq, Rng};

// ---------------------------------------------------------------------------
// per-entry-point differential drivers
// ---------------------------------------------------------------------------

/// E1 `convert_double_to_int`
#[track_caller]
fn diff_e1(v: f64) {
    let (c, r) = apis();
    let cv = unsafe { (c.convert_double_to_int)(v) };
    let rv = unsafe { (r.convert_double_to_int)(v) };
    assert_eq!(
        cv, rv,
        "convert_double_to_int({v:?} / bits {:#018x}): C = {cv} vs Rust = {rv}",
        v.to_bits()
    );
}

/// E2 `find_value_in_buffer`
#[track_caller]
fn diff_e2(buf: &[u8], size: usize, search_val: c_int) {
    let (c, r) = apis();
    let p = buf.as_ptr() as *const c_char;
    let cv = unsafe { (c.find_value_in_buffer)(p, size, search_val) };
    let rv = unsafe { (r.find_value_in_buffer)(p, size, search_val) };
    assert_eq!(
        cv, rv,
        "find_value_in_buffer(buf[{}], size={size}, search_val={search_val} \
         (low byte {:#04x})): C = {cv} vs Rust = {rv}",
        buf.len(),
        (search_val as u8),
    );
}

/// E2 with an explicit raw pointer (for the NULL cases).
#[track_caller]
fn diff_e2_raw(p: *const c_char, size: usize, search_val: c_int) {
    let (c, r) = apis();
    let cv = unsafe { (c.find_value_in_buffer)(p, size, search_val) };
    let rv = unsafe { (r.find_value_in_buffer)(p, size, search_val) };
    assert_eq!(
        cv, rv,
        "find_value_in_buffer({p:?}, size={size}, search_val={search_val}): \
         C = {cv} vs Rust = {rv}"
    );
}

/// E3 `process_negation`
#[track_caller]
fn diff_e3(v: c_int) {
    let (c, r) = apis();
    let cv = unsafe { (c.process_negation)(v) };
    let rv = unsafe { (r.process_negation)(v) };
    assert_eq!(cv, rv, "process_negation({v}): C = {cv} vs Rust = {rv}");
}

/// E4 `create_numeric_buffer` — compares the whole scratch area, including the
/// bytes the function is expected NOT to touch.
#[track_caller]
fn diff_e4(cap: usize, size: c_int, seed: c_int) {
    let (c, r) = apis();
    const SENTINEL: i8 = 0x5A;
    let mut cb = vec![SENTINEL; cap];
    let mut rb = vec![SENTINEL; cap];
    unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), size, seed) };
    unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), size, seed) };
    assert_eq!(
        cb, rb,
        "create_numeric_buffer(cap={cap}, size={size}, seed={seed}) diverged"
    );
}

/// E4 with a NULL destination (only legal when nothing is written).
#[track_caller]
fn diff_e4_null(size: c_int, seed: c_int) {
    let (c, r) = apis();
    unsafe { (c.create_numeric_buffer)(std::ptr::null_mut(), size, seed) };
    unsafe { (r.create_numeric_buffer)(std::ptr::null_mut(), size, seed) };
}

/// E5 `calculate_with_doubles`
#[track_caller]
fn diff_e5(a: c_int, b: c_int, cc: c_int) -> f64 {
    let (c, r) = apis();
    let cv = unsafe { (c.calculate_with_doubles)(a, b, cc) };
    let rv = unsafe { (r.calculate_with_doubles)(a, b, cc) };
    assert_f64_bits_eq(cv, rv, &format!("calculate_with_doubles({a}, {b}, {cc})"));
    cv
}

/// E6 `doubleneg` is covered by `tests/phase_d_pipeline.rs` (a `harness = false`
/// target), because comparing its stdout requires exclusive use of the
/// process-global file descriptor 1.
fn rng(tag: u64) -> Rng {
    Rng::new(0x5EED_D0B1_E000_0000 ^ tag)
}

// ===========================================================================
// E1 — convert_double_to_int
// ===========================================================================

#[test]
fn cfg_row_01_e1_in_range_integral() {
    let mut g = rng(1);
    for _ in 0..5000 {
        diff_e1(g.next_i32() as f64);
    }
}

#[test]
fn cfg_row_02_e1_in_range_positive_fraction() {
    let mut g = rng(2);
    for _ in 0..5000 {
        let base = (g.next_i32() / 2) as f64;
        let frac = (g.next_u32() as f64) / (u32::MAX as f64);
        diff_e1(base.abs() + frac);
        diff_e1(base.abs() + 0.5);
        diff_e1(base.abs() + 0.999_999_999);
    }
}

#[test]
fn cfg_row_03_e1_in_range_negative_fraction() {
    let mut g = rng(3);
    for _ in 0..5000 {
        let base = -(((g.next_i32() / 2) as f64).abs());
        let frac = (g.next_u32() as f64) / (u32::MAX as f64);
        diff_e1(base - frac);
        diff_e1(base - 0.5);
        diff_e1(base - 0.999_999_999);
    }
}

#[test]
fn cfg_row_04_e1_exact_boundaries() {
    for v in [
        0.0_f64,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2147483647.0,
        -2147483648.0,
        2147483646.0,
        -2147483647.0,
        i32::MAX as f64,
        i32::MIN as f64,
    ] {
        diff_e1(v);
    }
}

#[test]
fn cfg_row_05_e1_just_past_boundaries() {
    let mut vs = vec![
        2147483647.5_f64,
        2147483647.999,
        2147483648.0,
        2147483648.5,
        2147483649.0,
        -2147483648.5,
        -2147483648.999,
        -2147483649.0,
        -2147483649.5,
    ];
    for base in [2147483647.0_f64, 2147483648.0, -2147483648.0, -2147483649.0] {
        let mut v = base;
        for _ in 0..4 {
            v = v.next_up();
            vs.push(v);
        }
        let mut v = base;
        for _ in 0..4 {
            v = v.next_down();
            vs.push(v);
        }
    }
    for v in vs {
        diff_e1(v);
    }
}

#[test]
fn cfg_row_06_e1_far_out_of_range() {
    for v in [
        1e10_f64, -1e10, 1e18, -1e18, 1e300, -1e300, f64::MAX, f64::MIN, 4.294967296e9,
        -4.294967296e9, 9.223372036854776e18, -9.223372036854776e18,
    ] {
        diff_e1(v);
    }
}

#[test]
fn cfg_row_07_e1_non_finite() {
    let mut vs = vec![f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -f64::NAN];
    // NaN payload variants (quiet and signalling, both signs).
    for bits in [
        0x7FF8_0000_0000_0000u64,
        0xFFF8_0000_0000_0000,
        0x7FF0_0000_0000_0001,
        0xFFF0_0000_0000_0001,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FF4_2424_2424_2424,
    ] {
        vs.push(f64::from_bits(bits));
    }
    for v in vs {
        diff_e1(v);
    }
}

#[test]
fn cfg_row_08_e1_subnormals_and_tiny() {
    for v in [
        5e-324_f64,
        -5e-324,
        1e-300,
        -1e-300,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        -0.0,
        0.0,
        f64::EPSILON,
        -f64::EPSILON,
        0.999_999_999_999,
        -0.999_999_999_999,
    ] {
        diff_e1(v);
    }
}

#[test]
fn cfg_row_09_e1_random_bit_patterns() {
    let mut g = rng(9);
    for _ in 0..20000 {
        diff_e1(g.next_f64_bits());
    }
    // Biased towards the interesting exponent range as well.
    for _ in 0..20000 {
        let mant = g.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let exp = 1000 + (g.next_u64() % 80); // exponents around 2^31
        let sign = (g.next_u64() & 1) << 63;
        diff_e1(f64::from_bits(sign | (exp << 52) | mant));
    }
}

// ===========================================================================
// E2 — find_value_in_buffer
// ===========================================================================

fn random_buf(g: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| g.next_u8()).collect()
}

#[test]
fn cfg_row_10_e2_needle_present_random_position() {
    let mut g = rng(10);
    for _ in 0..3000 {
        // Buffer with a single distinguished byte at a random position.
        let mut buf = vec![0x11u8; 256];
        let pos = g.range_usize(0, 255);
        buf[pos] = 0xC3;
        diff_e2(&buf, 256, 0xC3);
        diff_e2(&buf, 256, 0x11);
        // first / last explicitly
        let mut b2 = vec![0x22u8; 256];
        b2[0] = 0x99;
        diff_e2(&b2, 256, 0x99);
        let mut b3 = vec![0x22u8; 256];
        b3[255] = 0x99;
        diff_e2(&b3, 256, 0x99);
    }
}

#[test]
fn cfg_row_11_e2_needle_absent() {
    let mut g = rng(11);
    for _ in 0..3000 {
        let missing = g.next_u8();
        let buf: Vec<u8> = (0..256)
            .map(|_| {
                let mut b = g.next_u8();
                if b == missing {
                    b = missing.wrapping_add(1);
                }
                b
            })
            .collect();
        diff_e2(&buf, 256, missing as c_int);
    }
}

#[test]
fn cfg_row_12_e2_zero_size() {
    let buf = [1u8, 2, 3, 4];
    for sv in [0, 1, 42, -1, 255, 256, i32::MIN, i32::MAX] {
        diff_e2(&buf, 0, sv);
        diff_e2_raw(std::ptr::null(), 0, sv);
    }
}

#[test]
fn cfg_row_13_e2_size_one() {
    let mut g = rng(13);
    for _ in 0..2000 {
        let b = g.next_u8();
        let buf = [b, b.wrapping_add(1), b.wrapping_add(2)];
        diff_e2(&buf, 1, b as c_int);
        diff_e2(&buf, 1, b.wrapping_add(1) as c_int);
        diff_e2(&buf, 1, g.next_i32());
    }
}

#[test]
fn cfg_row_14_e2_duplicated_needle_returns_first() {
    let mut g = rng(14);
    for _ in 0..2000 {
        let mut buf = vec![0x00u8; 256];
        let n = g.range_usize(2, 20);
        let mut first = usize::MAX;
        for _ in 0..n {
            let p = g.range_usize(0, 255);
            buf[p] = 0x7E;
            first = first.min(p);
        }
        let cres = {
            let (c, _) = apis();
            unsafe { (c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, 0x7E) }
        };
        assert_eq!(cres, first as c_int, "C should return first match");
        diff_e2(&buf, 256, 0x7E);
    }
}

#[test]
fn cfg_row_15_e2_needle_only_past_size() {
    let mut g = rng(15);
    for _ in 0..2000 {
        let len = g.range_usize(2, 300);
        let cut = g.range_usize(1, len - 1);
        let mut buf = vec![0x01u8; len];
        buf[cut] = 0xAB; // strictly at/after `cut`
        diff_e2(&buf, cut, 0xAB); // undersized: must miss
        diff_e2(&buf, cut + 1, 0xAB); // one more byte: must hit
    }
}

#[test]
fn cfg_row_16_e2_high_bit_needles() {
    let mut g = rng(16);
    for _ in 0..2000 {
        let buf = random_buf(&mut g, 256);
        for b in 0x80u16..=0xFF {
            diff_e2(&buf, 256, b as c_int);
        }
        break;
    }
    // randomized buffers, randomized high-bit needle
    for _ in 0..3000 {
        let buf = random_buf(&mut g, 256);
        let needle = 0x80 | (g.next_u8() & 0x7F);
        diff_e2(&buf, 256, needle as c_int);
        diff_e2(&buf, 256, (needle as i8) as c_int); // sign-extended form
    }
}

#[test]
fn cfg_row_17_e2_search_val_above_255() {
    let mut g = rng(17);
    for _ in 0..3000 {
        let buf = random_buf(&mut g, 256);
        let low = g.next_u8();
        for hi in [0x100i32, 0x141, 0x1FF, 0x10000, 0x7F00_0000] {
            diff_e2(&buf, 256, hi | (low as c_int));
        }
        diff_e2(&buf, 256, 0x141);
        diff_e2(&buf, 256, 0x1FF);
    }
}

#[test]
fn cfg_row_18_e2_negative_and_extreme_search_val() {
    let mut g = rng(18);
    for _ in 0..3000 {
        let buf = random_buf(&mut g, 256);
        for sv in [-1i32, -128, -129, -255, -256, -257, i32::MIN, i32::MAX] {
            diff_e2(&buf, 256, sv);
        }
        diff_e2(&buf, 256, -(g.next_u8() as c_int));
    }
}

#[test]
fn cfg_row_19_e2_zero_needle() {
    let mut g = rng(19);
    for _ in 0..2000 {
        // buffer guaranteed to contain no NUL
        let with_no_nul: Vec<u8> = (0..256)
            .map(|_| {
                let b = g.next_u8();
                if b == 0 { 1 } else { b }
            })
            .collect();
        diff_e2(&with_no_nul, 256, 0);
        diff_e2(&with_no_nul, 256, 0x100); // narrows to 0 as well
        let mut with_nul = with_no_nul.clone();
        let p = g.range_usize(0, 255);
        with_nul[p] = 0;
        diff_e2(&with_nul, 256, 0);
        diff_e2(&with_nul, 256, 0x10000);
    }
}

#[test]
fn cfg_row_20_e2_fully_randomized() {
    let mut g = rng(20);
    for _ in 0..20000 {
        let len = g.range_usize(1, 512);
        let buf = random_buf(&mut g, len);
        let size = g.range_usize(0, len);
        diff_e2(&buf, size, g.next_i32());
    }
}

// ===========================================================================
// E3 — process_negation
// ===========================================================================

#[test]
fn cfg_row_21_e3_all_shapes() {
    for v in [0, 1, -1, 2, -2, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        diff_e3(v);
    }
    let mut g = rng(21);
    for _ in 0..20000 {
        diff_e3(g.next_i32());
    }
    for _ in 0..5000 {
        diff_e3(g.small_i32(3));
    }
}

// ===========================================================================
// E4 — create_numeric_buffer
// ===========================================================================

#[test]
fn cfg_row_22_e4_size256_positive_seed() {
    let mut g = rng(22);
    for _ in 0..3000 {
        diff_e4(300, 256, (g.next_u32() >> 1) as c_int);
    }
}

#[test]
fn cfg_row_23_e4_size256_negative_seed() {
    let mut g = rng(23);
    for _ in 0..3000 {
        let s = -((g.next_u32() >> 1) as c_int);
        diff_e4(300, 256, s);
    }
    for s in -300..=0 {
        diff_e4(300, 256, s);
    }
}

#[test]
fn cfg_row_24_e4_extreme_seeds() {
    for s in [
        0,
        1,
        -1,
        255,
        256,
        -255,
        -256,
        i32::MAX,
        i32::MAX - 7,
        i32::MAX - 1785,
        i32::MIN,
        i32::MIN + 7,
        i32::MIN + 1785,
    ] {
        for size in [1, 7, 256, 300, 1024] {
            diff_e4(2048, size, s);
        }
    }
}

#[test]
fn cfg_row_25_e4_zero_and_negative_size() {
    for size in [0, -1, -7, -256, i32::MIN, i32::MIN + 1] {
        for seed in [0, 1, -1, i32::MAX, i32::MIN] {
            diff_e4(64, size, seed);
            diff_e4_null(size, seed);
        }
    }
}

#[test]
fn cfg_row_26_e4_size_shapes() {
    let mut g = rng(26);
    for size in [1i32, 2, 7, 8, 255, 256, 257, 511, 512, 4096] {
        for _ in 0..40 {
            diff_e4(5000, size, g.next_i32());
        }
    }
    for _ in 0..2000 {
        let size = g.range_usize(0, 600) as c_int;
        diff_e4(700, size, g.next_i32());
    }
}

#[test]
fn cfg_row_27_e4_then_e2_composed() {
    let (c, r) = apis();
    let mut g = rng(27);
    for _ in 0..300 {
        let seed = g.next_i32();
        let mut cb = vec![0i8; 256];
        let mut rb = vec![0i8; 256];
        unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), 256, seed) };
        unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), 256, seed) };
        assert_eq!(cb, rb, "composed fill diverged for seed {seed}");
        for byte in 0..=255i32 {
            let cv = unsafe { (c.find_value_in_buffer)(cb.as_ptr(), 256, byte) };
            let rv = unsafe { (r.find_value_in_buffer)(rb.as_ptr(), 256, byte) };
            assert_eq!(cv, rv, "composed search seed={seed} byte={byte}");
        }
        // and the sign-extended / oversized needle forms
        for byte in [-1i32, -128, 256, 0x141, i32::MIN, i32::MAX] {
            let cv = unsafe { (c.find_value_in_buffer)(cb.as_ptr(), 256, byte) };
            let rv = unsafe { (r.find_value_in_buffer)(rb.as_ptr(), 256, byte) };
            assert_eq!(cv, rv, "composed search seed={seed} byte={byte}");
        }
    }
}

// ===========================================================================
// E5 — calculate_with_doubles
// ===========================================================================

#[test]
fn cfg_row_28_e5_b_zero_branch() {
    let mut g = rng(28);
    for _ in 0..5000 {
        diff_e5(g.next_i32(), 0, g.next_i32());
    }
    for c in -25..=25 {
        diff_e5(0, 0, c);
        diff_e5(i32::MAX, 0, c);
        diff_e5(i32::MIN, 0, c);
    }
}

#[test]
fn cfg_row_29_e5_exponent_zero() {
    let mut g = rng(29);
    for _ in 0..3000 {
        let mut b = g.next_i32();
        if b == 0 {
            b = 1;
        }
        let c = (g.next_i32() / 10).wrapping_mul(10); // multiple of 10 => c%10 == 0
        diff_e5(g.next_i32(), b, c);
    }
    for c in [0i32, 10, -10, 100, -100, 2147483640, -2147483640] {
        diff_e5(7, 3, c);
    }
}

#[test]
fn cfg_row_30_e5_positive_exponent() {
    let mut g = rng(30);
    for e in 1..=9i32 {
        for _ in 0..400 {
            let mut b = g.next_i32();
            if b == 0 {
                b = 1;
            }
            diff_e5(g.next_i32(), b, e);
            diff_e5(g.next_i32(), b, e + 10 * (g.next_u32() % 1000) as i32);
        }
    }
}

#[test]
fn cfg_row_31_e5_negative_exponent() {
    let mut g = rng(31);
    for e in -9..=-1i32 {
        for _ in 0..400 {
            let mut b = g.next_i32();
            if b == 0 {
                b = 1;
            }
            diff_e5(g.next_i32(), b, e);
            diff_e5(g.next_i32(), b, e - 10 * (g.next_u32() % 1000) as i32);
        }
    }
}

#[test]
fn cfg_row_32_e5_unit_divisors_extreme_dividend() {
    for a in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for b in [1i32, -1] {
            for c in -25..=25 {
                diff_e5(a, b, c);
            }
        }
    }
}

#[test]
fn cfg_row_33_e5_int_min_over_minus_one() {
    for c in -25..=25 {
        diff_e5(i32::MIN, -1, c);
        diff_e5(i32::MIN, i32::MIN, c);
        diff_e5(i32::MAX, i32::MIN, c);
    }
}

#[test]
fn cfg_row_34_e5_extreme_exponent_inputs() {
    let mut g = rng(34);
    for c in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -2147483640] {
        for _ in 0..500 {
            let mut b = g.next_i32();
            if b == 0 {
                b = 1;
            }
            diff_e5(g.next_i32(), b, c);
        }
        diff_e5(1, 3, c);
        diff_e5(i32::MIN, -1, c);
    }
}

#[test]
fn cfg_row_35_e5_fully_randomized() {
    let mut g = rng(35);
    for _ in 0..20000 {
        diff_e5(g.next_i32(), g.next_i32(), g.next_i32());
    }
    for _ in 0..10000 {
        diff_e5(g.small_i32(20), g.small_i32(20), g.small_i32(30));
    }
}

#[test]
fn cfg_row_36_e5_overflow_underflow_results() {
    // Largest magnitude ratio times 1e9, and smallest times 1e-9.
    for (a, b) in [
        (i32::MAX, 1),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (i32::MIN, -1),
        (1, i32::MAX),
        (1, i32::MIN),
        (-1, i32::MAX),
    ] {
        for c in [9i32, -9, 8, -8, 1, -1, 0, 19, -19] {
            diff_e5(a, b, c);
        }
    }
}

#[test]
fn cfg_row_37_e5_then_e1_composed() {
    let (c, r) = apis();
    let mut g = rng(37);
    for _ in 0..20000 {
        let (a, b, cc) = (g.next_i32(), g.next_i32(), g.next_i32());
        let cd = unsafe { (c.calculate_with_doubles)(a, b, cc) };
        let rd = unsafe { (r.calculate_with_doubles)(a, b, cc) };
        assert_f64_bits_eq(cd, rd, &format!("calc({a},{b},{cc})"));
        let ci = unsafe { (c.convert_double_to_int)(cd) };
        let ri = unsafe { (r.convert_double_to_int)(rd) };
        assert_eq!(ci, ri, "composed calc->convert({a},{b},{cc}) d={cd:?}");
        // and the `% 1000` step doubleneg applies
        assert_eq!(ci.wrapping_rem(1000), ri.wrapping_rem(1000));
    }
}

