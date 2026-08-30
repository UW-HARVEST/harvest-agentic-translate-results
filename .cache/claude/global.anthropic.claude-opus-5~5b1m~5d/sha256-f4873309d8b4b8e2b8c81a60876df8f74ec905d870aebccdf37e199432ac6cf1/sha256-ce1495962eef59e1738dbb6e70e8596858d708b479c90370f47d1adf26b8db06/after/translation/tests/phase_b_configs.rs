//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported symbols and asserts byte-identical stdout.
//! Randomized rows use the fixed-seed RNG from `common` for reproducibility.

mod common;

use common::*;
use std::ffi::c_char;

// ===========================================================================
// C1..C6 — printLine (lowest level output primitive), `const char *` shapes
// ===========================================================================

#[test]
fn c1_printline_random_ascii() {
    let mut rng = Rng::new();
    let corpus: Vec<Vec<u8>> = (0..512)
        .map(|_| {
            let len = 1 + rng.below(64) as usize;
            (0..len).map(|_| 0x20 + (rng.below(95) as u8)).collect()
        })
        .collect();
    for (i, bytes) in corpus.iter().enumerate() {
        let s = cstring(bytes);
        diff_one(&format!("C1/printLine random ascii #{i}"), |api| {
            (api.print_line)(s.as_ptr())
        });
    }
}

#[test]
fn c2_printline_empty_string() {
    let s = cstring(b"");
    let out = diff_one("C2/printLine empty", |api| (api.print_line)(s.as_ptr()));
    assert_eq!(out, b"\n", "empty string must still emit the newline");
}

#[test]
fn c3_printline_every_single_byte() {
    // 0x01..=0xFF, including bytes that are not valid UTF-8.
    for b in 1u8..=255 {
        let s = cstring(&[b]);
        let out = diff_one(&format!("C3/printLine byte {b:#04x}"), |api| {
            (api.print_line)(s.as_ptr())
        });
        assert_eq!(out, vec![b, b'\n']);
    }
}

#[test]
fn c4_printline_oversized() {
    for &len in &[1024usize, 4095, 4096, 4097, 8192, 65536] {
        let bytes: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let s = cstring(&bytes);
        let out = diff_one(&format!("C4/printLine oversized {len}"), |api| {
            (api.print_line)(s.as_ptr())
        });
        assert_eq!(out.len(), len + 1);
    }
}

#[test]
fn c5_printline_format_specifiers() {
    // `line` is the %s ARGUMENT, never the format string, so these must be
    // reproduced verbatim by both implementations. (Also ERRORS.md E13.)
    let cases: &[&[u8]] = &[
        b"%d",
        b"%s",
        b"%n",
        b"%%",
        b"%p %x %o",
        b"100%% done",
        b"%s%s%s%s%s%s%s%s",
        b"%1000000d",
        b"%.*f",
        b"a%db%sc%nd",
    ];
    for case in cases {
        let s = cstring(case);
        let out = diff_one(&format!("C5/printLine {:?}", String::from_utf8_lossy(case)), |api| {
            (api.print_line)(s.as_ptr())
        });
        let mut want = case.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "format specifiers must print verbatim");
    }
}

#[test]
fn c6_printline_embedded_whitespace() {
    let cases: &[&[u8]] = &[
        b"a\nb",
        b"\n",
        b"\n\n\n",
        b"tab\there",
        b"line1\nline2\nline3",
        b"trailing\n",
        b"\r\n",
    ];
    for case in cases {
        let s = cstring(case);
        diff_one(&format!("C6/printLine {case:?}"), |api| {
            (api.print_line)(s.as_ptr())
        });
    }
}

// ===========================================================================
// C7..C8 — printIntLine, `int` shapes
// ===========================================================================

#[test]
fn c7_printintline_boundaries() {
    let values: &[i32] = &[
        0,
        1,
        -1,
        2,
        -2,
        9,
        10,
        -10,
        99,
        100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        2147483647,
        -2147483647,
        -2147483648,
    ];
    diff_samples("C7/printIntLine boundaries", values, |api, v| {
        (api.print_int_line)(v)
    });
}

#[test]
fn c8_printintline_random_full_range() {
    let mut rng = Rng::new();
    let values: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();
    diff_samples("C8/printIntLine random", &values, |api, v| {
        (api.print_int_line)(v)
    });
}

// ===========================================================================
// C9..C16 — bad(float): the unguarded 100.0/data division and (int) cast
// ===========================================================================

#[test]
fn c9_bad_exact_quotients() {
    let values: &[f32] = &[2.0, 4.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 1.0, 0.5, 0.25];
    diff_samples("C9/bad exact", values, |api, v| (api.bad)(v));
    // Sanity anchor against the C's documented behaviour.
    let out = diff_one("C9/bad(2.0)", |api| (api.bad)(2.0));
    assert_eq!(out, b"50\n");
}

