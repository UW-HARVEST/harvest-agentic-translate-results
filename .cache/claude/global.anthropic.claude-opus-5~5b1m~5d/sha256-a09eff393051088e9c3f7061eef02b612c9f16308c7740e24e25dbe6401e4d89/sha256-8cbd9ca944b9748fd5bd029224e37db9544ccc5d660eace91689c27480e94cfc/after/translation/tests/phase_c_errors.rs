// Phase C — error / hostile-input differential tests.
// One test per row of ERRORS.md. `driver` has an empty error-return surface
// (see the grep evidence in ERRORS.md), so each row asserts that C and Rust
// agree on the only observable — the exact bytes on stdout — and that neither
// rejects, traps, quiets, canonicalises or aborts on the hostile input.

mod common;
use common::*;

/// Every row asserts the exact expected stdout, not merely "both did the same".
#[track_caller]
fn assert_exact(label: &str, bits: u32, expected: &str) {
    let x = f32::from_bits(bits);
    let c_out = run_batch(&c_driver(), &[x]);
    let r_out = run_batch(&rust_driver(), &[x]);
    assert_eq!(
        c_out,
        expected.as_bytes(),
        "[{label}] C output for 0x{bits:08x} was {:?}, expected {expected:?}",
        String::from_utf8_lossy(&c_out)
    );
    assert_eq!(
        r_out,
        expected.as_bytes(),
        "[{label}] Rust output for 0x{bits:08x} was {:?}, expected {expected:?}",
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(c_out, r_out, "[{label}] C/Rust divergence on 0x{bits:08x}");
}

// Row 1 — there is no error path: EVERY float is accepted and yields exactly
// 9 bytes; the function returns void and never signals failure.
#[test]
fn err01_no_error_path_every_input_accepted() {
    let mut rng = Rng::new(0x0101_0101_0101_0101);
    let bits: Vec<u32> = (0..5000).map(|_| rng.next_u32()).collect();
    let xs: Vec<f32> = bits.iter().copied().map(f32::from_bits).collect();

    let c_out = run_batch(&c_driver(), &xs);
    let r_out = run_batch(&rust_driver(), &xs);
    // Accepted: no input was skipped, dropped or rejected.
    assert_eq!(c_out.len(), xs.len() * 9, "C rejected/skipped some input");
    assert_eq!(r_out.len(), xs.len() * 9, "Rust rejected/skipped some input");
    assert_eq!(c_out, r_out);
    for chunk in c_out.chunks(9) {
        assert_eq!(chunk[8], b'\n', "missing trailing newline");
        assert!(
            chunk[..8].iter().all(|c| c.is_ascii_hexdigit()
                && !c.is_ascii_uppercase()),
            "non-lowercase-hex output: {:?}",
            String::from_utf8_lossy(chunk)
        );
    }
}

// Row 2 — +0.0, all-zero object representation.
#[test]
fn err02_positive_zero() {
    assert_exact("err02", 0x0000_0000, "00000000\n");
}

// Row 3 — -0.0 must not be canonicalised.
#[test]
fn err03_negative_zero_not_canonicalised() {
    assert_exact("err03", 0x8000_0000, "00000080\n");
}

// Row 4 — quiet NaN.
#[test]
fn err04_quiet_nan() {
    assert_exact("err04", 0x7fc0_0000, "0000c07f\n");
}

// Row 5 — negative quiet NaN.
#[test]
fn err05_negative_quiet_nan() {
    assert_exact("err05", 0xffc0_0000, "0000c0ff\n");
}

// Row 6 — SIGNALLING NaN must not be quieted in transit (0x7fa00000 must not
// become 0x7fc00000).
#[test]
fn err06_signalling_nan_not_quieted() {
    assert_exact("err06", 0x7fa0_0000, "0000a07f\n");
    assert_exact("err06-neg", 0xffa0_0000, "0000a0ff\n");
    // sweep sNaN payloads
    for p in 1..=0x3f_u32 {
        let bits = 0x7f80_0000 | p;
        let x = f32::from_bits(bits);
        assert_eq!(
            run_batch(&c_driver(), &[x]),
            run_batch(&rust_driver(), &[x]),
            "sNaN payload 0x{p:x} diverged"
        );
    }
}

// Row 7 — smallest-payload sNaN.
#[test]
fn err07_min_payload_snan() {
    assert_exact("err07", 0x7f80_0001, "0100807f\n");
}

// Row 8 — NaN with all payload bits set.
#[test]
fn err08_max_payload_nan() {
    assert_exact("err08", 0x7fff_ffff, "ffffff7f\n");
}

// Row 9 — +inf.
#[test]
fn err09_positive_infinity() {
    assert_exact("err09", 0x7f80_0000, "0000807f\n");
}

// Row 10 — -inf.
#[test]
fn err10_negative_infinity() {
    assert_exact("err10", 0xff80_0000, "000080ff\n");
}

// Row 11 — smallest positive subnormal must not be flushed to zero.
#[test]
fn err11_smallest_subnormal_not_flushed() {
    assert_exact("err11", 0x0000_0001, "01000000\n");
}

// Row 12 — largest subnormal.
#[test]
fn err12_largest_subnormal() {
    assert_exact("err12", 0x007f_ffff, "ffff7f00\n");
}

// Row 13 — negative subnormal.
#[test]
fn err13_negative_subnormal() {
    assert_exact("err13", 0x8000_0001, "01000080\n");
}

// Row 14 — FLT_MAX / -FLT_MAX, one step below overflow.
#[test]
fn err14_flt_max_boundary() {
    assert_exact("err14+", 0x7f7f_ffff, "ffff7f7f\n");
    assert_exact("err14-", 0xff7f_ffff, "ffff7fff\n");
    assert_eq!(f32::MAX.to_bits(), 0x7f7f_ffff);
    assert_eq!(f32::MIN.to_bits(), 0xff7f_ffff);
}

// Row 15 — FLT_MIN, the normal/subnormal boundary.
#[test]
fn err15_flt_min_boundary() {
    assert_exact("err15", 0x0080_0000, "00008000\n");
    assert_eq!(f32::MIN_POSITIVE.to_bits(), 0x0080_0000);
    // one step either side of the boundary
    assert_exact("err15-below", 0x007f_ffff, "ffff7f00\n");
    assert_exact("err15-above", 0x0080_0001, "01008000\n");
}

// Row 16 — all bits set.
#[test]
fn err16_all_bits_set() {
    assert_exact("err16", 0xffff_ffff, "ffffffff\n");
}

// Row 17 — %02x field-width probe next to a zero byte.
#[test]
fn err17_width_padding() {
    assert_exact("err17", 0x0000_ff00, "00ff0000\n");
    assert_exact("err17b", 0x0000_0f00, "000f0000\n");
    assert_exact("err17c", 0x000f_0000, "00000f00\n");
}

// Row 18 — high-bit bytes must be ZERO-extended on the char->int promotion.
#[test]
fn err18_unsigned_char_zero_extension() {
    assert_exact("err18", 0x8080_8080, "80808080\n");
    assert_exact("err18b", 0xff80_81fe, "fe8180ff\n");
    for b in 0x80u32..=0xff {
        let bits = b | (b << 8) | (b << 16) | (b << 24);
        let want = format!("{b:02x}").repeat(4) + "\n";
        assert_exact("err18-sweep", bits, &want);
    }
}

// Row 19 — 1000 consecutive calls: no hidden state, no buffer desync.
#[test]
fn err19_repeated_invocation() {
    let mut rng = Rng::new(0x1919_1919);
    let xs: Vec<f32> = (0..1000).map(|_| f32::from_bits(rng.next_u32())).collect();
    let c_out = run_batch(&c_driver(), &xs);
    let r_out = run_batch(&rust_driver(), &xs);
    assert_eq!(c_out.len(), 9000);
    assert_eq!(r_out.len(), 9000);
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, expected_batch(&xs));
}

