//! Phase C — error-path differential tests, GATED on `ERRORS.md`.
//!
//! `ERRORS.md` records that the C library's error surface is EMPTY: no
//! `return`, no error enum, no `assert`, no `errno`, no NULL check, no range
//! check — both functions return `void`. The differential assertion for every
//! row is therefore "both `.so`s accept the input, return normally, and emit
//! byte-identical output", plus, where `ERRORS.md` names an exact expected
//! string, that the emitted bytes equal it. That is the same error/rejection
//! outcome on both sides (namely: no rejection), not merely "both failed
//! somehow".

mod common;
use common::*;

/// Calls BOTH `.so`s with `bits` and asserts identical stdout plus the exact
/// expected byte string.
fn assert_exact(row: &str, bits: u32, expect: &str) {
    let api = api();
    let x = f32::from_bits(bits);
    let c = capture(|| unsafe { (api.c_driver)(x) });
    let r = capture(|| unsafe { (api.r_driver)(x) });
    assert_eq!(
        c,
        r,
        "{row}: 0x{bits:08x}: C {:?} != Rust {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(
        c,
        expect.as_bytes(),
        "{row}: 0x{bits:08x}: got {:?}, ERRORS.md expects {expect:?}",
        String::from_utf8_lossy(&c)
    );
}

// --- Row 1 -----------------------------------------------------------------
#[test]
fn err_row01_quiet_nan() {
    assert_exact("err_row01", 0x7fc0_0000, "0000c07f\n");
}

// --- Row 2 -----------------------------------------------------------------
#[test]
fn err_row02_negative_quiet_nan() {
    assert_exact("err_row02", 0xffc0_0000, "0000c0ff\n");
}

// --- Row 3 -----------------------------------------------------------------
#[test]
fn err_row03_signalling_nan() {
    assert_exact("err_row03", 0x7fa0_0000, "0000a07f\n");
    // Signalling NaN is passed in xmm0 and never used as a number, so no
    // invalid-operation trap and no quieting on either side.
    assert_exact("err_row03", 0xffa0_0000, "0000a0ff\n");
}

// --- Row 4 -----------------------------------------------------------------
#[test]
fn err_row04_nan_payload_preserved() {
    // The payload must NOT be canonicalised on either side.
    assert_exact("err_row04", 0x7fff_ffff, "ffffff7f\n");
    assert_exact("err_row04", 0x7f80_0001, "0100807f\n");
    assert_exact("err_row04", 0xffbf_ffff, "ffffbfff\n");
    // A broad randomized sweep of NaN payloads.
    let mut rng = Rng::new(104);
    let inputs: Vec<u32> = (0..2000)
        .map(|i| {
            let frac = 1 + (rng.next_u32() % 0x007f_ffff);
            (((i & 1) as u32) << 31) | (0xff << 23) | frac
        })
        .collect();
    for &b in &inputs {
        assert!(f32::from_bits(b).is_nan());
    }
    run_batch("err_row04_nan_payload_preserved", &inputs);
}

// --- Row 5 / 6 -------------------------------------------------------------
#[test]
fn err_row05_pos_infinity() {
    assert_exact("err_row05", 0x7f80_0000, "0000807f\n");
}

#[test]
fn err_row06_neg_infinity() {
    assert_exact("err_row06", 0xff80_0000, "000080ff\n");
}

// --- Row 7 -----------------------------------------------------------------
#[test]
fn err_row07_negative_zero() {
    // -0.0 == +0.0 by value comparison, so a value-based translation diverges
    // here; only a representation-based one matches.
    assert_exact("err_row07", 0x8000_0000, "00000080\n");
    assert_exact("err_row07", 0x0000_0000, "00000000\n");
    let api = api();
    let c_neg = capture(|| unsafe { (api.c_driver)(-0.0f32) });
    let c_pos = capture(|| unsafe { (api.c_driver)(0.0f32) });
    let r_neg = capture(|| unsafe { (api.r_driver)(-0.0f32) });
    let r_pos = capture(|| unsafe { (api.r_driver)(0.0f32) });
    assert_ne!(c_neg, c_pos, "C must distinguish -0.0 from +0.0");
    assert_eq!(c_neg, r_neg);
    assert_eq!(c_pos, r_pos);
}

