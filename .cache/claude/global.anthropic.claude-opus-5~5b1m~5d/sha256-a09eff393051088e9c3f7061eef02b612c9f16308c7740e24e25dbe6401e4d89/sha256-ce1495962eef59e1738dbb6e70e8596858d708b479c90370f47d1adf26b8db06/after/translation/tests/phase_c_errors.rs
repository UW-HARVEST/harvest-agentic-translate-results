//! Phase C — error-path / boundary differential tests. One test per ERRORS.md row.
//!
//! The C library has no error channel (no `return`, no sentinel, no errno, no
//! assert — see ERRORS.md), so "same rejection" is asserted as: identical
//! stdout bytes, identical byte counts, and identical non-aborting completion
//! for both `.so`s on every invalid / boundary input.

mod common;
use common::{assert_same, capture_stdout, libs, Rng, N, SEED};

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Rows 1-3: the only conditional in the C (`i < len` in `print_hex`).
///
/// `driver` always passes `sizeof(float)` == 4, so `len > 0` (row 1) is the only
/// reachable configuration; rows 2 and 3 (`len == 0`, `len < 0`) are
/// unreachable through the ABI. We assert the reachable consequence exactly:
/// each call emits exactly 4 hex byte-pairs plus a newline, from both libraries.
#[test]
fn row01_loop_guard_len_positive_emits_exactly_four_bytes() {
    let l = libs();
    for bits in [0x0000_0000u32, 0xffff_ffff, 0x7f80_0001, 0x0000_0001] {
        let c = capture_stdout(|| unsafe { (l.c_driver)(f(bits)) });
        let r = capture_stdout(|| unsafe { (l.rust_driver)(f(bits)) });
        assert_eq!(c, r, "row01: outputs differ for 0x{bits:08x}");
        assert_eq!(c.len(), 9, "row01: len(output) must be 4*2+1 for 0x{bits:08x}");
        assert_eq!(c[8], b'\n', "row01: must end with a newline");
        assert!(
            c[..8].iter().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "row01: expected 8 lowercase hex digits, got {:?}",
            String::from_utf8_lossy(&c)
        );
    }
}

#[test]
fn row02_row03_len_zero_and_negative_are_unreachable_via_abi() {
    // `print_hex` is `static` in the C source: `nm -D` on the C .so exports
    // only `driver`, so no caller can supply len <= 0. Assert that fact so the
    // row is verified rather than assumed.
    let l = libs();
    let c = std::fs::read(&l.c_path).expect("read C .so");
    // The symbol name exists in the C .so only as a *local* symbol; confirm the
    // dynamic symbol lookup fails for both libraries.
    assert!(!c.is_empty());
    unsafe {
        let cl = libloading::Library::new(&l.c_path).unwrap();
        let rl = libloading::Library::new(&l.rust_path).unwrap();
        let cs: Result<libloading::Symbol<unsafe extern "C" fn(*const u8, i32)>, _> =
            cl.get(b"print_hex\0");
        let rs: Result<libloading::Symbol<unsafe extern "C" fn(*const u8, i32)>, _> =
            rl.get(b"print_hex\0");
        assert!(cs.is_err(), "print_hex must NOT be dynamically exported by the C .so");
        assert!(
            rs.is_err(),
            "print_hex must NOT be exported by the Rust .so either (C has it `static`)"
        );
    }
}

/// Rows 4-6: the generic C-API boundary classes, asserted absent from the ABI.
///
/// `void driver(float)` takes no pointer, no length, and no enum/flag, so null
/// pointers, zero/oversized lengths and out-of-range enum values cannot be
/// expressed. We verify the ABI really is that narrow (a wider Rust signature
/// would be a divergence), and we verify the closest expressible analogue: an
/// arbitrary 32-bit garbage pattern in the single argument slot.
#[test]
fn row04_row05_row06_no_pointer_length_or_enum_parameters() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .filter(|l| l.contains('('))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "driver.h should declare exactly one function, found {decls:?}"
    );
    assert!(
        decls[0].contains("void driver(float"),
        "unexpected public declaration: {decls:?}"
    );
    assert!(!header.contains('*'), "public API must expose no pointer parameter");
    assert!(!header.contains("enum"), "public API must expose no enum parameter");

    // Closest expressible "invalid argument": garbage bits in the float slot.
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    let v: Vec<f32> = (0..N).map(|_| f(rng.next_u32())).collect();
    assert_same("row04-06 garbage argument bits", &v);
}

