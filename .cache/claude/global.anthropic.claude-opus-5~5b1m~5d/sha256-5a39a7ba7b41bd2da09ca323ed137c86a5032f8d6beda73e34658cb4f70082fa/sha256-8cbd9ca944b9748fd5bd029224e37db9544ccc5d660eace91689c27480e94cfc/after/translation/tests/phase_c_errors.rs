//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row. Each asserts the C and Rust `.so`s produce the *same* concrete
//! `uint16_t`, not merely that both "failed somehow".

mod common;

use common::{bits_for, observed_base, observed_shift, pair, Rng, SHIFT_RUNS};

/// E1 — documentation row. Re-runs the mechanical scan that established the C
/// code has no rejection branch at all, so the absence of intrinsic-error rows
/// in `ERRORS.md` stays true if `c_src` ever changes.
#[test]
fn no_intrinsic_error_paths_documented() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let c = std::fs::read_to_string(root.join("c_src/src/lib.c")).expect("read c_src/src/lib.c");
    let h = std::fs::read_to_string(root.join("c_src/include/lib.h")).expect("read lib.h");

    // Strip the two big table initialisers so the scan looks at real code only.
    let code_only: String = c
        .lines()
        .filter(|l| !l.trim_start().starts_with("0x"))
        .collect::<Vec<_>>()
        .join("\n");

    for needle in [
        "RETURN_ERROR",
        "errno",
        "assert",
        "abort",
        "exit(",
        "goto",
        "NULL",
        "malloc",
        "free(",
        "#ifdef",
        "#if ",
        "switch",
    ] {
        assert!(
            !code_only.contains(needle),
            "ERRORS.md claims the C source has no `{needle}`, but it does — \
             the error-surface table must be regenerated"
        );
    }

    // No conditionals: `if` must not appear as a statement.
    assert!(
        !code_only.contains("if ("),
        "ERRORS.md claims the C source is branch-free, but an `if (` appeared"
    );

    // Exactly one `return` in the whole function.
    assert_eq!(
        code_only.matches("return").count(),
        1,
        "expected exactly one return statement in float2half"
    );

    // The public API really is one function with one non-pointer scalar
    // parameter, so there are no null-pointer / length / enum error rows.
    assert!(
        h.contains("uint16_t float2half(float flt);"),
        "public header changed; ERRORS.md rows about the absent pointer/length/enum \
         surface must be re-derived. Header was:\n{h}"
    );
    assert!(!h.contains('*'), "header now has a pointer parameter");
    assert_eq!(
        h.matches('(').count(),
        1,
        "header now declares more than one function"
    );
}