// --- Row 8 -----------------------------------------------------------------
#[test]
fn err_row08_subnormals() {
    // No flush-to-zero on either side.
    assert_exact("err_row08", 0x0000_0001, "01000000\n");
    assert_exact("err_row08", 0x007f_ffff, "ffff7f00\n");
    assert_exact("err_row08", 0x8000_0001, "01000080\n");
    assert_exact("err_row08", 0x807f_ffff, "ffff7f80\n");
    let mut rng = Rng::new(108);
    let inputs: Vec<u32> = (0..2000)
        .map(|i| (((i & 1) as u32) << 31) | (1 + rng.next_u32() % 0x007f_ffff))
        .collect();
    for &b in &inputs {
        assert!(f32::from_bits(b).is_subnormal());
    }
    run_batch("err_row08_subnormals", &inputs);
}

// --- Row 9 -----------------------------------------------------------------
#[test]
fn err_row09_flt_limits() {
    assert_exact("err_row09 FLT_MAX", 0x7f7f_ffff, "ffff7f7f\n");
    assert_exact("err_row09 -FLT_MAX", 0xff7f_ffff, "ffff7fff\n");
    assert_exact("err_row09 FLT_MIN", 0x0080_0000, "00008000\n");
    assert_exact("err_row09 -FLT_MIN", 0x8080_0000, "00008080\n");
    assert_exact("err_row09 FLT_EPSILON", 0x3400_0000, "00000034\n");
    // One representable step past each limit, in both directions.
    for b in [
        0x7f7f_fffeu32,
        0x7f80_0000,
        0x007f_ffff,
        0x0080_0001,
        0x33ff_ffff,
        0x3400_0001,
    ] {
        let api = api();
        let x = f32::from_bits(b);
        let c = capture(|| unsafe { (api.c_driver)(x) });
        let r = capture(|| unsafe { (api.r_driver)(x) });
        assert_eq!(c, r, "err_row09: divergence at 0x{b:08x}");
        assert_eq!(c, expected_line(b));
    }
}

// --- Row 10 ----------------------------------------------------------------
#[test]
fn err_row10_zero_pad_low_bytes() {
    // A missing `%02x` zero-pad would silently shorten the line.
    assert_exact("err_row10", 0x0101_0101, "01010101\n");
    assert_exact("err_row10", 0x0000_0000, "00000000\n");
    assert_exact("err_row10", 0x0f0f_0f0f, "0f0f0f0f\n");
    let mut rng = Rng::new(110);
    let inputs: Vec<u32> = (0..2000).map(|_| rng.next_u32() & 0x0f0f_0f0f).collect();
    run_batch("err_row10_zero_pad_low_bytes", &inputs);
}

// --- Row 11 ----------------------------------------------------------------
#[test]
fn err_row11_high_bytes_no_sign_extend() {
    // Sign-extending the `unsigned char` -> `int` promotion would print
    // `ffffff80` per byte, i.e. 32 chars per line instead of 8.
    assert_exact("err_row11", 0x8080_8080, "80808080\n");
    assert_exact("err_row11", 0xffff_ffff, "ffffffff\n");
    let mut rng = Rng::new(111);
    let inputs: Vec<u32> = (0..2000).map(|_| rng.next_u32() | 0x8080_8080).collect();
    run_batch("err_row11_high_bytes_no_sign_extend", &inputs);
    // Every byte value >= 0x80 individually, at every position.
    let mut per_byte = Vec::new();
    for pos in 0..4u32 {
        for b in 0x80..=0xffu32 {
            per_byte.push(b << (8 * pos));
        }
    }
    run_batch("err_row11_high_bytes_each_position", &per_byte);
}

// --- Row 12 ----------------------------------------------------------------
#[test]
fn err_row12_output_length_always_9() {
    // `print_hex`'s `len` is hard-wired to sizeof(float) == 4 by its only
    // caller, so the output is invariably 8 hex chars + '\n'.
    let api = api();
    let mut rng = Rng::new(112);
    for _ in 0..500 {
        let b = rng.next_u32();
        let x = f32::from_bits(b);
        let c = capture(|| unsafe { (api.c_driver)(x) });
        let r = capture(|| unsafe { (api.r_driver)(x) });
        assert_eq!(c.len(), 9, "C emitted {} bytes for 0x{b:08x}", c.len());
        assert_eq!(r.len(), 9, "Rust emitted {} bytes for 0x{b:08x}", r.len());
        assert_eq!(c, r);
    }
    assert_eq!(std::mem::size_of::<f32>(), 4);
}

