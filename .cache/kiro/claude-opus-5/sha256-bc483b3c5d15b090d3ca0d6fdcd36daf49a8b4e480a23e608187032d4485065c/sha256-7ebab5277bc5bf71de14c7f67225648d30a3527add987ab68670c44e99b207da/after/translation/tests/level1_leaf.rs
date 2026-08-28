//! Level 1: leaf functions with no callees inside the library.
//! `safe_double_to_int`, `process_with_fallthrough`, `handle_pointer_operations`,
//! `copy_data_block`.

mod common;

use common::*;
use std::ffi::{c_double, c_int, c_void};

/// Interesting doubles: the exact `INT_MAX` / `INT_MIN` boundaries and their
/// nearest neighbours, NaNs (both signs, quiet + payload), infinities, signed
/// zeros, subnormals, and plain truncation cases.
fn double_cases() -> Vec<c_double> {
    let mut v: Vec<c_double> = vec![
        0.0,
        -0.0,
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
        42.0,
        -42.0,
        123.456,
        -123.456,
        1e15,
        -1e15,
        1e300,
        -1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
    ];

    // Exact integer boundaries and neighbours.
    let imax = c_int::MAX as c_double; // 2147483647.0, exactly representable
    let imin = c_int::MIN as c_double; // -2147483648.0, exactly representable
    for base in [imax, imin] {
        v.push(base);
        v.push(base.next_up());
        v.push(base.next_down());
        v.push(base + 0.5);
        v.push(base - 0.5);
        v.push(base + 1.0);
        v.push(base - 1.0);
        v.push(base + 2.0);
        v.push(base - 2.0);
    }

    // A NaN with a non-default payload, plus a signalling-style NaN bit pattern.
    v.push(f64::from_bits(0x7FF8_0000_DEAD_BEEF));
    v.push(f64::from_bits(0xFFF8_0000_0000_0001));
    v.push(f64::from_bits(0x7FF0_0000_0000_0001)); // sNaN
    v.push(f64::from_bits(0xFFF0_0000_0000_0001));

    // Deterministic pseudo-random spread across many magnitudes.
    let mut state: u64 = 0x243F_6A88_85A3_08D3;
    for _ in 0..400 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mantissa = ((state >> 11) as f64) / ((1u64 << 53) as f64); // [0,1)
        let exp = ((state >> 3) % 80) as i32 - 40;
        let sign = if state & 1 == 0 { 1.0 } else { -1.0 };
        v.push(sign * mantissa * 2f64.powi(exp));
        // Also sweep tightly around the int range where the branches live.
        v.push(sign * mantissa * 4.294_967_296e9);
    }
    v
}

fn safe_double_to_int_matches() {
    let im = impls();
    let cf = im.c_sym::<FnSafeDoubleToInt>("safe_double_to_int");
    let rf = im.rust_sym::<FnSafeDoubleToInt>("safe_double_to_int");

    for d in double_cases() {
        let (c_ret, c_out) = capture_stdout(|| unsafe { cf(d) });
        let (r_ret, r_out) = capture_stdout(|| unsafe { rf(d) });
        assert_eq!(
            c_ret, r_ret,
            "safe_double_to_int({d:?}) [bits {:#018x}]: C={c_ret} Rust={r_ret}",
            d.to_bits()
        );
        assert_eq!(c_out, r_out, "stdout differs for safe_double_to_int({d:?})");
    }
}