/// E2..E10, E17 — special IEEE-754 values, including signalling NaNs whose
/// payload must not be quieted while crossing the FFI boundary, and the exact
/// expected results for the infinities and zeros.
#[test]
fn errors_special_values() {
    let p = pair();

    // (bits, description, expected C result if fixed by the tables)
    let exact: [(u32, &str, u16); 4] = [
        (0x7f80_0000, "+inf (E7)", 0x7c00),
        (0xff80_0000, "-inf (E8)", 0xfc00),
        (0x0000_0000, "+0.0 (E9)", 0x0000),
        (0x8000_0000, "-0.0 (E10)", 0x8000),
    ];
    for (bits, what, want) in exact {
        let got_c = unsafe { (p.c)(f32::from_bits(bits)) };
        let got_rust = unsafe { (p.rust)(f32::from_bits(bits)) };
        assert_eq!(got_c, want, "{what}: C returned 0x{got_c:04x}, expected 0x{want:04x}");
        assert_eq!(
            got_rust, got_c,
            "{what}: Rust returned 0x{got_rust:04x} but C returned 0x{got_c:04x}"
        );
    }

    // NaNs: no fixed expectation is asserted, only C/Rust equality, because the
    // C tables define the answer and the C is ground truth.
    let nans: [(u32, &str); 8] = [
        (0x7fc0_0000, "+qNaN (E2)"),
        (0xffc0_0000, "-qNaN (E3)"),
        (0x7f80_0001, "+sNaN minimal payload (E4)"),
        (0xff80_0001, "-sNaN minimal payload (E5)"),
        (0x7fbf_ffff, "+sNaN maximal payload (E4)"),
        (0xffbf_ffff, "-sNaN maximal payload (E5)"),
        (0x7fff_ffff, "+qNaN all payload bits (E6)"),
        (0xffff_ffff, "-qNaN all payload bits (E6)"),
    ];
    for (bits, what) in nans {
        let x = f32::from_bits(bits);
        assert_eq!(
            x.to_bits(),
            bits,
            "{what}: sNaN payload was altered before the call (0x{:08x})",
            x.to_bits()
        );
        let got_c = unsafe { (p.c)(x) };
        let got_rust = unsafe { (p.rust)(x) };
        assert_eq!(
            got_c, got_rust,
            "{what} (bits 0x{bits:08x}): C=0x{got_c:04x} Rust=0x{got_rust:04x}"
        );
    }

    // E6 specifically: the all-ones NaN is the maximum-mantissa case in the
    // shift-13 infinity bucket, which is where the uint16_t sum is largest.
    let max_pos = unsafe { (p.c)(f32::from_bits(0x7fff_ffff)) };
    let max_neg = unsafe { (p.c)(f32::from_bits(0xffff_ffff)) };
    assert_eq!(max_pos, 0x7fff, "E6: expected 0x7fff for 0x7fffffff");
    assert_eq!(max_neg, 0xffff, "E6: expected 0xffff for 0xffffffff");
    assert_eq!(max_pos, unsafe { (p.rust)(f32::from_bits(0x7fff_ffff)) });
    assert_eq!(max_neg, unsafe { (p.rust)(f32::from_bits(0xffff_ffff)) });

    // E17 — the C ABI returns uint16_t in `ax`; both libraries must agree on the
    // full 16 bits for every special value above.
    for (bits, _) in nans {
        p.check_bits(bits);
    }
}

/// E11, E12, E13 — one step past each documented valid range boundary:
/// overflow to half infinity, underflow to half zero, and the
/// subnormal/normal exponent-bucket seams.
#[test]
fn errors_range_boundaries() {
    let p = pair();

    // E11 — overflow side.
    //
    // NOTE: this table *truncates* the mantissa (`>> shift`, round-toward-zero)
    // rather than rounding to nearest, so it does NOT behave like a
    // round-to-nearest float->half conversion. Everything in the whole binade
    // [65504, 65536) maps to 0x7bff (the largest finite half), and the jump to
    // half infinity happens exactly at 65536.0, where the exponent bucket
    // advances from j=142 to j=143. These expectations were read back from the C
    // library, which is ground truth.
    let overflow: [(f32, u16); 8] = [
        (65504.0, 0x7bff),
        (65505.0, 0x7bff),
        (65519.0, 0x7bff),
        (65520.0, 0x7bff),
        (65535.0, 0x7bff),
        (65536.0, 0x7c00),
        (131072.0, 0x7c00),
        (f32::MAX, 0x7c00),
    ];
    for (v, want) in overflow {
        let got_c = unsafe { (p.c)(v) };
        let got_rust = unsafe { (p.rust)(v) };
        assert_eq!(got_c, want, "E11: C({v}) = 0x{got_c:04x}, expected 0x{want:04x}");
        assert_eq!(got_rust, got_c, "E11: Rust({v}) = 0x{got_rust:04x} != C 0x{got_c:04x}");
        // Negated form.
        let got_c_n = unsafe { (p.c)(-v) };
        let got_rust_n = unsafe { (p.rust)(-v) };
        assert_eq!(
            got_c_n,
            want | 0x8000,
            "E11: C({}) = 0x{got_c_n:04x}, expected 0x{:04x}",
            -v,
            want | 0x8000
        );
        assert_eq!(got_rust_n, got_c_n, "E11: Rust/-{v} divergence");
    }

    // E12 — underflow side.
    let underflow: [f32; 6] = [
        5.960_464_5e-8,           // smallest positive half subnormal
        2.980_232_2e-8,           // half of it: underflows
        1.0e-10,
        f32::MIN_POSITIVE,        // smallest normal f32
        f32::from_bits(1),        // smallest positive f32 subnormal
        f32::from_bits(0x0000_0f00),
    ];
    for v in underflow {
        p.check_f32(v);
        p.check_f32(-v);
    }
    // The extreme underflows must land exactly on signed zero.
    assert_eq!(unsafe { (p.c)(f32::from_bits(1)) }, 0x0000, "E12");
    assert_eq!(unsafe { (p.rust)(f32::from_bits(1)) }, 0x0000, "E12");
    assert_eq!(
        unsafe { (p.c)(f32::from_bits(0x8000_0001)) },
        0x8000,
        "E12 negative"
    );
    assert_eq!(
        unsafe { (p.rust)(f32::from_bits(0x8000_0001)) },
        0x8000,
        "E12 negative"
    );

    // E13 — the subnormal/normal exponent seams, both signs. Sweep the whole
    // mantissa range coarsely plus every edge value at each seam bucket.
    for j in [101u32, 102, 103, 111, 112, 113, 114, 357, 358, 359, 367, 368, 369, 370] {
        for m in [0u32, 1, 0x1fff, 0x2000, 0x3fff, 0x4000, 0x0040_0000, 0x007f_fffe, 0x007f_ffff] {
            p.check_bits(bits_for(j, m));
        }
        let mut rng = Rng::with_seed(0xB0BA_FE77 ^ u64::from(j));
        for _ in 0..2000 {
            p.check_bits(bits_for(j, rng.next_u32() & 0x007f_ffff));
        }
    }
}

