//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row
//! (grouped where the rows share a driver), every one randomized with a fixed
//! seed. Both implementations are invoked through their `.so` exports only.

mod harness;

use std::ffi::{c_char, c_int};

use harness::{Rng, apis};

// ---------------------------------------------------------------------------
// Rows 1-3 — process_negation
// ---------------------------------------------------------------------------

#[test]
fn cfg_negation() {
    let p = apis();
    let mut fixed: Vec<i32> = vec![0, 1, -1, 2, -2, 255, 256, -256, i32::MAX, i32::MIN];
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for _ in 0..5000 {
        fixed.push(rng.spicy_i32());
    }
    for v in fixed {
        let (c, r) = unsafe { ((p.c.process_negation)(v), (p.rust.process_negation)(v)) };
        assert_eq!(c, r, "process_negation({v})");
        // Row 1/2/3 semantics: exactly 0 or 1.
        assert!(c == 0 || c == 1, "process_negation({v}) = {c}");
    }
}

// ---------------------------------------------------------------------------
// Rows 4-10 — convert_double_to_int
// ---------------------------------------------------------------------------

fn check_conv(values: &[f64], label: &str) {
    let p = apis();
    for &v in values {
        let (c, r) = unsafe {
            (
                (p.c.convert_double_to_int)(v),
                (p.rust.convert_double_to_int)(v),
            )
        };
        assert_eq!(
            c, r,
            "{label}: convert_double_to_int({v:?} / bits {:#018x}) C={c} Rust={r}",
            v.to_bits()
        );
    }
}

#[test]
fn cfg_conv_in_range() {
    let mut rng = Rng::new(0xAAAA_0001);
    let mut v: Vec<f64> = vec![0.0, 1.0, -1.0, 2147483647.0, -2147483648.0];
    for _ in 0..5000 {
        v.push(rng.next_i32() as f64);
    }
    check_conv(&v, "row4 in-range integral");
}

#[test]
fn cfg_conv_fractional() {
    let mut rng = Rng::new(0xAAAA_0002);
    let mut v: Vec<f64> = vec![0.5, -0.5, 1.9, -1.9, 42.7, -42.7, 0.9999999999999999];
    for _ in 0..5000 {
        let base = rng.next_i32() as f64;
        let frac = (rng.next_u32() as f64) / (u32::MAX as f64 + 1.0);
        v.push(base + frac);
        v.push(base - frac);
    }
    check_conv(&v, "row5 fractional truncation");
}

#[test]
fn cfg_conv_tiny() {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),                  // smallest subnormal
        f64::from_bits(1 | (1u64 << 63)),   // negative smallest subnormal
        f64::from_bits(0x000f_ffff_ffff_ffff), // largest subnormal
        1e-300,
        -1e-300,
        f64::EPSILON,
        -f64::EPSILON,
    ];
    let mut rng = Rng::new(0xAAAA_0003);
    for _ in 0..2000 {
        // Random subnormals and tiny normals.
        v.push(f64::from_bits(rng.next_u64() >> 12));
        v.push(f64::from_bits((rng.next_u64() >> 12) | (1u64 << 63)));
    }
    check_conv(&v, "row6 tiny/zero/subnormal");
}

#[test]
fn cfg_conv_boundary() {
    let mut v: Vec<f64> = Vec::new();
    for anchor in [
        2147483647.0f64,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        2147483646.0,
        -2147483647.0,
    ] {
        v.push(anchor);
        v.push(anchor.next_up());
        v.push(anchor.next_down());
        v.push(anchor + 0.5);
        v.push(anchor - 0.5);
        v.push(anchor + 0.9999999);
        v.push(anchor - 0.9999999);
    }
    // Every double one ulp around the two exact bounds.
    let mut x = 2147483640.0f64;
    for _ in 0..200 {
        v.push(x);
        v.push(-x);
        x = x.next_up();
    }
    check_conv(&v, "row7 boundaries");
}