/// Row 7: `+0.0f` — the all-zero / "empty" boundary value.
#[test]
fn row07_positive_zero() {
    let l = libs();
    let c = capture_stdout(|| unsafe { (l.c_driver)(0.0f32) });
    let r = capture_stdout(|| unsafe { (l.rust_driver)(0.0f32) });
    assert_eq!(c, r, "row07: +0.0 differs");
    assert_eq!(c, b"00000000\n", "row07: got {:?}", String::from_utf8_lossy(&c));
}

/// Row 8: `-0.0f` — must NOT be flattened to `+0.0`.
#[test]
fn row08_negative_zero_sign_bit_preserved() {
    let l = libs();
    let c = capture_stdout(|| unsafe { (l.c_driver)(-0.0f32) });
    let r = capture_stdout(|| unsafe { (l.rust_driver)(-0.0f32) });
    assert_eq!(c, r, "row08: -0.0 differs");
    assert_eq!(c, b"00000080\n", "row08: got {:?}", String::from_utf8_lossy(&c));
}

/// Rows 9 & 10: infinities.
#[test]
fn row09_row10_infinities() {
    let l = libs();
    for (x, want) in [
        (f32::INFINITY, &b"0000807f\n"[..]),
        (f32::NEG_INFINITY, &b"000080ff\n"[..]),
    ] {
        let c = capture_stdout(|| unsafe { (l.c_driver)(x) });
        let r = capture_stdout(|| unsafe { (l.rust_driver)(x) });
        assert_eq!(c, r, "row09/10: {x} differs");
        assert_eq!(c, want, "row09/10: got {:?}", String::from_utf8_lossy(&c));
    }
}

/// Row 11: canonical quiet NaN.
#[test]
fn row11_quiet_nan() {
    let l = libs();
    let x = f(0x7fc0_0000);
    let c = capture_stdout(|| unsafe { (l.c_driver)(x) });
    let r = capture_stdout(|| unsafe { (l.rust_driver)(x) });
    assert_eq!(c, r, "row11: qNaN differs");
    assert_eq!(c, b"0000c07f\n", "row11: got {:?}", String::from_utf8_lossy(&c));
}