fn process_with_fallthrough_matches() {
    let im = impls();
    let cf = im.c_sym::<FnProcessWithFallthrough>("process_with_fallthrough");
    let rf = im.rust_sym::<FnProcessWithFallthrough>("process_with_fallthrough");

    let mut codes: Vec<c_int> = (-20..=20).collect();
    codes.extend([c_int::MIN, c_int::MIN + 1, c_int::MAX, c_int::MAX - 1, 100, -100]);

    // Base values chosen to make every fall-through arm overflow if the
    // implementations disagree about wrapping.
    let mut bases: Vec<c_int> = vec![
        0,
        1,
        -1,
        7,
        -7,
        1000,
        -1000,
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX - 9,
        c_int::MAX - 10,
        c_int::MAX - 11,
        c_int::MAX - 149,
        c_int::MAX - 150,
        c_int::MAX - 151,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 150,
    ];
    let mut state: u32 = 0x9E37_79B9;
    for _ in 0..64 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        bases.push(state as c_int);
    }

    for &code in &codes {
        for &base in &bases {
            let (c_ret, c_out) = capture_stdout(|| unsafe { cf(code, base) });
            let (r_ret, r_out) = capture_stdout(|| unsafe { rf(code, base) });
            assert_eq!(
                c_ret, r_ret,
                "process_with_fallthrough({code}, {base}): C={c_ret} Rust={r_ret}"
            );
            assert_eq!(c_out, r_out, "stdout differs for ({code}, {base})");
        }
    }
}

fn handle_pointer_operations_matches() {
    let im = impls();
    let cf = im.c_sym::<FnHandlePointerOperations>("handle_pointer_operations");
    let rf = im.rust_sym::<FnHandlePointerOperations>("handle_pointer_operations");

    let mut vals: Vec<c_int> = (-40..=40).collect();
    vals.extend([
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN / 2,
        c_int::MIN / 2 - 1,
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX / 2,
        c_int::MAX / 2 + 1,
        1_073_741_774, // *2 + 100 lands exactly on INT_MAX-ish territory
        1_073_741_824,
    ]);
    let mut state: u32 = 0x1234_5678;
    for _ in 0..128 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        vals.push(state as c_int);
    }

    for &v in &vals {
        let (c_ret, c_out) = capture_stdout(|| unsafe { cf(v) });
        let (r_ret, r_out) = capture_stdout(|| unsafe { rf(v) });
        assert_eq!(
            c_ret, r_ret,
            "handle_pointer_operations({v}): C={c_ret} Rust={r_ret}"
        );
        assert_eq!(c_out, r_out, "stdout differs for handle_pointer_operations({v})");
    }
}

fn copy_data_block_matches() {
    let im = impls();
    let cf = im.c_sym::<FnCopyDataBlock>("copy_data_block");
    let rf = im.rust_sym::<FnCopyDataBlock>("copy_data_block");

    let cases: Vec<(c_int, c_double, &[u8])> = vec![
        (0, 0.0, b""),
        (1, 1.5, b"Source"),
        (-1, -0.0, b"abc"),
        (c_int::MAX, f64::MAX, b"0123456789012345678"),
        (c_int::MIN, f64::MIN, b"0123456789012345678"),
        (7, f64::NAN, b"nan-label"),
        (8, f64::INFINITY, b"inf"),
        (-9, f64::NEG_INFINITY, b"-inf"),
        (12345, 1e-300, b"tiny"),
    ];

    for (fill_src, fill_dst) in [(0x00u8, 0xAAu8), (0xFF, 0x55), (0x5A, 0xA5)] {
        for &(id, value, label) in &cases {
            let src = make_block_bytes(id, value, label, fill_src);

            let mut c_dst = [fill_dst; BLOCK_SCRATCH];
            let mut r_dst = [fill_dst; BLOCK_SCRATCH];

            let (_, c_out) = capture_stdout(|| unsafe {
                cf(c_dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void)
            });
            let (_, r_out) = capture_stdout(|| unsafe {
                rf(r_dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void)
            });

            assert_eq!(
                c_dst, r_dst,
                "copy_data_block byte image differs (id={id}, value={value:?}, label={:?})\n C: {:02x?}\n R: {:02x?}",
                String::from_utf8_lossy(label),
                c_dst,
                r_dst
            );
            assert_eq!(c_out, r_out, "stdout differs for copy_data_block");
        }
    }
}

/// Single entry point: see `capture_stdout` for why each test binary must
/// contain exactly one `#[test]`.
#[test]
fn leaf_functions_match_c() {
    safe_double_to_int_matches();
    process_with_fallthrough_matches();
    handle_pointer_operations_matches();
    copy_data_block_matches();
}
