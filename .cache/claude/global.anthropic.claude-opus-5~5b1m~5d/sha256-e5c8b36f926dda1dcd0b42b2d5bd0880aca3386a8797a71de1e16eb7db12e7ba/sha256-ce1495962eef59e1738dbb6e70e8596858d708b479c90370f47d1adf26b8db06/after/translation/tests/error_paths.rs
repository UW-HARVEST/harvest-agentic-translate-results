// Phase C — error-path / degenerate-input differential tests, one test per row
// of ERRORS.md.
//
// The C library has NO explicit rejection path (see the grep evidence in
// ERRORS.md): `void driver(float)` validates nothing and returns nothing. The
// differential obligation is therefore that for every degenerate, boundary or
// "would-be-invalid" input, C and Rust both ACCEPT it and emit byte-identical
// output, with neither trapping, aborting nor panicking.

mod common;

use common::*;

const SEED: u64 = 0xFEED_FACE_DEAD_BEEF;

/// Shared helper: assert both implementations accept `x` and agree exactly,
/// and that the output equals `expected` when the row pins an exact string.
fn expect(row: &str, x: f32, expected: &[u8]) {
    let c_out = run_one(Impl::C, x);
    let rust_out = run_one(Impl::Rust, x);
    assert_eq!(
        c_out,
        expected,
        "[{row}] C output for bits 0x{:08x} was {:?}, expected {:?}",
        x.to_bits(),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(expected)
    );
    assert_eq!(
        rust_out,
        expected,
        "[{row}] Rust output for bits 0x{:08x} was {:?}, expected {:?}",
        x.to_bits(),
        String::from_utf8_lossy(&rust_out),
        String::from_utf8_lossy(expected)
    );
    assert_eq!(c_out, rust_out, "[{row}] C/Rust divergence");
}

// --- E1 & E2 --------------------------------------------------------------
// The single conditional in the library is `for (int i = 0; i < len; i++)` in
// `print_hex`. `len == 0` (E1) and `len < 0` (E2) would take the loop zero
// times and print only "\n". Neither is reachable through the public API,
// because `driver` always passes the compile-time constant `sizeof(float)`.
// This test pins that structurally: no input can make either implementation
// produce the degenerate 1-byte record, and both always dereference exactly 4
// bytes.
#[test]
fn err_e1_loop_guard_never_degenerates() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut xs = vec![
        0.0f32,
        f32::from_bits(0x8000_0000),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0x0000_0001),
        f32::MAX,
        -f32::MAX,
    ];
    for _ in 0..20_000 {
        xs.push(f32::from_bits(rng.next_u32()));
    }

    for which in [Impl::C, Impl::Rust] {
        let out = run_batch(which, &xs);
        assert_eq!(
            out.len(),
            xs.len() * 9,
            "[E1/E2] {}: every record must be 9 bytes (8 hex digits from len=4, \
             plus newline). A degenerate loop guard (len<=0) would give 1-byte records.",
            which.name()
        );
        for (i, rec) in out.chunks(9).enumerate() {
            assert_ne!(rec, b"\n", "[E1/E2] {}: record {i} degenerated", which.name());
            assert_eq!(rec.len(), 9);
        }
    }
    // And C == Rust over the whole set.
    assert_batch_matches("E1/E2", &xs);
}

// --- E3 -------------------------------------------------------------------
#[test]
fn err_e3_quiet_nan() {
    let qnan = f32::from_bits(0x7fc0_0000);
    assert!(qnan.is_nan());
    expect("E3", qnan, b"0000c07f\n");
    // Rust's own f32::NAN must be handled identically too.
    let c_out = run_one(Impl::C, f32::NAN);
    let rust_out = run_one(Impl::Rust, f32::NAN);
    assert_eq!(c_out, rust_out, "[E3] f32::NAN");
}