/// Row 12: signalling NaN — the payload must not be quietened by the Rust
/// `extern "C"` wrapper.
#[test]
fn row12_signalling_nan_not_quietened() {
    let l = libs();
    let x = f(0x7f80_0001);
    let c = capture_stdout(|| unsafe { (l.c_driver)(x) });
    let r = capture_stdout(|| unsafe { (l.rust_driver)(x) });
    assert_eq!(
        c,
        r,
        "row12: sNaN differs (Rust quietened the payload?) C={:?} RUST={:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert_eq!(c, b"0100807f\n", "row12: got {:?}", String::from_utf8_lossy(&c));
}

/// Row 13: NaNs with arbitrary payloads, both signs, quiet and signalling,
/// exhaustively over the payload's high bits and randomized over the rest.
#[test]
fn row13_arbitrary_nan_payloads() {
    let mut rng = Rng::new(SEED ^ 0x0A0A_u64);
    let mut v = Vec::new();
    for sign in 0..2u32 {
        // every high-7-bit mantissa pattern (quiet and signalling), random low bits
        for hi in 0..128u32 {
            let mut mant = (hi << 16) | (rng.next_u32() & 0xffff);
            if mant == 0 {
                mant = 1; // keep it a NaN, not an infinity
            }
            v.push(f((sign << 31) | (0xffu32 << 23) | mant));
        }
    }
    // plus a randomized sweep
    for _ in 0..N {
        let sign = rng.next_u32() >> 31;
        let mut mant = rng.next_u32() & 0x007f_ffff;
        if mant == 0 {
            mant = 1;
        }
        v.push(f((sign << 31) | (0xffu32 << 23) | mant));
    }
    assert_same("row13 arbitrary NaN payloads", &v);
}

/// Row 14: subnormal extremes — no denormal flush-to-zero anywhere.
#[test]
fn row14_subnormal_extremes_no_ftz() {
    let l = libs();
    for (bits, want) in [
        (0x0000_0001u32, &b"01000000\n"[..]), // smallest positive subnormal
        (0x8000_0001, &b"01000080\n"[..]),    // smallest negative subnormal
        (0x007f_ffff, &b"ffff7f00\n"[..]),    // largest positive subnormal
        (0x807f_ffff, &b"ffff7f80\n"[..]),    // largest negative subnormal
    ] {
        let x = f(bits);
        let c = capture_stdout(|| unsafe { (l.c_driver)(x) });
        let r = capture_stdout(|| unsafe { (l.rust_driver)(x) });
        assert_eq!(c, r, "row14: 0x{bits:08x} differs");
        assert_eq!(
            c,
            want,
            "row14: 0x{bits:08x} -> {:?}",
            String::from_utf8_lossy(&c)
        );
    }
}

/// Row 15: documented range endpoints FLT_MIN / FLT_MAX.
#[test]
fn row15_flt_min_flt_max() {
    let l = libs();
    for (x, want) in [
        (f32::MIN_POSITIVE, &b"00008000\n"[..]),
        (-f32::MIN_POSITIVE, &b"00008080\n"[..]),
        (f32::MAX, &b"ffff7f7f\n"[..]),
        (f32::MIN, &b"ffff7fff\n"[..]),
    ] {
        let c = capture_stdout(|| unsafe { (l.c_driver)(x) });
        let r = capture_stdout(|| unsafe { (l.rust_driver)(x) });
        assert_eq!(c, r, "row15: {x:e} differs");
        assert_eq!(c, want, "row15: {x:e} -> {:?}", String::from_utf8_lossy(&c));
    }
}

/// Row 16: one step past each documented endpoint (`nextafter` neighbours).
#[test]
fn row16_one_step_past_each_endpoint() {
    let mut v = Vec::new();
    // one ULP past FLT_MAX in the direction of +inf, and its negative twin
    v.push(f32::MAX.next_up()); // == +inf
    v.push(f32::MIN.next_down()); // == -inf
    // one ULP below FLT_MAX / above FLT_MIN etc.
    v.push(f32::MAX.next_down());
    v.push(f32::MIN_POSITIVE.next_down()); // largest subnormal
    v.push(f32::MIN_POSITIVE.next_up());
    v.push((0.0f32).next_up()); // smallest subnormal
    v.push((-0.0f32).next_down());
    v.push((0.0f32).next_down()); // -0.0's neighbour
    // one step past +/-inf stays inf; past the largest finite in each direction
    v.push(f32::INFINITY.next_down()); // FLT_MAX
    v.push(f32::NEG_INFINITY.next_up()); // -FLT_MAX
    // neighbours of 1.0 and -1.0
    v.push(1.0f32.next_up());
    v.push(1.0f32.next_down());
    v.push((-1.0f32).next_up());
    v.push((-1.0f32).next_down());
    assert_same("row16 one step past endpoints", &v);
}

/// Row 17: every one of the 2^32 patterns is legal input; sweep broadly,
/// including a systematic walk of the exponent field for both signs.
#[test]
fn row17_arbitrary_32bit_patterns() {
    // systematic: every exponent value, both signs, random mantissa
    let mut rng = Rng::new(SEED ^ 0xFEED);
    let mut v = Vec::new();
    for sign in 0..2u32 {
        for exp in 0..256u32 {
            v.push(f((sign << 31) | (exp << 23) | (rng.next_u32() & 0x007f_ffff)));
            v.push(f((sign << 31) | (exp << 23))); // zero mantissa
            v.push(f((sign << 31) | (exp << 23) | 0x007f_ffff)); // full mantissa
        }
    }
    assert_same("row17 exponent sweep", &v);

    // uniform random garbage
    for batch in 0..4 {
        let v: Vec<f32> = (0..8192).map(|_| f(rng.next_u32())).collect();
        assert_same(&format!("row17 uniform garbage batch {batch}"), &v);
    }
}

/// Row 18: repeated / interleaved calls — no hidden state, no init requirement.
#[test]
fn row18_repeated_calls_no_state() {
    let l = libs();
    // Same input many times must give identical repeated records for both libs.
    let x = f(0xdead_beef);
    let c = capture_stdout(|| {
        for _ in 0..1000 {
            unsafe { (l.c_driver)(x) }
        }
    });
    let r = capture_stdout(|| {
        for _ in 0..1000 {
            unsafe { (l.rust_driver)(x) }
        }
    });
    assert_eq!(c, r, "row18: repeated-call output differs");
    assert_eq!(c.len(), 9000, "row18: expected 9000 bytes");
    let first = &c[..9];
    for (i, rec) in c.chunks(9).enumerate() {
        assert_eq!(rec, first, "row18: record {i} differs from the first");
    }

    // No initialisation call exists, and calling with different values in any
    // order must not carry state between calls.
    let mut rng = Rng::new(SEED ^ 0x1234);
    let vals: Vec<f32> = (0..512).map(|_| f(rng.next_u32())).collect();
    let mut reversed = vals.clone();
    reversed.reverse();
    let (c1, r1) = common::run_both(&vals);
    let (c2, r2) = common::run_both(&reversed);
    assert_eq!(c1, r1, "row18: forward order differs");
    assert_eq!(c2, r2, "row18: reverse order differs");
    let mut fwd: Vec<&[u8]> = c1.chunks(9).collect();
    let rev: Vec<&[u8]> = c2.chunks(9).collect();
    fwd.reverse();
    assert_eq!(fwd, rev, "row18: order-dependent output detected");
}