#[test]
fn c10_bad_truncating_positive() {
    let values: &[f32] = &[3.0, 6.0, 7.0, 8.0, 9.0, 11.0, 13.0, 30.0, 33.0, 99.0, 101.0, 150.0];
    diff_samples("C10/bad truncating", values, |api, v| (api.bad)(v));
    let out = diff_one("C10/bad(3.0)", |api| (api.bad)(3.0));
    assert_eq!(out, b"33\n", "truncation toward zero");
}

#[test]
fn c11_bad_negative() {
    let values: &[f32] = &[-1.0, -2.0, -3.0, -4.0, -7.0, -100.0, -0.5, -101.0, -150.0, -1e3];
    diff_samples("C11/bad negative", values, |api, v| (api.bad)(v));
    let out = diff_one("C11/bad(-3.0)", |api| (api.bad)(-3.0));
    assert_eq!(out, b"-33\n", "negative truncation is toward zero");
}

#[test]
fn c12_bad_quotient_below_one() {
    let values: &[f32] = &[
        1e3, 1e4, 1e6, 1e10, 1e30, f32::MAX, -1e3, -1e6, -1e30, f32::MIN, 200.0, -200.0, 101.0,
    ];
    diff_samples("C12/bad |quotient|<1", values, |api, v| (api.bad)(v));
    let out = diff_one("C12/bad(1e6)", |api| (api.bad)(1e6));
    assert_eq!(out, b"0\n");
}

#[test]
fn c13_bad_quotient_overflows_int() {
    let values: &[f32] = &[
        1e-8, 1e-9, 1e-12, 1e-20, 1e-30, 1e-38, 1e-45, f32::MIN_POSITIVE, -1e-8, -1e-12, -1e-30,
        -1e-45, -f32::MIN_POSITIVE,
    ];
    diff_samples("C13/bad overflow", values, |api, v| (api.bad)(v));
    let out = diff_one("C13/bad(1e-30)", |api| (api.bad)(1e-30));
    assert_eq!(
        out, b"-2147483648\n",
        "out-of-range double->int yields the x86-64 integer-indefinite value"
    );
}

#[test]
fn c14_bad_special_values() {
    diff_samples("C14/bad specials", INTERESTING_FLOATS, |api, v| (api.bad)(v));
}

#[test]
fn c15_bad_random_decades() {
    let mut rng = Rng::new();
    let values: Vec<f32> = (0..1024).map(|_| rng.next_f32_decades()).collect();
    diff_samples("C15/bad random decades", &values, |api, v| (api.bad)(v));
}

#[test]
fn c16_bad_random_bit_patterns() {
    let mut rng = Rng::new();
    let values: Vec<f32> = (0..512).map(|_| rng.next_f32_bits()).collect();
    diff_samples("C16/bad random bits", &values, |api, v| (api.bad)(v));
}

// ===========================================================================
// C17..C21 — good(float): goodG2B (constant) + goodB2G (threshold branch)
// ===========================================================================

#[test]
fn c17_good_threshold_true() {
    let values: &[f32] = &[1.0, 2.0, 3.0, -3.0, 100.0, -100.0, 0.5, 1e-3, -1e-3, 1e-5, 1e3, 1e30];
    diff_samples("C17/good branch TRUE", values, |api, v| (api.good)(v));
    let out = diff_one("C17/good(2.0)", |api| (api.good)(2.0));
    assert_eq!(out, b"50\n50\n", "goodG2B prints 50, then goodB2G prints 50");
}

#[test]
fn c18_good_threshold_false() {
    let values: &[f32] = &[0.0, -0.0, 1e-9, -1e-9, 1e-30, -1e-45, f32::NAN, -f32::NAN, 1e-7];
    diff_samples("C18/good branch FALSE", values, |api, v| (api.good)(v));
    let out = diff_one("C18/good(0.0)", |api| (api.good)(0.0));
    assert_eq!(out, b"50\nThis would result in a divide by zero\n");
}

#[test]
fn c19_good_threshold_straddle() {
    // The literal 0.000001 is a DOUBLE. (double)1e-6f == 9.99999997475e-07,
    // which is BELOW it, so `good(1e-6f)` takes the FALSE branch.
    let values: &[f32] = &[
        1e-6,
        -1e-6,
        1.0000001e-6,
        -1.0000001e-6,
        9.9e-7,
        1.1e-6,
        -1.1e-6,
        1.000001e-6,
        1.00001e-6,
        f32::from_bits(1e-6f32.to_bits() + 1),
        f32::from_bits(1e-6f32.to_bits() - 1),
        f32::from_bits(1e-6f32.to_bits() + 2),
    ];
    diff_samples("C19/good threshold straddle", values, |api, v| (api.good)(v));

    let out = diff_one("C19/good(1e-6f)", |api| (api.good)(1e-6));
    assert_eq!(
        out, b"50\nThis would result in a divide by zero\n",
        "(double)1e-6f is strictly less than the double literal 0.000001"
    );
    let out = diff_one("C19/good(nextafter(1e-6f))", |api| {
        (api.good)(f32::from_bits(1e-6f32.to_bits() + 1))
    });
    // 100.0 / 1.00000001e-6 ~= 9.9999988e7, truncated to 99999988.
    assert_eq!(out, b"50\n99999988\n", "one ULP up crosses the threshold");
}