// --- Row 13 ----------------------------------------------------------------
#[test]
fn err_row13_no_pointer_parameter() {
    // The public ABI is `void driver(float)`: there is no pointer parameter,
    // so a null-pointer rejection cannot be constructed. Assert this
    // structurally, then confirm the internal `print_hex` is not reachable
    // through either `.so` (so its `p` can never be supplied by a caller).
    let libs: [&str; 2] = ["c", "rust"];
    for which in libs {
        let path = if which == "c" {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("c_src/build/libdriver.so")
        } else {
            let mut p = std::env::current_exe().unwrap();
            p.pop();
            if p.file_name().and_then(|s| s.to_str()) == Some("deps") {
                p.pop();
            }
            p.join("libdriver.so")
        };
        let lib = unsafe { libloading::Library::new(&path) }.unwrap();
        let sym: Result<libloading::Symbol<unsafe extern "C" fn()>, _> =
            unsafe { lib.get(b"print_hex\0") };
        assert!(
            sym.is_err(),
            "{which} .so unexpectedly exports the static function `print_hex`"
        );
    }
    // And `driver` itself is callable with any float, needing no pointer.
    assert_exact("err_row13", 0x3f80_0000, "0000803f\n");
}

// --- Row 14 ----------------------------------------------------------------
#[test]
fn err_row14_all_bit_patterns_are_valid() {
    // The C declares no enum / flag / mode parameter, so there is no enum
    // value to push out of range. The equivalent boundary is that every one of
    // the 2^32 argument bit patterns is a valid input; sweep it densely, and
    // include the patterns an out-of-range `int` enum would have produced if
    // reinterpreted through this ABI.
    let mut inputs: Vec<u32> = vec![
        0x0000_0000,
        0x0000_0001,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0xdead_beef,
        0xcafe_babe,
        // values a bogus C enum argument would carry
        (-1i32) as u32,
        (i32::MIN) as u32,
        (i32::MAX) as u32,
        999_999u32,
        0xffff_fff0,
    ];
    let mut rng = Rng::new(114);
    inputs.extend((0..8000).map(|_| rng.next_u32()));
    // Dense sweep of the top byte (exponent+sign region) with random low bits.
    for hi in 0..=255u32 {
        inputs.push((hi << 24) | (rng.next_u32() & 0x00ff_ffff));
    }
    run_batch("err_row14_all_bit_patterns_are_valid", &inputs);
}

// --- Row 15 ----------------------------------------------------------------
#[test]
fn err_row15_shared_stdout_no_state() {
    // The only global state touched is the process-wide stdout FILE stream.
    // N calls must emit exactly N 9-byte lines, in order, with nothing carried
    // between calls, and calling one library must not disturb the other.
    let api = api();
    let mut rng = Rng::new(115);
    let inputs: Vec<u32> = (0..300).map(|_| rng.next_u32()).collect();

    // Same batch, run twice through each library: identical both times.
    for pass in 0..2 {
        let c = capture(|| {
            for &b in &inputs {
                unsafe { (api.c_driver)(f32::from_bits(b)) };
            }
        });
        let r = capture(|| {
            for &b in &inputs {
                unsafe { (api.r_driver)(f32::from_bits(b)) };
            }
        });
        assert_eq!(c, r, "pass {pass}: C/Rust divergence");
        assert_eq!(c.len(), inputs.len() * 9);
        for (i, &b) in inputs.iter().enumerate() {
            assert_eq!(&c[i * 9..i * 9 + 9], &expected_line(b)[..]);
        }
    }

    // Cross-library interleave in a single window must not corrupt either side.
    let out = capture(|| {
        for &b in inputs.iter().take(50) {
            unsafe { (api.r_driver)(f32::from_bits(b)) };
            unsafe { (api.c_driver)(f32::from_bits(b)) };
            unsafe { (api.c_driver)(f32::from_bits(b)) };
            unsafe { (api.r_driver)(f32::from_bits(b)) };
        }
    });
    assert_eq!(out.len(), 50 * 4 * 9);
    for (i, &b) in inputs.iter().take(50).enumerate() {
        for k in 0..4 {
            let lo = (i * 4 + k) * 9;
            assert_eq!(
                &out[lo..lo + 9],
                &expected_line(b)[..],
                "interleave line {} for 0x{b:08x}",
                i * 4 + k
            );
        }
    }
}