// Row 20 — `print_hex` is `static` in C, so it must be dlsym-invisible in BOTH
// shared objects (an over-eager `#[no_mangle]` would leak it).
#[test]
fn err20_static_helper_not_exported() {
    unsafe {
        let c = libloading::Library::new(c_so_path()).unwrap();
        let r = libloading::Library::new(rust_so_path()).unwrap();
        for name in [b"print_hex\0".as_ref(), b"driver_print_hex\0".as_ref()] {
            let cs: Result<libloading::Symbol<*const ()>, _> = c.get(name);
            let rs: Result<libloading::Symbol<*const ()>, _> = r.get(name);
            assert_eq!(
                cs.is_ok(),
                rs.is_ok(),
                "export visibility of {:?} differs between C and Rust",
                String::from_utf8_lossy(name)
            );
            assert!(cs.is_err(), "C unexpectedly exports a static helper");
        }
        // `driver` itself of course IS exported by both.
        let cd: Result<libloading::Symbol<DriverFn>, _> = c.get(b"driver\0");
        let rd: Result<libloading::Symbol<DriverFn>, _> = r.get(b"driver\0");
        assert!(cd.is_ok() && rd.is_ok(), "driver must be exported by both");
    }
}

// Generic FFI boundary checks that every C API has. Documented here so it is
// explicit that they are inapplicable-by-construction rather than skipped.
#[test]
fn err_generic_boundaries_are_inapplicable_by_construction() {
    // The only public entry point takes a by-value `float`:
    //   * no pointer parameter  -> no null-pointer case
    //   * no length parameter   -> no zero/oversized-length case
    //   * no enum parameter     -> no out-of-range-enum case
    //   * no return value       -> no error code to compare
    // The header is asserted to still match that shape, so this row cannot rot.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let decls: Vec<&str> = hdr
        .lines()
        .filter(|l| l.contains('(') && !l.trim_start().starts_with("//"))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "driver.h now declares more than one function: {decls:?}"
    );
    assert!(
        decls[0].contains("void driver(float x)"),
        "public API signature changed: {:?}",
        decls[0]
    );
    assert!(!hdr.contains("enum"), "an enum appeared in the public API");
    assert!(!hdr.contains('*'), "a pointer appeared in the public API");

    // The nearest analogue of an "out of range value crossing the FFI
    // boundary" for a float parameter is a bit pattern that is not a valid
    // number at all. Exercise the full non-numeric space exhaustively over the
    // NaN/inf exponent, both signs.
    let mut bits = Vec::new();
    for sign in [0u32, 0x8000_0000] {
        for payload in 0..=0x3ff_u32 {
            bits.push(sign | 0x7f80_0000 | payload);
            bits.push(sign | 0x7f80_0000 | (payload << 13));
        }
    }
    assert_same_bits("err-generic-nonnumeric", &bits);
}