#[test]
fn c20_good_random_decades() {
    let mut rng = Rng::new();
    let values: Vec<f32> = (0..1024).map(|_| rng.next_f32_decades()).collect();
    diff_samples("C20/good random decades", &values, |api, v| (api.good)(v));
}

#[test]
fn c21_good_random_bit_patterns() {
    let mut rng = Rng::new();
    let values: Vec<f32> = (0..512).map(|_| rng.next_f32_bits()).collect();
    diff_samples("C21/good random bits", &values, |api, v| (api.good)(v));
}

// ===========================================================================
// C22..C27 — driver(float, float): the composed top-level pipeline
// ===========================================================================

#[test]
fn c22_driver_good_true_bad_normal() {
    let out = diff_one("C22/driver(2.0, 4.0)", |api| (api.driver)(2.0, 4.0));
    assert_eq!(
        out,
        b"Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n25\nFinished bad()\n",
        "full transcript verifies call ORDER through the pipeline"
    );
}

#[test]
fn c23_driver_good_false_bad_normal() {
    let out = diff_one("C23/driver(0.0, 4.0)", |api| (api.driver)(0.0, 4.0));
    let want: &[u8] = b"Calling good()...\n50\nThis would result in a divide by zero\n\
                        Finished good()\nCalling bad()...\n25\nFinished bad()\n";
    assert_eq!(out, want);
}

#[test]
fn c24_driver_bad_divide_by_zero() {
    let out = diff_one("C24/driver(2.0, 0.0)", |api| (api.driver)(2.0, 0.0));
    assert_eq!(
        out,
        b"Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n-2147483648\nFinished bad()\n",
        "the CWE-369 path: bad() divides by zero and prints INT_MIN"
    );
}

#[test]
fn c25_driver_good_false_bad_special() {
    let bads: &[f32] = &[0.0, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-45];
    diff_samples("C25/driver(0.0, special)", bads, |api, b| {
        (api.driver)(0.0, b)
    });
}

#[test]
fn c26_driver_interesting_cross_product() {
    // Full cross-product of the interesting-float set with itself.
    let pairs: Vec<(f32, f32)> = INTERESTING_FLOATS
        .iter()
        .flat_map(|&g| INTERESTING_FLOATS.iter().map(move |&b| (g, b)))
        .collect();
    assert_eq!(pairs.len(), INTERESTING_FLOATS.len() * INTERESTING_FLOATS.len());
    diff_samples("C26/driver cross-product", &pairs, |api, (g, b)| {
        (api.driver)(g, b)
    });
}

#[test]
fn c27_driver_random_pairs() {
    let mut rng = Rng::new();
    let pairs: Vec<(f32, f32)> = (0..512)
        .map(|i| {
            if i % 3 == 0 {
                (rng.next_f32_bits(), rng.next_f32_bits())
            } else {
                (rng.next_f32_decades(), rng.next_f32_decades())
            }
        })
        .collect();
    diff_samples("C27/driver random pairs", &pairs, |api, (g, b)| {
        (api.driver)(g, b)
    });
}

// ===========================================================================
// C28 — interleaving: many different entry points inside ONE capture
// ===========================================================================

#[test]
fn c28_interleaved_entry_points() {
    let mut rng = Rng::new();
    let msg = cstring(b"interleave");
    let msg2 = cstring(b"");

    // Build one long deterministic script mixing every exported symbol, so any
    // cross-call state, ordering or buffering divergence shows up.
    #[derive(Copy, Clone, Debug)]
    enum Op {
        Line,
        Empty,
        Null,
        Int(i32),
        Bad(f32),
        Good(f32),
        Driver(f32, f32),
    }

    let script: Vec<Op> = (0..600)
        .map(|_| match rng.below(7) {
            0 => Op::Line,
            1 => Op::Empty,
            2 => Op::Null,
            3 => Op::Int(rng.next_i32()),
            4 => Op::Bad(rng.next_f32_decades()),
            5 => Op::Good(rng.next_f32_decades()),
            _ => Op::Driver(rng.next_f32_decades(), rng.next_f32_bits()),
        })
        .collect();

    let run = |api: &Api| {
        for op in &script {
            match *op {
                Op::Line => (api.print_line)(msg.as_ptr()),
                Op::Empty => (api.print_line)(msg2.as_ptr()),
                Op::Null => (api.print_line)(std::ptr::null::<c_char>()),
                Op::Int(v) => (api.print_int_line)(v),
                Op::Bad(v) => (api.bad)(v),
                Op::Good(v) => (api.good)(v),
                Op::Driver(g, b) => (api.driver)(g, b),
            }
        }
    };

    let c = capture(|| run(c_api()));
    let r = capture(|| run(rust_api()));
    assert_eq!(
        c,
        r,
        "\n[C28] interleaved script diverged\n  C    = {}\n  Rust = {}\n",
        show(&c),
        show(&r)
    );
    assert!(!c.is_empty());
}