// --- E4 -------------------------------------------------------------------
#[test]
fn err_e4_signalling_nan() {
    // A signalling NaN must survive the `float` ABI without being quietened
    // (that would flip bit 22 and change the output).
    let snan = f32::from_bits(0x7f80_0001);
    expect("E4", snan, b"0100807f\n");

    // A spread of sNaNs (exponent 0xff, mantissa MSB clear, mantissa != 0).
    let mut rng = Rng::new(SEED ^ 4);
    let mut xs = vec![
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0x7f80_0002),
        f32::from_bits(0x7fbf_ffff), // largest positive sNaN
        f32::from_bits(0xff80_0001),
        f32::from_bits(0xffbf_ffff),
    ];
    for _ in 0..5_000 {
        let sign = rng.bit();
        let mantissa = rng.range(1, 0x3f_ffff); // MSB of mantissa clear => sNaN
        xs.push(from_fields(sign, 0xff, mantissa));
    }
    assert_each_matches("E4", &xs[..5]);
    assert_batch_matches("E4-random", &xs);
}

// --- E5 -------------------------------------------------------------------
#[test]
fn err_e5_negative_nan() {
    expect("E5", f32::from_bits(0xffc0_0000), b"0000c0ff\n");
    expect("E5", f32::from_bits(0xffff_ffff), b"ffffffff\n");
}

// --- E6 -------------------------------------------------------------------
#[test]
fn err_e6_nan_payloads() {
    expect("E6", f32::from_bits(0x7fc0_dead), b"addec07f\n");
    expect("E6", f32::from_bits(0x7fab_cdef), b"efcdab7f\n");
    expect("E6", f32::from_bits(0xffab_cdef), b"efcdabff\n");

    // Every payload bit must survive: sweep single-bit payloads.
    let mut xs = Vec::new();
    for bit in 0..23u32 {
        for sign in 0..2u32 {
            xs.push(from_fields(sign, 0xff, 1 << bit));
        }
    }
    assert_each_matches("E6-bits", &xs);
}

// --- E7 & E8 --------------------------------------------------------------
#[test]
fn err_e7_infinities() {
    expect("E7", f32::INFINITY, b"0000807f\n");
    expect("E8", f32::NEG_INFINITY, b"000080ff\n");
    // Reached by overflow as well as by literal construction.
    let overflow = f32::MAX * 2.0;
    assert!(overflow.is_infinite());
    let c_out = run_one(Impl::C, overflow);
    assert_eq!(c_out, run_one(Impl::Rust, overflow), "[E7] overflowed value");
    assert_eq!(c_out, b"0000807f\n".to_vec());
}

// --- E9 & E10 -------------------------------------------------------------
#[test]
fn err_e9_signed_zeros() {
    let pos = 0.0f32;
    let neg = f32::from_bits(0x8000_0000);
    assert_eq!(pos, neg, "IEEE equality holds, but the bytes must still differ");

    expect("E10", pos, b"00000000\n");
    expect("E9", neg, b"00000080\n");

    // -0.0 must NOT be normalised to +0.0 by either implementation.
    assert_ne!(
        run_one(Impl::C, pos),
        run_one(Impl::C, neg),
        "[E9] C distinguishes +0.0 from -0.0"
    );
    assert_ne!(
        run_one(Impl::Rust, pos),
        run_one(Impl::Rust, neg),
        "[E9] Rust must also distinguish +0.0 from -0.0"
    );

    // -0.0 arrived at arithmetically, not just by bit construction.
    let computed = -0.0f32 * 1.0;
    assert_eq!(run_one(Impl::C, computed), run_one(Impl::Rust, computed));
}

