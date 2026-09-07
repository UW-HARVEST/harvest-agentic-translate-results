//! C-vs-Rust differential tests for the `driver` library.
//!
//! Both implementations are loaded from their shared objects via `libloading`
//! and invoked through the exported `driver` symbol only — never by calling the
//! Rust function directly — so the `#[no_mangle] extern "C"` wrapper is
//! exercised as an external caller would exercise it.
//!
//! Test names map onto the rows of `CONFIGS.md` (Phase B, `cfg*`) and
//! `ERRORS.md` (Phase C, `err*`).

mod harness;

use harness::{assert_same, assert_same_batch, Buffering, Impls, Rng, Sink};

fn impls() -> Impls {
    Impls::load()
}

// ===========================================================================
// Phase B — valid-path differential tests (one test per CONFIGS.md row)
// ===========================================================================

/// CONFIGS rows 1–4: exhaustively place every one of the 256 byte values into
/// every one of the 4 byte positions.
#[test]
fn cfg01_04_all_byte_values_in_every_position() {
    let im = impls();
    for shift in [0u32, 8, 16, 24] {
        let xs: Vec<f32> = (0u32..=255)
            .map(|nn| f32::from_bits(nn << shift))
            .collect();
        assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
    }
}

/// CONFIGS row 5: uniform random 32-bit patterns. Covers every IEEE-754 class
/// (normals, subnormals, zeros, infinities, qNaN, sNaN) by construction.
#[test]
fn cfg05_uniform_random_bit_patterns() {
    let im = impls();
    let mut rng = Rng::new();
    let xs: Vec<f32> = (0..20_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 6: random *positive normal* finite floats.
#[test]
fn cfg06_random_positive_normals() {
    let im = impls();
    let mut rng = Rng::with_seed(0x1111_2222_3333_4444);
    let xs: Vec<f32> = (0..4_000)
        .map(|_| {
            let exp = 1 + rng.below(254); // 1..=254 => normal
            let mant = rng.next_u32() & 0x007f_ffff;
            f32::from_bits((exp << 23) | mant)
        })
        .collect();
    for x in &xs {
        assert!(x.is_normal(), "row 6 generator must yield normals");
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 7: random *negative normal* finite floats (sign bit set).
#[test]
fn cfg07_random_negative_normals() {
    let im = impls();
    let mut rng = Rng::with_seed(0x5555_6666_7777_8888);
    let xs: Vec<f32> = (0..4_000)
        .map(|_| {
            let exp = 1 + rng.below(254);
            let mant = rng.next_u32() & 0x007f_ffff;
            f32::from_bits(0x8000_0000 | (exp << 23) | mant)
        })
        .collect();
    for x in &xs {
        assert!(x.is_normal() && x.is_sign_negative());
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 8: subnormals only (exponent field 0, non-zero mantissa), both
/// signs. A translation that routed the value through any kind of arithmetic
/// normalisation would break here.
#[test]
fn cfg08_random_subnormals() {
    let im = impls();
    let mut rng = Rng::with_seed(0x9999_aaaa_bbbb_cccc);
    let xs: Vec<f32> = (0..4_000)
        .map(|_| {
            let mut mant = rng.next_u32() & 0x007f_ffff;
            if mant == 0 {
                mant = 1;
            }
            let sign = (rng.next_u32() & 1) << 31;
            f32::from_bits(sign | mant)
        })
        .collect();
    for x in &xs {
        assert!(x.is_subnormal(), "row 8 generator must yield subnormals");
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 9: the whole NaN space — quiet and signalling, both signs,
/// random payloads. The bit pattern must survive the FFI argument pass
/// untouched (no quieting, no canonicalisation).
#[test]
fn cfg09_random_nan_payloads() {
    let im = impls();
    let mut rng = Rng::with_seed(0xdddd_eeee_ffff_0001);
    let xs: Vec<f32> = (0..4_000)
        .map(|_| {
            let mut mant = rng.next_u32() & 0x007f_ffff;
            if mant == 0 {
                mant = 1; // non-zero mantissa => NaN, not infinity
            }
            let sign = (rng.next_u32() & 1) << 31;
            f32::from_bits(sign | 0x7f80_0000 | mant)
        })
        .collect();
    for x in &xs {
        assert!(x.is_nan(), "row 9 generator must yield NaNs");
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 10: every byte below `0x10`, so `%02x` must zero-pad in all four
/// positions (`0f` not `f`, `00` not `0`).
#[test]
fn cfg10_low_nibble_only_bytes() {
    let im = impls();
    let mut rng = Rng::with_seed(0x0f0f_0f0f_0f0f_0f0f);
    let xs: Vec<f32> = (0..2_000)
        .map(|_| {
            let b = |r: &mut Rng| (r.next_u8() & 0x0f) as u32;
            let (b0, b1, b2, b3) = (b(&mut rng), b(&mut rng), b(&mut rng), b(&mut rng));
            f32::from_bits(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
        })
        .collect();
    for x in &xs {
        assert!(x.to_bits().to_ne_bytes().iter().all(|&b| b < 0x10));
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 11: every byte at or above `0x80`. This is the signed-`char`
/// sign-extension trap: the C casts `char raw[]` to `unsigned char *`, so each
/// byte must print as exactly two unsigned hex digits (`ff`), never as a
/// sign-extended `ffffffff`.
#[test]
fn cfg11_high_bit_bytes_only() {
    let im = impls();
    let mut rng = Rng::with_seed(0x8080_8080_8080_8080);
    let xs: Vec<f32> = (0..2_000)
        .map(|_| {
            let b = |r: &mut Rng| (r.next_u8() | 0x80) as u32;
            let (b0, b1, b2, b3) = (b(&mut rng), b(&mut rng), b(&mut rng), b(&mut rng));
            f32::from_bits(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
        })
        .collect();
    for x in &xs {
        assert!(x.to_bits().to_ne_bytes().iter().all(|&b| b >= 0x80));
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 12: the values a typical consumer actually passes — small
/// integers widened to `f32`.
#[test]
fn cfg12_integer_valued_floats() {
    let im = impls();
    let mut rng = Rng::with_seed(0x1234_5678_9abc_def0);
    let mut xs: Vec<f32> = Vec::new();
    for _ in 0..1_500 {
        xs.push(rng.next_u32() as i16 as f32);
    }
    for _ in 0..1_500 {
        xs.push(rng.next_u32() as i32 as f32);
    }
    for i in -256i32..=256 {
        xs.push(i as f32);
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 13: arguments arriving from a *computation* rather than a
/// constant load, so the value reaches the callee in an xmm register produced by
/// arithmetic.
#[test]
fn cfg13_computed_arguments() {
    let im = impls();
    let mut rng = Rng::with_seed(0xfeed_face_cafe_beef);
    let mut xs: Vec<f32> = Vec::new();
    for _ in 0..1_000 {
        let a = f32::from_bits(rng.next_u32() & 0x7f7f_ffff);
        let b = f32::from_bits(rng.next_u32() & 0x7f7f_ffff);
        xs.push(a / b);
        xs.push(a * b);
        xs.push(a + b);
        xs.push(a - b);
        xs.push(a.sqrt());
        xs.push(1.0f32 / a);
    }
    // Division and 1/x deliberately include the inf/NaN-producing cases.
    xs.push(0.0f32 / 0.0f32);
    xs.push(1.0f32 / 0.0f32);
    xs.push(-1.0f32 / 0.0f32);
    xs.push((-1.0f32).sqrt());
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 14: the named special values.
#[test]
fn cfg14_named_special_values() {
    let im = impls();
    let xs = [
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        std::f32::consts::PI,
        std::f32::consts::E,
        std::f32::consts::LN_2,
        std::f32::consts::SQRT_2,
    ];
    for &x in &xs {
        assert_same(&im, x);
    }
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 15: a single call, and then 1000 sequential calls inside one
/// capture window, so accumulated output is compared line for line.
#[test]
fn cfg15_single_and_repeated_calls() {
    let im = impls();
    assert_same(&im, 1.0);

    let mut rng = Rng::with_seed(0xabcd_ef01_2345_6789);
    let xs: Vec<f32> = (0..1_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 16: C and Rust calls interleaved in the *same* capture window,
/// sharing one libc `stdout` `FILE` with no intervening flush. The two
/// implementations must produce identical alternating lines.
#[test]
fn cfg16_interleaved_c_and_rust_calls() {
    let im = impls();
    let mut rng = Rng::with_seed(0x0dd1_0dd2_0dd3_0dd4);
    let xs: Vec<f32> = (0..500).map(|_| f32::from_bits(rng.next_u32())).collect();

    let _g = harness::lock_captures();
    let out = harness::capture_with(Sink::File, Buffering::Default, || {
        for &x in &xs {
            unsafe { (im.c_driver)(x) };
            unsafe { (im.rust_driver)(x) };
        }
    });

    let lines: Vec<&[u8]> = out
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), xs.len() * 2, "expected two lines per input");
    for (i, pair) in lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0],
            pair[1],
            "interleaved divergence for input {:?} (bits {:#010x}): C {:?} vs Rust {:?}",
            xs[i],
            xs[i].to_bits(),
            String::from_utf8_lossy(pair[0]),
            String::from_utf8_lossy(pair[1]),
        );
    }
}

/// CONFIGS row 17: stdout is a **pipe** rather than a regular file.
#[test]
fn cfg17_stdout_is_a_pipe() {
    let im = impls();
    let mut rng = Rng::with_seed(0x7777_1111_2222_3333);
    let xs: Vec<f32> = (0..2_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same_batch(&im, &xs, Sink::Pipe, Buffering::Default);
}

/// CONFIGS row 18: stdout explicitly unbuffered (`setvbuf(_IONBF)`), for both
/// sink kinds.
#[test]
fn cfg18_stdout_unbuffered() {
    let im = impls();
    let mut rng = Rng::with_seed(0x4444_5555_6666_7777);
    let xs: Vec<f32> = (0..500).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same_batch(&im, &xs, Sink::File, Buffering::Unbuffered);
    assert_same_batch(&im, &xs, Sink::Pipe, Buffering::Unbuffered);
}

/// CONFIGS row 19: the argument arrives via a `from_bits`/`to_ne_bytes` round
/// trip and as a value loaded out of a `Vec<f32>` (a memory operand) rather
/// than an immediate.
#[test]
fn cfg19_argument_provenance() {
    let im = impls();
    let mut rng = Rng::with_seed(0x2222_3333_4444_5555);

    let round_tripped: Vec<f32> = (0..1_000)
        .map(|_| {
            let bits = rng.next_u32();
            let x = f32::from_bits(bits);
            f32::from_ne_bytes(x.to_ne_bytes())
        })
        .collect();
    assert_same_batch(&im, &round_tripped, Sink::File, Buffering::Default);

    // Heap-allocated, so the callee's argument comes from a load.
    let heap: Vec<f32> = (0..1_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    let boxed: Box<[f32]> = heap.clone().into_boxed_slice();
    assert_same_batch(&im, &boxed, Sink::File, Buffering::Default);
}

// ===========================================================================
// Phase C — error/boundary-path differential tests
//
// `ERRORS.md` records that the C code contains no rejection path at all (no
// `return`, no `assert`, no range check, no null check, no error enum). These
// tests therefore cover the generic boundaries every C API has, asserting the
// two implementations behave *identically* at each one.
// ===========================================================================

/// ERRORS E5, E6: zero and negative zero. `-0.0` has a distinct object
/// representation and must not be normalised to `+0.0`.
#[test]
fn err_zero_and_negative_zero() {
    let im = impls();
    let _g = harness::lock_captures();

    let pos = harness::capture(|| unsafe { (im.c_driver)(0.0) });
    let pos_r = harness::capture(|| unsafe { (im.rust_driver)(0.0) });
    assert_eq!(pos, pos_r);
    assert_eq!(pos, b"00000000\n");

    let neg = harness::capture(|| unsafe { (im.c_driver)(-0.0) });
    let neg_r = harness::capture(|| unsafe { (im.rust_driver)(-0.0) });
    assert_eq!(neg, neg_r);
    assert_eq!(neg, b"00000080\n", "-0.0 must keep its sign bit");
    assert_ne!(pos, neg, "+0.0 and -0.0 must print differently");
}

/// ERRORS E7, E8: one step past the largest / most negative finite value, i.e.
/// the finite-range boundary and the infinity just beyond it. No clamping.
#[test]
fn err_past_finite_range() {
    let im = impls();
    for bits in [
        0x7f7f_ffffu32, // f32::MAX
        0x7f80_0000,    // +inf, one ulp past MAX
        0xff7f_ffff,    // f32::MIN
        0xff80_0000,    // -inf, one ulp past MIN
        0x7f80_0001,    // one past +inf: smallest positive sNaN
        0xff80_0001,    // one past -inf
    ] {
        assert_same(&im, f32::from_bits(bits));
    }
}

/// ERRORS E9: the infinities.
#[test]
fn err_infinities() {
    let im = impls();
    let _g = harness::lock_captures();

    for (x, expected) in [
        (f32::INFINITY, &b"0000807f\n"[..]),
        (f32::NEG_INFINITY, &b"000080ff\n"[..]),
    ] {
        let c = harness::capture(|| unsafe { (im.c_driver)(x) });
        let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
        assert_eq!(c, r, "divergence for {x:?}");
        assert_eq!(c.as_slice(), expected);
    }
}

/// ERRORS E10–E12: qNaN, sNaN, and non-canonical NaN payloads, including one
/// step past the canonical quiet NaN. A signalling NaN must not be quieted by
/// the FFI argument pass on either side.
#[test]
fn err_nan_payloads() {
    let im = impls();
    let _g = harness::lock_captures();

    let cases: [(u32, &[u8]); 8] = [
        (0x7fc0_0000, b"0000c07f\n"), // canonical qNaN
        (0x7fc0_0001, b"0100c07f\n"), // one past canonical qNaN
        (0x7fa0_0000, b"0000a07f\n"), // sNaN
        (0xffa0_0000, b"0000a0ff\n"), // negative sNaN
        (0x7f80_0001, b"0100807f\n"), // smallest positive sNaN
        (0xff80_0001, b"010080ff\n"), // smallest negative sNaN
        (0x7fff_ffff, b"ffffff7f\n"), // maximum payload
        (0xffff_ffff, b"ffffffff\n"), // maximum payload, sign set
    ];

    for (bits, expected) in cases {
        let x = f32::from_bits(bits);
        assert!(x.is_nan(), "bits {bits:#010x} must be a NaN");
        let c = harness::capture(|| unsafe { (im.c_driver)(x) });
        let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
        assert_eq!(
            c, r,
            "NaN payload divergence for bits {bits:#010x}: C {:?} vs Rust {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert_eq!(
            c.as_slice(),
            expected,
            "payload not preserved for bits {bits:#010x}"
        );
    }
}

/// ERRORS E13, E14: the smallest subnormal (one step past zero), the largest
/// subnormal, and the subnormal-to-normal step.
#[test]
fn err_subnormal_boundaries() {
    let im = impls();
    let _g = harness::lock_captures();

    let cases: [(u32, &[u8]); 6] = [
        (0x0000_0001, b"01000000\n"), // smallest positive subnormal
        (0x8000_0001, b"01000080\n"), // smallest negative subnormal
        (0x007f_ffff, b"ffff7f00\n"), // largest positive subnormal
        (0x0080_0000, b"00008000\n"), // smallest positive normal
        (0x807f_ffff, b"ffff7f80\n"), // largest negative subnormal
        (0x8080_0000, b"00008080\n"), // smallest negative normal
    ];

    for (bits, expected) in cases {
        let x = f32::from_bits(bits);
        let c = harness::capture(|| unsafe { (im.c_driver)(x) });
        let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
        assert_eq!(c, r, "divergence for bits {bits:#010x}");
        assert_eq!(c.as_slice(), expected, "wrong bytes for {bits:#010x}");
    }
}

/// ERRORS E15: `%02x` zero padding for bytes below `0x10`.
#[test]
fn err_hex_padding() {
    let im = impls();
    let _g = harness::lock_captures();

    let cases: [(u32, &[u8]); 4] = [
        (0x0000_0000, b"00000000\n"),
        (0x0403_0201, b"01020304\n"),
        (0x0f0f_0f0f, b"0f0f0f0f\n"),
        (0x0100_0f00, b"000f0001\n"),
    ];
    for (bits, expected) in cases {
        let x = f32::from_bits(bits);
        let c = harness::capture(|| unsafe { (im.c_driver)(x) });
        let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
        assert_eq!(c, r);
        assert_eq!(
            c.as_slice(),
            expected,
            "zero padding wrong for {bits:#010x}"
        );
        assert_eq!(c.len(), 9, "must be exactly 8 hex digits plus a newline");
    }
}

/// ERRORS E16: bytes with the high bit set must print unsigned (`ff`), not
/// sign-extended (`ffffffff`) — the `char` vs `unsigned char` trap.
#[test]
fn err_high_bit_bytes() {
    let im = impls();
    let _g = harness::lock_captures();

    for bits in [0xffff_ffffu32, 0x8080_8080, 0xff00_ff00, 0x80ff_7f01] {
        let x = f32::from_bits(bits);
        let c = harness::capture(|| unsafe { (im.c_driver)(x) });
        let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
        assert_eq!(c, r, "divergence for {bits:#010x}");
        assert_eq!(
            c.len(),
            9,
            "sign extension detected for {bits:#010x}: {:?}",
            String::from_utf8_lossy(&c)
        );
        let expected: String = bits
            .to_ne_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            + "\n";
        assert_eq!(String::from_utf8_lossy(&c), expected);
    }
}

/// ERRORS E17: every byte value in every byte position, checked one call at a
/// time (separate capture windows), not just batched.
#[test]
fn err_all_byte_values_all_positions() {
    let im = impls();
    let _g = harness::lock_captures();

    for shift in [0u32, 8, 16, 24] {
        for nn in 0u32..=255 {
            let bits = nn << shift;
            let x = f32::from_bits(bits);
            let c = harness::capture(|| unsafe { (im.c_driver)(x) });
            let r = harness::capture(|| unsafe { (im.rust_driver)(x) });
            assert_eq!(c, r, "divergence for bits {bits:#010x}");
            assert_eq!(c.len(), 9);
        }
    }
}

/// ERRORS E18: repeated and interleaved calls against the shared `stdout` FILE,
/// with no intervening flush, in both buffering modes.
#[test]
fn err_interleaved_calls() {
    let im = impls();
    let _g = harness::lock_captures();

    for buffering in [Buffering::Default, Buffering::Unbuffered] {
        for sink in [Sink::File, Sink::Pipe] {
            let out = harness::capture_with(sink, buffering, || unsafe {
                (im.c_driver)(1.0);
                (im.rust_driver)(1.0);
                (im.c_driver)(f32::NAN);
                (im.rust_driver)(f32::NAN);
                (im.rust_driver)(-0.0);
                (im.c_driver)(-0.0);
            });
            assert_eq!(
                out,
                b"0000803f\n0000803f\n0000c07f\n0000c07f\n00000080\n00000080\n",
                "interleaving diverged for sink={sink:?} buffering={buffering:?}: {:?}",
                String::from_utf8_lossy(&out)
            );
        }
    }
}

/// The whole public surface is one `void`-returning function taking a `float`
/// by value. `ERRORS.md` rows E1–E4 (null pointer, zero length, oversized
/// length, out-of-range enum) are not representable. This test pins that fact
/// against the header so the claim breaks loudly if the C API ever grows a
/// pointer, length or enum parameter.
#[test]
fn err_no_pointer_length_or_enum_parameters() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");

    // Strip the licence comment block so only declarations are inspected.
    let decls: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        decls.contains("void driver(float x);"),
        "public API changed; re-derive ERRORS.md / CONFIGS.md. Saw:\n{decls}"
    );
    assert!(
        !decls.contains('*'),
        "a pointer parameter appeared in the public API; ERRORS.md row E1 is no \
         longer n/a:\n{decls}"
    );
    assert!(
        !decls.contains("enum"),
        "an enum appeared in the public API; ERRORS.md row E4 is no longer n/a:\n{decls}"
    );

    // Exactly one public function is declared.
    let fn_decls = decls.matches(';').count();
    assert_eq!(
        fn_decls, 1,
        "expected exactly one declaration in driver.h, found {fn_decls}"
    );
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under
/// the exact same name, and the C's `static` helper must stay unexported on
/// both sides.
#[test]
fn phase_d_symbol_parity() {
    fn exported(path: &std::path::Path) -> std::collections::BTreeSet<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {path:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let name = it.next()?;
                let kind = it.next()?;
                // Global text/data only; skip local and toolchain-internal entries.
                if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V")
                    && !name.starts_with("_init")
                    && !name.starts_with("_fini")
                    && !name.starts_with("__")
                {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    let c_so = harness::c_so_path();
    let rust_so = harness::rust_so_path();
    assert!(c_so.exists(), "C .so missing at {c_so:?}");
    assert!(rust_so.exists(), "Rust .so missing at {rust_so:?}");

    let c_syms = exported(&c_so);
    let rust_syms = exported(&rust_so);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing C-exported symbols: {missing:?}"
    );
    assert!(
        c_syms.contains("driver"),
        "sanity: C .so must export `driver`, saw {c_syms:?}"
    );

    // `print_hex` is `static` in the C, so it must not be exported anywhere.
    assert!(!c_syms.contains("print_hex"));
    assert!(
        !rust_syms.contains("print_hex"),
        "Rust exported the internal-linkage helper `print_hex`"
    );
}

// ===========================================================================
// Phase B (continued) — high-volume sweeps.
//
// Rows 1-4 already prove the `%02x` formatting exhaustively: the output is a
// per-byte function, and every one of the 256 byte values is checked in every
// one of the 4 positions. What these sweeps add is coverage of the *float
// argument pass* itself over a very large portion of the 2^32 input space,
// where a value-dependent bug (NaN quieting, subnormal normalisation, x87
// double rounding) would show up.
// ===========================================================================

/// Heavy sweeps cost ~4 minutes. `DRIVER_FAST=1` skips them so the redundant
/// feature-combination re-runs (which, absent a `[features]` table, compile the
/// exact same code) do not pay for them repeatedly. The full sweep is always run
/// at least once per cdylib profile by `run_all.sh`.
fn fast_mode() -> bool {
    std::env::var("DRIVER_FAST").map(|v| v == "1").unwrap_or(false)
}

/// CONFIGS row 21: exhaustive over all 65536 values of the **upper** 16 bits
/// (sign + exponent + high mantissa), for several fixed low halves.
#[test]
fn cfg21_exhaustive_upper_16_bits() {
    let im = impls();
    for low in [0x0000u32, 0x0001, 0x1234, 0xffff] {
        let xs: Vec<f32> = (0u32..=0xffff)
            .map(|hi| f32::from_bits((hi << 16) | low))
            .collect();
        assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
    }
}

/// CONFIGS row 22: exhaustive over all 65536 values of the **lower** 16 bits
/// (low mantissa), for fixed upper halves spanning the zero, subnormal, normal,
/// infinity and NaN exponents.
#[test]
fn cfg22_exhaustive_lower_16_bits() {
    let im = impls();
    for hi in [
        0x0000u32, // zero / subnormal
        0x0080,    // smallest normal exponent
        0x3f80,    // exponent of 1.0
        0x7f80,    // infinity / signalling NaN
        0x7fc0,    // quiet NaN
        0xff80,    // -infinity / negative NaN
    ] {
        let xs: Vec<f32> = (0u32..=0xffff)
            .map(|low| f32::from_bits((hi << 16) | low))
            .collect();
        assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
    }
}

/// CONFIGS row 23: a strided sweep across the **entire** 2^32 pattern space.
/// The stride is prime, so the samples walk every exponent and mantissa region
/// rather than clustering.
#[test]
fn cfg23_strided_full_range_sweep() {
    if fast_mode() {
        eprintln!("{}: skipped (DRIVER_FAST=1)", stringify!(cfg23_strided_full_range_sweep));
        return;
    }
    let im = impls();
    const STRIDE: usize = 1021; // prime
    let xs: Vec<f32> = (0u64..(1u64 << 32))
        .step_by(STRIDE)
        .map(|bits| f32::from_bits(bits as u32))
        .collect();
    assert!(
        xs.len() > 4_000_000,
        "sweep unexpectedly small: {}",
        xs.len()
    );
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 24: two million uniform-random patterns, fixed seed.
#[test]
fn cfg24_large_random_sweep() {
    if fast_mode() {
        eprintln!("{}: skipped (DRIVER_FAST=1)", stringify!(cfg24_large_random_sweep));
        return;
    }
    let im = impls();
    let mut rng = Rng::with_seed(0x0123_4567_89ab_cdef);
    let xs: Vec<f32> = (0..2_000_000)
        .map(|_| f32::from_bits(rng.next_u32()))
        .collect();
    assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
}

/// CONFIGS row 25: the truly exhaustive check over all 2^32 patterns.
///
/// Ignored by default: it pushes ~38 GiB of output through libc per side and
/// takes far longer than the 600 s budget. Chunked so it can be run
/// deliberately with `cargo test -- --ignored`.
#[test]
#[ignore = "exhaustive 2^32 sweep: ~38 GiB of stdout per side"]
fn cfg25_exhaustive_all_2p32_patterns() {
    let im = impls();
    const CHUNK: u64 = 1 << 20;
    let mut base = 0u64;
    while base < (1u64 << 32) {
        let xs: Vec<f32> = (base..(base + CHUNK))
            .map(|bits| f32::from_bits(bits as u32))
            .collect();
        assert_same_batch(&im, &xs, Sink::File, Buffering::Default);
        base += CHUNK;
    }
}