/// E14 — every `m__shift` run boundary, plus the two anomalous shift-13
/// buckets (`j == 255` and `j == 511`) embedded in otherwise shift-24 regions.
#[test]
fn errors_shift_table_anomalies() {
    let p = pair();

    // The anomalous buckets: infinity/NaN, where shift is 13 not 24, so a NaN
    // mantissa is *shifted into* the result instead of being discarded.
    for (j, base) in [(255u32, 0x7c00u16), (511u32, 0xfc00u16)] {
        assert_eq!(
            observed_shift(p.c, j),
            13,
            "E14: expected the anomalous shift 13 at j={j}"
        );
        assert_eq!(observed_shift(p.rust, j), 13, "E14: Rust shift differs at j={j}");
        assert_eq!(observed_base(p.c, j), base, "E14: base at j={j}");
        assert_eq!(observed_base(p.rust, j), base, "E14: Rust base at j={j}");

        // Exhaustive over the 1024 attainable quotients, both low-bit extremes.
        for q in 0..1024u32 {
            p.check_bits(bits_for(j, q << 13));
            p.check_bits(bits_for(j, (q << 13) | 0x1fff));
        }
    }

    // Every run boundary: the last j of each run and the first j of the next.
    for (lo, hi, shift) in SHIFT_RUNS {
        for j in [lo, hi, lo.saturating_sub(1), (hi + 1).min(511)] {
            let cs = observed_shift(p.c, j);
            let rs = observed_shift(p.rust, j);
            assert_eq!(cs, rs, "E14: shift mismatch at run-boundary j={j}");
            for m in [0u32, 1, 0x007f_ffff, 0x0040_0000, (1 << shift.min(23)) - 1] {
                p.check_bits(bits_for(j, m & 0x007f_ffff));
            }
        }
    }
}

