// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row drives BOTH the C `.so` and the Rust
// `.so` through `dlsym`'d `driver` and asserts byte-identical stdout, over many
// randomized inputs from a fixed seed.

use crate::common::*;

// C1 -- positive normal doubles, randomized across the whole normal range.
pub fn c01_positive_normals_randomized() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut v = Vec::new();
    for _ in 0..8000 {
        // exponent 1..=2046 keeps us in the normal range
        let exp = 1 + rng.below(2046);
        v.push(compose(0, exp, rng.mantissa()));
    }
    assert_same("C1", &v);
}

// C2 -- the same shapes with the sign bit set.
pub fn c02_negative_normals_randomized() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut v = Vec::new();
    for _ in 0..8000 {
        let exp = 1 + rng.below(2046);
        v.push(compose(1, exp, rng.mantissa()));
    }
    assert_same("C2", &v);
}

// C3 -- zero mantissa: exact powers of two at every exponent, both signs.
// Drives glibc's `%a` trailing-zero trimming to the fully trimmed `0x1p+N`.
pub fn c03_exact_powers_of_two_all_exponents() {
    let mut v = Vec::new();
    for sign in 0..2 {
        for exp in 0..2048 {
            v.push(compose(sign, exp, 0));
        }
        // subnormal powers of two: a single mantissa bit set, exponent field 0
        for bit in 0..52 {
            v.push(compose(sign, 0, 1u64 << bit));
        }
    }
    assert_same("C3", &v);
}

// C4 -- full 52-bit mantissa: `%a` prints all 13 hex digits, nothing trimmed.
pub fn c04_full_mantissa_no_trimming() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut v = Vec::new();
    for sign in 0..2 {
        for exp in 0..2048 {
            v.push(compose(sign, exp, 0x000F_FFFF_FFFF_FFFF));
        }
        for _ in 0..2000 {
            // force the low bit set so no trailing hex digit can be trimmed
            let m = rng.mantissa() | 1;
            v.push(compose(sign, 1 + rng.below(2046), m));
        }
    }
    assert_same("C4", &v);
}

// C5 -- partial trailing-zero runs: exercises every intermediate `%a` trim
// length (1..=12 trailing zero hex digits).
pub fn c05_partial_trailing_zero_mantissa_runs() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut v = Vec::new();
    for zeros in 1..=13u32 {
        let mask = if zeros >= 13 {
            0
        } else {
            0x000F_FFFF_FFFF_FFFFu64 & !((1u64 << (4 * zeros)) - 1)
        };
        for _ in 0..400 {
            let m = rng.mantissa() & mask;
            let exp = 1 + rng.below(2046);
            v.push(compose(rng.next_u64() & 1, exp, m));
            // and the same mantissa shape in the subnormal range
            v.push(compose(rng.next_u64() & 1, 0, m));
        }
    }
    assert_same("C5", &v);
}

// C6 -- signed zeros.
pub fn c06_signed_zeros() {
    assert_same("C6", &[0x0000_0000_0000_0000, 0x8000_0000_0000_0000]);
}

// C7 -- infinities.
pub fn c07_infinities() {
    assert_same("C7", &[0x7FF0_0000_0000_0000, 0xFFF0_0000_0000_0000]);
}

// C8 -- the whole NaN family: quiet/signalling x sign x randomized payloads.
// `%llx` must reproduce the payload bits exactly while `%a`/`%.4f` collapse to
// `nan`/`-nan`.
pub fn c08_nan_family_randomized_payloads() {
    let mut rng = Rng::new(SEED ^ 8);
    let mut v = vec![
        0x7FF8_0000_0000_0000, // +quiet NaN (canonical)
        0xFFF8_0000_0000_0000, // -quiet NaN
        0x7FF0_0000_0000_0001, // +signalling NaN (minimal payload)
        0xFFF0_0000_0000_0001, // -signalling NaN
        0x7FFF_FFFF_FFFF_FFFF, // all payload bits set, quiet
        0xFFF7_FFFF_FFFF_FFFF, // max signalling payload, negative
    ];
    for _ in 0..3000 {
        let sign = rng.next_u64() & 1;
        let payload = rng.mantissa();
        // quiet: mantissa MSB set; signalling: MSB clear but payload nonzero
        let quiet = payload | (1u64 << 51);
        let sig = (payload & !(1u64 << 51)) | 1;
        v.push(compose(sign, 0x7FF, quiet));
        v.push(compose(sign, 0x7FF, sig));
    }
    assert_same("C8", &v);
}