#[test]
fn cfg_conv_out_of_range() {
    let mut rng = Rng::new(0xAAAA_0004);
    let mut v: Vec<f64> = vec![
        2147483648.0,
        -2147483649.0,
        4294967296.0,
        -4294967296.0,
        1e18,
        -1e18,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
    ];
    for _ in 0..5000 {
        let e = 31 + (rng.below(280) as i32);
        let m = 1.0 + (rng.next_u64() as f64) / (u64::MAX as f64);
        let s = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        v.push(s * m * 2f64.powi(e.min(1020)));
    }
    check_conv(&v, "row8 out-of-range");
}

#[test]
fn cfg_conv_special() {
    let v: Vec<f64> = vec![
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xfff8_0000_0000_0001), // negative quiet NaN
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7ff7_ffff_ffff_ffff),
        f64::from_bits(0xffff_ffff_ffff_ffff),
    ];
    check_conv(&v, "row9 inf/NaN");
}

#[test]
fn cfg_conv_random_bits() {
    let mut rng = Rng::new(0xAAAA_0005);
    let mut v: Vec<f64> = Vec::with_capacity(400_000);
    for _ in 0..200_000 {
        v.push(f64::from_bits(rng.next_u64()));
        v.push(rng.spicy_f64());
    }
    check_conv(&v, "row10 random bit patterns");
}

// ---------------------------------------------------------------------------
// Rows 11-16 — create_numeric_buffer
// ---------------------------------------------------------------------------

/// Call both implementations over identically pre-filled buffers and compare
/// the full byte images (including bytes the call must leave untouched).
fn check_create(size: c_int, seed: c_int, alloc: usize, prefill: u8) {
    let p = apis();
    let mut bc = vec![prefill as i8; alloc];
    let mut br = vec![prefill as i8; alloc];
    unsafe {
        (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, size, seed);
        (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, size, seed);
    }
    assert_eq!(
        bc, br,
        "create_numeric_buffer(size={size}, seed={seed}, alloc={alloc})"
    );
}

#[test]
fn cfg_create_small_sizes() {
    for size in [0, 1, 2] {
        for seed in [0, 1, -1, 7, 255, 256, -256] {
            check_create(size, seed, 8, 0xCD);
        }
    }
}

#[test]
fn cfg_create_sizes() {
    for size in [3, 7, 8, 127, 128, 255, 256, 257, 511, 512, 1024, 4095, 65536, 1 << 20] {
        check_create(size, 0, size as usize + 16, 0xCD);
    }
}

#[test]
fn cfg_create_random() {
    let mut rng = Rng::new(0xBBBB_0001);
    for _ in 0..3000 {
        let size = 1 + rng.below(512) as c_int;
        let seed = rng.spicy_i32();
        let alloc = size as usize + rng.below(8) as usize;
        check_create(size, seed, alloc, (rng.next_u32() & 0xff) as u8);
    }
}

#[test]
fn cfg_create_overflow_seeds() {
    // `seed + i * 7` wraps mid-buffer for these seeds.
    for seed in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        i32::MAX - 700,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MIN + 700,
        -1,
        -7,
        -6,
    ] {
        for size in [1, 2, 101, 256, 512, 1024] {
            check_create(size, seed, size as usize + 4, 0x5A);
        }
    }
}

#[test]
fn cfg_create_negative_size() {
    let mut rng = Rng::new(0xBBBB_0002);
    let mut sizes: Vec<c_int> = vec![-1, -2, -7, -255, -256, i32::MIN, i32::MIN + 1];
    for _ in 0..500 {
        sizes.push(-(1 + rng.below(1 << 20) as c_int));
    }
    for size in sizes {
        check_create(size, rng.spicy_i32(), 32, 0xA5);
    }
}

// ---------------------------------------------------------------------------
// Rows 17-24 — find_value_in_buffer
// ---------------------------------------------------------------------------