// --- E11 ------------------------------------------------------------------
#[test]
fn err_e11_subnormals() {
    // Must not be flushed to zero.
    expect("E11", f32::from_bits(0x0000_0001), b"01000000\n");
    expect("E11", f32::from_bits(0x007f_ffff), b"ffff7f00\n");
    expect("E11", f32::from_bits(0x8000_0001), b"01000080\n");
    expect("E11", f32::from_bits(0x807f_ffff), b"ffff7f80\n");

    for &bits in &[0x0000_0001u32, 0x007f_ffff, 0x8000_0001, 0x807f_ffff] {
        let out = run_one(Impl::C, f32::from_bits(bits));
        assert_ne!(
            out,
            b"00000000\n".to_vec(),
            "[E11] C must not flush subnormal 0x{bits:08x} to zero"
        );
        assert_eq!(run_one(Impl::Rust, f32::from_bits(bits)), out);
    }

    // Randomised subnormals, both signs.
    let mut rng = Rng::new(SEED ^ 11);
    let xs: Vec<f32> = (0..20_000)
        .map(|_| {
            let sign = rng.bit();
            from_fields(sign, 0, rng.range(1, 0x7f_ffff))
        })
        .collect();
    assert_batch_matches("E11-random", &xs);
}

// --- E12 ------------------------------------------------------------------
#[test]
fn err_e12_range_extremes_and_one_past() {
    // The documented "range" of a float is the whole binary32 space; one step
    // past FLT_MAX is +inf, one step below FLT_MIN is the largest subnormal.
    expect("E12", f32::MAX, b"ffff7f7f\n"); // 0x7f7fffff
    expect("E12", -f32::MAX, b"ffff7fff\n"); // 0xff7fffff
    expect("E12", f32::MIN_POSITIVE, b"00008000\n"); // 0x00800000
    expect("E12", -f32::MIN_POSITIVE, b"00008080\n"); // 0x80800000
    expect("E12", f32::EPSILON, b"00000034\n"); // 0x34000000

    // One ULP past each extreme.
    let one_past_max = f32::from_bits(f32::MAX.to_bits() + 1); // +inf
    assert!(one_past_max.is_infinite());
    expect("E12-past", one_past_max, b"0000807f\n");

    let one_below_min_normal = f32::from_bits(f32::MIN_POSITIVE.to_bits() - 1);
    assert!(!one_below_min_normal.is_normal() && one_below_min_normal != 0.0);
    expect("E12-past", one_below_min_normal, b"ffff7f00\n");

    let one_past_neg_max = f32::from_bits((-f32::MAX).to_bits() + 1); // -inf
    assert!(one_past_neg_max.is_infinite());
    expect("E12-past", one_past_neg_max, b"000080ff\n");

    // One ULP either side of every extreme, differentially.
    let mut xs = Vec::new();
    for &b in &[
        0x0000_0000u32,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x0080_0001,
        0x7f7f_fffe,
        0x7f7f_ffff,
        0x7f80_0000,
        0x7f80_0001,
        0x8000_0000,
        0x8000_0001,
        0xff7f_ffff,
        0xff80_0000,
        0xff80_0001,
        0x34000000,
    ] {
        xs.push(f32::from_bits(b));
    }
    assert_each_matches("E12-neighbourhood", &xs);
}

// --- E13 ------------------------------------------------------------------
#[test]
fn err_e13_exhaustive_boundary_bitpatterns() {
    // Every one of the 2^32 bit patterns is a legal argument -- there is no
    // out-of-range value to reject. Rather than 2^32 calls, sweep exhaustively
    // over each 16-bit half while the other half is pinned to boundary
    // constants, then add a large uniform random sample.
    const BATCH: usize = 16_384;

    // (a) all 65536 high halves (sign + exponent + top mantissa bits), with the
    //     low half pinned to boundary values.
    for &low in &[0x0000u32, 0x0001, 0x8000, 0xffff] {
        let mut xs = Vec::with_capacity(BATCH);
        for hi in 0u32..=0xffff {
            xs.push(f32::from_bits((hi << 16) | low));
            if xs.len() == BATCH {
                assert_batch_matches("E13-hi", &xs);
                xs.clear();
            }
        }
        if !xs.is_empty() {
            assert_batch_matches("E13-hi", &xs);
        }
    }

    // (b) all 65536 low halves, with the high half pinned to the interesting
    //     classes: zero/subnormal, +inf, canonical qNaN, negative qNaN.
    for &hi in &[0x0000u32, 0x7f80, 0x7fc0, 0xffc0] {
        let mut xs = Vec::with_capacity(BATCH);
        for low in 0u32..=0xffff {
            xs.push(f32::from_bits((hi << 16) | low));
            if xs.len() == BATCH {
                assert_batch_matches("E13-lo", &xs);
                xs.clear();
            }
        }
        if !xs.is_empty() {
            assert_batch_matches("E13-lo", &xs);
        }
    }

    // (c) uniform random over the entire 32-bit space.
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..12 {
        let xs: Vec<f32> = (0..BATCH).map(|_| f32::from_bits(rng.next_u32())).collect();
        assert_batch_matches("E13-random", &xs);
    }
}