// C9 -- subnormals: `%a` switches to the `0x0.…p-1022` form.
pub fn c09_subnormals_randomized() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut v = vec![
        0x0000_0000_0000_0001, // smallest positive subnormal
        0x8000_0000_0000_0001, // smallest negative subnormal
        0x000F_FFFF_FFFF_FFFF, // largest positive subnormal
        0x800F_FFFF_FFFF_FFFF, // largest negative subnormal
    ];
    for _ in 0..4000 {
        // nonzero mantissa keeps it a subnormal rather than a zero
        let m = rng.mantissa() | 1;
        v.push(compose(rng.next_u64() & 1, 0, m));
    }
    assert_same("C9", &v);
}

// C10 -- class boundaries and their nextafter neighbours on both sides.
pub fn c10_class_boundaries_with_neighbours() {
    let anchors: Vec<f64> = vec![
        f64::MIN_POSITIVE,               // smallest normal
        -f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        0.0,
        -0.0,
        f64::from_bits(0x000F_FFFF_FFFF_FFFF), // largest subnormal
        f64::EPSILON,
        1.0 - f64::EPSILON / 2.0,
    ];
    let mut v = Vec::new();
    for a in anchors {
        v.push(a.to_bits());
        // step outward and inward across each boundary
        for dir in [f64::INFINITY, f64::NEG_INFINITY] {
            let mut x = a;
            for _ in 0..3 {
                x = next_after(x, dir);
                v.push(x.to_bits());
            }
        }
    }
    assert_same("C10", &v);
}

// C11 -- huge magnitudes: `%.4f` emits up to ~310 integer digits.
pub fn c11_huge_magnitudes_long_fixed_output() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut v = vec![f64::MAX.to_bits(), f64::MIN.to_bits()];
    // exponent field for 1e50 is ~1189; sweep from there up to the top finite.
    for _ in 0..3000 {
        let exp = 1189 + rng.below(2046 - 1189 + 1);
        v.push(compose(rng.next_u64() & 1, exp, rng.mantissa()));
    }
    // Confirm we really are producing the long-output case.
    let i = impls();
    let out = capture_stdout(|| unsafe { (i.c)(f64::MAX) });
    assert!(
        out.len() > 300,
        "expected DBL_MAX to yield a long %.4f expansion, got {} bytes",
        out.len()
    );
    assert_same("C11", &v);
}

// C12 -- tiny magnitudes: `%.4f` underflows to 0.0000 / -0.0000, sign retained.
pub fn c12_tiny_magnitudes_signed_underflow() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut v = Vec::new();
    // exponent field for 1e-5 is ~1006; sweep everything below it, incl. subnormals.
    for _ in 0..4000 {
        let exp = rng.below(1007);
        v.push(compose(rng.next_u64() & 1, exp, rng.mantissa()));
    }
    for x in [1e-6f64, -1e-6, 1e-300, -1e-300, 5e-324, -5e-324] {
        v.push(x.to_bits());
    }
    assert_same("C12", &v);
}