fn check_find(buf: &[i8], size: usize, search_val: c_int, label: &str) {
    let p = apis();
    let (c, r) = unsafe {
        (
            (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, search_val),
            (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, search_val),
        )
    };
    assert_eq!(
        c, r,
        "{label}: find_value_in_buffer(size={size}, search_val={search_val}) C={c} Rust={r}"
    );
}

#[test]
fn cfg_find_positions() {
    let n = 64usize;
    for pos in [0usize, 1, 17, n / 2, n - 2, n - 1] {
        let mut buf = vec![0x11i8; n];
        buf[pos] = 0x7A;
        check_find(&buf, n, 0x7A, &format!("row17-19 needle at {pos}"));
        // Duplicates: memchr returns the FIRST match.
        let mut dup = vec![0x11i8; n];
        dup[pos] = 0x7A;
        if pos + 3 < n {
            dup[pos + 3] = 0x7A;
        }
        check_find(&dup, n, 0x7A, &format!("row18 first-of-duplicates at {pos}"));
    }
    // Row 20: absent.
    let buf = vec![0x11i8; n];
    check_find(&buf, n, 0x7A, "row20 absent");
}

#[test]
fn cfg_find_small_sizes() {
    let buf: Vec<i8> = vec![0x00, 0x2A, -0x01, 0x64, 0x7F, -0x80];
    for size in 0..=buf.len() {
        for sv in [0, 0x2A, 0xFF, 0x64, 0x7F, 0x80, -1, -128, 255, 256] {
            check_find(&buf, size, sv, &format!("row21 size={size}"));
        }
    }
}

#[test]
fn cfg_find_random() {
    let mut rng = Rng::new(0xCCCC_0001);
    for _ in 0..3000 {
        let n = 1 + rng.below(300) as usize;
        // Mix of dense (few distinct bytes -> frequent hits) and sparse buffers.
        let modulus = 1 + rng.below(256);
        let buf: Vec<i8> = (0..n)
            .map(|_| (rng.next_u32() as u64 % modulus) as u8 as i8)
            .collect();
        for _ in 0..6 {
            let sv = rng.spicy_i32();
            check_find(&buf, n, sv, "row22 random");
        }
        // Guaranteed hit at a random index.
        let idx = rng.below(n as u64) as usize;
        check_find(&buf, n, buf[idx] as c_int, "row22 guaranteed hit");
        // Sub-ranges: hit that falls outside the searched prefix.
        let prefix = rng.below(n as u64 + 1) as usize;
        check_find(&buf, prefix, buf[idx] as c_int, "row22 prefix");
    }
}

#[test]
fn cfg_find_aliasing() {
    // Every `search_val` sharing a low byte must behave identically.
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    let mut rng = Rng::new(0xCCCC_0002);
    for low in 0..256i32 {
        let mut variants = vec![low, low - 256, low + 256, low + 65536, low - 65536];
        for _ in 0..4 {
            let k = rng.next_i32() / 256;
            variants.push(k.wrapping_mul(256).wrapping_add(low));
        }
        for v in variants {
            check_find(&buf, 256, v, &format!("row23 alias low={low}"));
        }
    }
}