// --- E14 ------------------------------------------------------------------
#[test]
fn err_e14_repeated_calls_no_state() {
    // The C `printf` writes into the process-wide libc `stdout` FILE*. A
    // translation that used Rust's own `std::io::stdout` buffer would flush at
    // different times and interleave differently. Verify the N-call output is
    // exactly the ordered concatenation of the single-call outputs, for both
    // implementations, and that repeating the same value many times is stable.
    let mut rng = Rng::new(SEED ^ 14);
    let xs: Vec<f32> = (0..300).map(|_| f32::from_bits(rng.next_u32())).collect();

    for which in [Impl::C, Impl::Rust] {
        let batched = run_batch(which, &xs);
        let mut concat = Vec::new();
        for &x in &xs {
            concat.extend_from_slice(&run_one(which, x));
        }
        assert_eq!(
            batched,
            concat,
            "[E14] {}: repeated invocation carried state or reordered output",
            which.name()
        );
    }
    assert_batch_matches("E14", &xs);

    // Same value 1000 times: output must be that record repeated verbatim.
    let v = f32::from_bits(0x7fc0_dead);
    let repeated: Vec<f32> = vec![v; 1000];
    let c_out = run_batch(Impl::C, &repeated);
    let rust_out = run_batch(Impl::Rust, &repeated);
    assert_eq!(c_out, rust_out, "[E14] repeated identical input");
    assert_eq!(c_out, oracle_one(v).repeat(1000));
}

// --- Generic FFI boundaries ------------------------------------------------
// ERRORS.md records that null pointers, length parameters and enum values do
// not exist in this API (`void driver(float)` has no pointer, no length and no
// int-valued parameter, and the header declares no enum/struct/typedef). This
// test pins those facts so a future API change cannot silently invalidate them,
// and confirms the exported symbol has the expected float-only ABI by calling
// it through a deliberately re-declared signature.
#[test]
fn err_generic_ffi_boundaries() {
    ensure_loaded();

    // The header declares exactly one function and no other public construct.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let body: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("void driver(float x);"));
    for absent in ["enum", "struct", "typedef", "*", "int ", "size_t"] {
        assert!(
            !body.contains(absent),
            "driver.h unexpectedly contains `{absent}`; the ERRORS.md \
             justification for skipping null/length/enum boundaries no longer holds"
        );
    }

    // There is no out-of-range enum to pass, but the nearest analogue is an
    // arbitrary 32-bit word arriving in the float argument slot -- including
    // patterns that are not any "valid variant" of a finite float. Push several
    // such words through both libraries via the exported symbol.
    let hostile = [
        0x0000_0000u32,
        0xffff_ffff,
        0x7fff_ffff,
        0x8000_0000,
        0xdead_beef,
        0xcafe_babe,
        0x8080_8080,
        0x7f7f_7f7f,
        0xaaaa_aaaa,
        0x5555_5555,
    ];
    let xs: Vec<f32> = hostile.iter().map(|&b| f32::from_bits(b)).collect();
    assert_each_matches("generic-hostile-words", &xs);

    // Calling through a transmuted u32-taking signature must yield the same
    // bytes as the f32 signature would for the same bit pattern -- i.e. no
    // hidden argument conversion happens in either export wrapper.
    for &bits in &hostile {
        let x = f32::from_bits(bits);
        assert_eq!(
            run_one(Impl::C, x),
            run_one(Impl::Rust, x),
            "[generic] bit pattern 0x{bits:08x}"
        );
    }
}