// C13 -- `%.4f` round-half-to-even ties, resolved off the exact binary value.
pub fn c13_round_half_even_ties() {
    let mut v: Vec<u64> = Vec::new();
    let bases: [f64; 16] = [
        0.00005, 0.00015, 0.00025, 0.00035, 0.00045, 0.00055, 0.000125, 0.000375,
        1.00005, 1.00015, 2.00005, 12.34565, 12.34575, 99999.00005, 0.5, 1.5,
    ];
    for b in bases {
        for s in [1.0f64, -1.0] {
            let x = b * s;
            v.push(x.to_bits());
            // the neighbours are what separate "exact tie" from "just off a tie"
            let mut up = x;
            let mut down = x;
            for _ in 0..4 {
                up = next_after(up, f64::INFINITY);
                down = next_after(down, f64::NEG_INFINITY);
                v.push(up.to_bits());
                v.push(down.to_bits());
            }
        }
    }
    // exact binary ties at the 4th fractional digit: k/2^n forms
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..2000 {
        let k = rng.below(1 << 20) as f64;
        for d in [16.0f64, 32.0, 1024.0, 65536.0] {
            let x = k / d;
            v.push(x.to_bits());
            v.push((-x).to_bits());
        }
    }
    assert_same("C13", &v);
}

// C14 -- `%a` exponent sign flip around 1.0: p+N, p+0, p-N.
pub fn c14_hexfloat_exponent_sign_flip() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut v = Vec::new();
    // exponent field 1023 == p+0; sweep a window either side of it
    for exp in (1023 - 40)..=(1023 + 40) {
        for sign in 0..2 {
            v.push(compose(sign, exp, 0));
            v.push(compose(sign, exp, 0x000F_FFFF_FFFF_FFFF));
            for _ in 0..4 {
                v.push(compose(sign, exp, rng.mantissa()));
            }
        }
    }
    assert_same("C14", &v);
}

// C15 -- exhaustive exponent-field sweep: all 2048 encodings x both signs x
// several randomized mantissas each.
pub fn c15_exhaustive_exponent_sweep() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut v = Vec::new();
    for exp in 0..2048u64 {
        for sign in 0..2u64 {
            v.push(compose(sign, exp, 0));
            v.push(compose(sign, exp, 0x000F_FFFF_FFFF_FFFF));
            for _ in 0..3 {
                v.push(compose(sign, exp, rng.mantissa()));
            }
        }
    }
    assert_same("C15", &v);
}

// C16 -- full-domain randomized raw bit-pattern sweep. Uniform u64s reinterpreted
// as doubles, so every class appears in its natural proportion, including
// patterns no source literal can name.
pub fn c16_full_domain_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 16);
    let v: Vec<u64> = (0..40_000).map(|_| rng.next_u64()).collect();
    assert_same("C16", &v);
}

// C17 -- repeated/sequential invocation: N calls must yield N lines, in order,
// with no state carried between calls.
pub fn c17_repeated_sequential_calls_are_stateless() {
    let mut rng = Rng::new(SEED ^ 17);
    let v: Vec<u64> = (0..500).map(|_| rng.next_u64()).collect();
    assert_same("C17", &v);

    let i = impls();

    // N calls => exactly N lines, for both implementations.
    for (name, d) in [("C", i.c), ("Rust", i.rust)] {
        let out = capture_stdout(|| {
            for &b in &v {
                unsafe { d(f64::from_bits(b)) };
            }
        });
        let lines = out.iter().filter(|&&c| c == b'\n').count();
        assert_eq!(lines, v.len(), "{name}: expected {} lines", v.len());
    }

    // Calling the same value repeatedly must reproduce the identical line, and
    // the same value must print the same thing regardless of what preceded it.
    let probe = f64::from_bits(0x400921FB54442D18); // pi
    let once_c = capture_stdout(|| unsafe { (i.c)(probe) });
    let once_r = capture_stdout(|| unsafe { (i.rust)(probe) });
    assert_eq!(once_c, once_r, "C17: single-call outputs differ");

    let after_c = capture_stdout(|| unsafe {
        (i.c)(f64::NAN);
        (i.c)(f64::MAX);
        (i.c)(probe);
    });
    let after_r = capture_stdout(|| unsafe {
        (i.rust)(f64::NAN);
        (i.rust)(f64::MAX);
        (i.rust)(probe);
    });
    assert_eq!(after_c, after_r, "C17: outputs differ after preceding calls");
    assert!(
        after_c.ends_with(&once_c),
        "C17: pi's output changed depending on preceding calls (library is not stateless)"
    );
}