#[test]
fn cfg_find_over_generated_buffer() {
    // The exact composition `doubleneg` performs.
    let p = apis();
    let mut rng = Rng::new(0xCCCC_0003);
    for _ in 0..2000 {
        let seed = rng.spicy_i32();
        let mut bc = vec![0i8; 256];
        let mut br = vec![0i8; 256];
        unsafe {
            (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, 256, seed);
            (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, 256, seed);
        }
        assert_eq!(bc, br, "row24 generated buffer differs (seed={seed})");
        for sv in [42, 100, 0, 255, -1, rng.spicy_i32(), rng.spicy_i32()] {
            let (c, r) = unsafe {
                (
                    (p.c.find_value_in_buffer)(bc.as_ptr() as *const c_char, 256, sv),
                    (p.rust.find_value_in_buffer)(br.as_ptr() as *const c_char, 256, sv),
                )
            };
            assert_eq!(c, r, "row24 seed={seed} search_val={sv}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 25-31 — calculate_with_doubles
// ---------------------------------------------------------------------------

fn check_calc(a: c_int, b: c_int, c: c_int, label: &str) {
    let p = apis();
    let (rc, rr) = unsafe {
        (
            (p.c.calculate_with_doubles)(a, b, c),
            (p.rust.calculate_with_doubles)(a, b, c),
        )
    };
    assert_eq!(
        rc.to_bits(),
        rr.to_bits(),
        "{label}: calculate_with_doubles({a}, {b}, {c}) C={rc:?} ({:#018x}) Rust={rr:?} ({:#018x})",
        rc.to_bits(),
        rr.to_bits()
    );
}

#[test]
fn cfg_calc_b_zero_all_exponents() {
    let mut rng = Rng::new(0xDDDD_0001);
    for c in -9..=9 {
        for a in [0, 1, -1, 12345, -12345, i32::MAX, i32::MIN] {
            check_calc(a, 0, c, "row25 b==0");
        }
        for _ in 0..200 {
            check_calc(rng.spicy_i32(), 0, c, "row25 b==0 random a");
        }
    }
    // Also every `c` whose `c % 10` reaches those exponents.
    let mut rng = Rng::new(0xDDDD_0002);
    for _ in 0..2000 {
        check_calc(rng.spicy_i32(), 0, rng.spicy_i32(), "row25 b==0 random c");
    }
}

#[test]
fn cfg_calc_exponent_zero() {
    let mut rng = Rng::new(0xDDDD_0003);
    for c in [0, 10, -10, 20, -20, 100, -100, 2147483640, -2147483640] {
        for _ in 0..300 {
            let a = rng.spicy_i32();
            let b = loop {
                let b = rng.spicy_i32();
                if b != 0 {
                    break b;
                }
            };
            check_calc(a, b, c, "row26 exponent 0");
        }
    }
}

#[test]
fn cfg_calc_positive_exponents() {
    let mut rng = Rng::new(0xDDDD_0004);
    for e in 1..=9 {
        for _ in 0..500 {
            let a = rng.spicy_i32();
            let b = loop {
                let b = rng.spicy_i32();
                if b != 0 {
                    break b;
                }
            };
            // Several `c` values that all reduce to the same exponent.
            for k in [0i64, 1, 7, 100, 214748364] {
                let c = (k * 10 + e as i64).clamp(i32::MIN as i64, i32::MAX as i64) as c_int;
                check_calc(a, b, c, "row27 positive exponent");
            }
        }
    }
}

#[test]
fn cfg_calc_negative_exponents() {
    let mut rng = Rng::new(0xDDDD_0005);
    for e in 1..=9 {
        for _ in 0..500 {
            let a = rng.spicy_i32();
            let b = loop {
                let b = rng.spicy_i32();
                if b != 0 {
                    break b;
                }
            };
            for k in [0i64, 1, 7, 100, 214748364] {
                let c = (-(k * 10 + e as i64)).clamp(i32::MIN as i64, i32::MAX as i64) as c_int;
                check_calc(a, b, c, "row28 negative exponent");
            }
        }
    }
}

#[test]
fn cfg_calc_a_zero() {
    for b in [1, -1, 2, -2, i32::MAX, i32::MIN] {
        for c in -20..=20 {
            check_calc(0, b, c, "row29 a==0");
        }
    }
}

#[test]
fn cfg_calc_extremes() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &extremes {
        for &b in &extremes {
            for &c in &[
                i32::MIN,
                i32::MIN + 1,
                -11,
                -10,
                -9,
                -1,
                0,
                1,
                9,
                10,
                11,
                i32::MAX - 1,
                i32::MAX,
            ] {
                check_calc(a, b, c, "row30 extremes");
            }
        }
    }
}

#[test]
fn cfg_calc_random() {
    let mut rng = Rng::new(0xDDDD_0006);
    for _ in 0..200_000 {
        check_calc(
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            "row31 random",
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 32-43 — doubleneg (return value AND printed bytes)
//
// `doubleneg`'s printed bytes are part of its observable contract, and
// capturing them requires redirecting fd 1 process-wide. libtest writes its
// own progress lines to fd 1 from other threads, which would contaminate the
// capture, so those rows live in dedicated single-test binaries that cargo
// runs on their own:
//
//   * tests/doubleneg_valid.rs — CONFIGS.md rows 32-43
//   * tests/doubleneg_error.rs — ERRORS.md rows 21-24
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Row 44 — the composed pipeline, driven exactly as `doubleneg` does, with
// every intermediate compared. Catches divergences that the aggregate return
// value could cancel out.
// ---------------------------------------------------------------------------

#[test]
fn cfg_pipeline_random() {
    let p = apis();
    let mut rng = Rng::new(0xFFFF_0001);
    for _ in 0..20000 {
        let (p1, p2, p3, p4) = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );

        // Stage 1: the double computation and its (UB) narrowing.
        let dc = unsafe { (p.c.calculate_with_doubles)(p1, p2, p3) };
        let dr = unsafe { (p.rust.calculate_with_doubles)(p1, p2, p3) };
        assert_eq!(dc.to_bits(), dr.to_bits(), "pipeline calc ({p1},{p2},{p3})");
        let ic = unsafe { (p.c.convert_double_to_int)(dc) };
        let ir = unsafe { (p.rust.convert_double_to_int)(dr) };
        assert_eq!(ic, ir, "pipeline conv ({p1},{p2},{p3}) from {dc:?}");

        // Stage 2: buffer generation.
        let mut bc = vec![0i8; 256];
        let mut br = vec![0i8; 256];
        unsafe {
            (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, 256, p1);
            (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, 256, p1);
        }
        assert_eq!(bc, br, "pipeline buffer (seed={p1})");

        // Stage 3: the four searches doubleneg performs.
        let mut acc_c: i32 = 0;
        let mut acc_r: i32 = 0;
        for sv in [
            p2.wrapping_rem(256),
            p3.wrapping_rem(256),
            p4.wrapping_rem(256),
            42,
        ] {
            let fc = unsafe { (p.c.find_value_in_buffer)(bc.as_ptr() as *const c_char, 256, sv) };
            let fr =
                unsafe { (p.rust.find_value_in_buffer)(br.as_ptr() as *const c_char, 256, sv) };
            assert_eq!(fc, fr, "pipeline find (seed={p1}, sv={sv})");
            if fc >= 0 {
                acc_c = acc_c.wrapping_add(fc);
                acc_r = acc_r.wrapping_add(fr);
            }
        }

        // Stage 4: the combined-feature loop's negations.
        for i in 0..10i32 {
            let byte = p1.wrapping_add(i.wrapping_mul(p2)).wrapping_rem(256);
            let hc = unsafe { (p.c.find_value_in_buffer)(bc.as_ptr() as *const c_char, 256, byte) };
            let hr =
                unsafe { (p.rust.find_value_in_buffer)(br.as_ptr() as *const c_char, 256, byte) };
            assert_eq!(hc, hr, "pipeline combined find (i={i}, byte={byte})");
            let nc = unsafe { (p.c.process_negation)(if hc >= 0 { 1 } else { 0 }) };
            let nr = unsafe { (p.rust.process_negation)(if hr >= 0 { 1 } else { 0 }) };
            assert_eq!(nc, nr, "pipeline negation (i={i})");
            acc_c = acc_c.wrapping_add(nc);
            acc_r = acc_r.wrapping_add(nr);
        }
        assert_eq!(acc_c, acc_r, "pipeline accumulator ({p1},{p2},{p3},{p4})");
    }
}