/// E15 — the narrowing `(uint16_t)` cast. The maximum attainable
/// `base + (mantissa >> shift)` is exactly `0xffff`, so nothing is ever
/// discarded and nothing ever wraps; verify that mechanically across all 512
/// buckets and confirm the maximum is attained at `j == 511`.
#[test]
fn errors_truncating_cast() {
    let p = pair();

    let mut max_seen: u32 = 0;
    let mut argmax: u32 = 0;

    for j in 0..512u32 {
        let base = observed_base(p.c, j) as u32;
        let shift = observed_shift(p.c, j);
        let mantissa_term = 0x007f_ffffu32 >> shift.min(31);
        let sum = base + mantissa_term;

        assert!(
            sum <= 0xffff,
            "E15: base[{j}]=0x{base:04x} + 0x{mantissa_term:x} = 0x{sum:x} exceeds 0xffff; \
             ERRORS.md must be updated"
        );
        if sum > max_seen {
            max_seen = sum;
            argmax = j;
        }

        // The all-ones mantissa realises that maximum for this bucket; both
        // libraries must produce it without panicking or wrapping.
        let got = p.agreed(bits_for(j, 0x007f_ffff));
        assert_eq!(
            got as u32, sum,
            "E15: float2half(j={j}, m=0x7fffff) = 0x{got:04x} but base+mant = 0x{sum:04x}"
        );
    }

    assert_eq!(max_seen, 0xffff, "E15: expected the global maximum to be 0xffff");
    assert_eq!(argmax, 511, "E15: expected the maximum at j=511");
}

/// E16 — the `float` analogue of an out-of-range enum value: bit patterns that
/// are not canonical floats. All 512 index buckets must be reachable and
/// answered identically, including with non-canonical / pseudo-denormal
/// mantissas. This is also the out-of-bounds-index guard: `j` is masked to 9
/// bits in C, so no input can index past the 512-entry tables — and the Rust
/// must likewise never panic on a bounds check.
#[test]
fn errors_all_512_buckets_reachable() {
    let p = pair();
    let mut seen = [false; 512];
    let mut rng = Rng::with_seed(0xFACE_FEED_CAFE_BABE);

    for j in 0..512u32 {
        // Non-canonical mantissas: a "pseudo-denormal" (non-zero mantissa with
        // a zero-exponent bucket), and mantissas with scattered high bits.
        for m in [
            0x0000_0000u32,
            0x0000_0001,
            0x0000_00ff,
            0x0002_aaaa,
            0x0055_5555,
            0x0040_0000,
            0x003f_ffff,
            0x007f_ffff,
        ] {
            let bits = bits_for(j, m);
            p.check_bits(bits);
            assert_eq!((bits >> 23) & 0x1ff, j, "index reconstruction failed");
        }
        for _ in 0..32 {
            p.check_bits(bits_for(j, rng.next_u32() & 0x007f_ffff));
        }
        seen[j as usize] = true;
    }

    assert!(seen.iter().all(|&x| x), "not all 512 buckets were exercised");

    // One step past the largest valid index: the mask must fold it back around.
    // j = 512 is unrepresentable in 9 bits, so bits (512 << 23) aliases j = 0.
    let aliased = 512u32.wrapping_shl(23);
    assert_eq!((aliased >> 23) & 0x1ff, 0, "512 must alias to bucket 0");
    p.check_bits(aliased);
    p.check_bits(aliased | 0x007f_ffff);
    // And a fully saturated 32-bit pattern, i.e. j = 511.
    assert_eq!((0xffff_ffffu32 >> 23) & 0x1ff, 511);
    p.check_bits(0xffff_ffff);
}

/// Generic boundary sweep every C API deserves, even where the table has no
/// row: the numerically extreme inputs and the first/last bit pattern of every
/// exponent bucket.
#[test]
fn errors_generic_boundaries() {
    let p = pair();

    // First and last bit pattern of each of the 256 exponent values, both signs.
    for sign in [0u32, 1] {
        for exp in 0..256u32 {
            for m in [0u32, 1, 0x007f_fffe, 0x007f_ffff] {
                p.check_bits((sign << 31) | (exp << 23) | m);
            }
        }
    }

    // Absolute extremes of the 32-bit space.
    for bits in [
        0x0000_0000u32,
        0x0000_0001,
        0x7fff_fffe,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_fffe,
        0xffff_ffff,
    ] {
        p.check_bits(bits);
    }
}