// C18 -- interleaving with the caller's own stdout writes. Both libraries must
// write through the *same* libc `stdout`, so ordering is preserved. A
// translation using Rust's `println!` (a separate buffer) would reorder here.
pub fn c18_interleaves_with_caller_stdio() {
    let i = impls();
    let mut rng = Rng::new(SEED ^ 18);
    let vals: Vec<u64> = (0..200).map(|_| rng.next_u64()).collect();

    let run = |d: DriverFn| {
        capture_stdout(|| {
            for (n, &b) in vals.iter().enumerate() {
                libc_print(&format!("before{n} "));
                unsafe { d(f64::from_bits(b)) };
                libc_print(&format!("after{n}\n"));
            }
        })
    };

    let c_out = run(i.c);
    let r_out = run(i.rust);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "C18: interleaved output ordering differs between C and Rust"
    );
    // The interleaving must actually be on the same line, proving shared buffering.
    let s = String::from_utf8_lossy(&c_out);
    assert!(
        s.starts_with("before0 "),
        "C18: caller's text did not precede driver's output: {:?}",
        &s[..s.len().min(80)]
    );
    assert!(
        s.contains("after0\nbefore1 "),
        "C18: expected strict interleaving in the captured stream"
    );
}

// C19 -- non-"C" LC_NUMERIC locale: `%.4f`'s decimal separator must agree.
pub fn c19_non_c_locale_decimal_point() {
    let candidates = [
        "de_DE.UTF-8",
        "de_DE.utf8",
        "fr_FR.UTF-8",
        "fr_FR.utf8",
        "es_ES.UTF-8",
        "nl_NL.UTF-8",
    ];
    let mut chosen = None;
    for c in candidates {
        if try_setlocale(LC_ALL, c) {
            chosen = Some(c);
            break;
        }
    }
    let Some(loc) = chosen else {
        eprintln!("C19: no comma-decimal locale installed; testing C locale only");
        try_setlocale(LC_ALL, "C");
        let mut rng = Rng::new(SEED ^ 19);
        let v: Vec<u64> = (0..2000).map(|_| rng.next_u64()).collect();
        assert_same("C19(C-locale-fallback)", &v);
        return;
    };
    eprintln!("C19: using locale {loc}");

    let mut rng = Rng::new(SEED ^ 19);
    let mut v: Vec<u64> = (0..3000).map(|_| rng.next_u64()).collect();
    // plus values whose %.4f definitely has a fractional part to separate
    for _ in 0..1000 {
        let exp = 1000 + rng.below(60);
        v.push(compose(rng.next_u64() & 1, exp, rng.mantissa()));
    }
    for x in [1.5f64, -2.25, 3.14159, 0.0001, 12345.6789] {
        v.push(x.to_bits());
    }
    assert_same("C19", &v);

    // restore, then re-verify under the C locale so ordering between tests
    // cannot leave a sticky locale behind
    try_setlocale(LC_ALL, "C");
    assert_same("C19(restored)", &v);
}

/// `nextafter` without pulling in libm: step by one ULP in the bit domain.
pub fn next_after(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() {
        return x + toward;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        // smallest subnormal with the sign of `toward`
        return if toward > 0.0 {
            f64::from_bits(1)
        } else {
            f64::from_bits(1 | (1u64 << 63))
        };
    }
    let bits = x.to_bits();
    let going_up = toward > x;
    let away_from_zero = (x > 0.0) == going_up;
    f64::from_bits(if away_from_zero { bits + 1 } else { bits - 1 })
}
