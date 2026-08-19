//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Every public function in this library
//! returns `void` and there are no error codes, so "the same rejection" means
//! byte-identical stdout, *including the case where the C emits nothing at all*.
//! Each test therefore also pins the expected C bytes, so that "both produced
//! nothing" cannot silently pass for the wrong reason.

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_char, c_int};

fn main() {
    common::run(cases![
        e01_print_line_null,
        e02_print_line_empty,
        e03_print_line_format_specifiers,
        e04_print_int_line_int_min,
        e05_print_int_line_int_max,
        e06_bad_positive_zero,
        e07_bad_negative_zero,
        e08_bad_quiet_nan,
        e09_bad_signalling_nan,
        e10_bad_tiny_overflows_int,
        e11_bad_tiny_negative_underflows_int,
        e12_bad_cvttsd2si_boundary,
        e13_bad_positive_infinity,
        e14_bad_negative_infinity,
        e15_good_positive_zero,
        e16_good_negative_zero,
        e17_good_nan_unordered,
        e18_good_subnormals_rejected,
        e19_good_guard_boundary_below,
        e20_good_guard_boundary_above,
        e21_good_guard_boundary_negative,
        e22_good_infinities,
        e23_driver_bad_data_rejected,
        e24_driver_both_rejected,
        e25_arbitrary_bit_patterns,
        // generic FFI boundary checks
        g01_null_pointer_repeated_and_mixed,
        g02_int_one_past_every_range,
        g03_float_one_ulp_past_every_documented_bound,
        g04_out_of_range_discriminants,
    ]);
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Differential comparison plus an assertion on the exact C bytes, so a row can
/// never pass because *both* implementations silently did nothing unexpected.
fn expect(what: &str, expected: &str, f: impl Fn(&Api)) {
    let l = libs();
    let (c_out, ()) = capture(|| f(&l.c.api));
    let (r_out, ()) = capture(|| f(&l.rs.api));
    assert_eq!(
        c_out,
        r_out,
        "\nDIVERGENCE for {what}\n  C   : \"{}\"\n  Rust: \"{}\"\n",
        esc(&c_out),
        esc(&r_out)
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        expected,
        "\nC reference bytes changed for {what}: got \"{}\"",
        esc(&c_out)
    );
}

const REJECT: &str = "This would result in a divide by zero\n";
const INT_MIN_LINE: &str = "-2147483648\n";
/// `good()` always prints `goodG2B`'s `100.0/2.0F` result first.
const G2B: &str = "50\n";

// ---------------------------------------------------------------------------
// ERRORS.md rows
// ---------------------------------------------------------------------------

/// ERRORS.md row E1 — `printLine(NULL)`: the `if (line != NULL)` guard at
/// driver.c:32 rejects the pointer; nothing at all is written and it must not
/// crash.
fn e01_print_line_null() {
    expect("E1 printLine(NULL)", "", |api| unsafe {
        (api.print_line)(std::ptr::null());
    });
}

/// ERRORS.md row E2 — degenerate but valid buffer: a lone NUL.
fn e02_print_line_empty() {
    expect("E2 printLine(\"\")", "\n", |api| unsafe {
        (api.print_line)(b"\0".as_ptr() as *const c_char);
    });
}

/// ERRORS.md row E3 — the payload consists of `printf` directives; C passes it
/// as an *argument*, so it must never be interpreted (no crash, no garbage).
fn e03_print_line_format_specifiers() {
    expect(
        "E3 printLine(\"%s %d %n %%\")",
        "%s %d %n %%\n",
        |api| unsafe {
            (api.print_line)(b"%s %d %n %%\0".as_ptr() as *const c_char);
        },
    );
    expect("E3 printLine(\"%n\")", "%n\n", |api| unsafe {
        (api.print_line)(b"%n\0".as_ptr() as *const c_char);
    });
    expect(
        "E3 printLine(\"%1000000d\")",
        "%1000000d\n",
        |api| unsafe {
            (api.print_line)(b"%1000000d\0".as_ptr() as *const c_char);
        },
    );
}

/// ERRORS.md row E4 — `printIntLine(INT_MIN)`, the extreme of the valid range.
fn e04_print_int_line_int_min() {
    expect("E4 printIntLine(INT_MIN)", INT_MIN_LINE, |api| unsafe {
        (api.print_int_line)(i32::MIN);
    });
}

/// ERRORS.md row E5 — `printIntLine(INT_MAX)`.
fn e05_print_int_line_int_max() {
    expect("E5 printIntLine(INT_MAX)", "2147483647\n", |api| unsafe {
        (api.print_int_line)(i32::MAX);
    });
}

/// ERRORS.md row E6 — the CWE-369 flaw: `bad(+0.0)` divides by zero, and the
/// resulting `(int)+INF` conversion is C-undefined; on x86-64 `cvttsd2si`
/// yields `INT_MIN`.
fn e06_bad_positive_zero() {
    expect("E6 bad(+0.0)", INT_MIN_LINE, |api| unsafe {
        (api.bad)(0.0);
    });
}

/// ERRORS.md row E7 — `bad(-0.0)` -> `-INF` -> `INT_MIN`.
fn e07_bad_negative_zero() {
    expect("E7 bad(-0.0)", INT_MIN_LINE, |api| unsafe {
        (api.bad)(-0.0);
    });
}

/// ERRORS.md row E8 — quiet NaN divisor, both signs and several payloads.
fn e08_bad_quiet_nan() {
    for (i, x) in [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0xffff_ffff),
        f32::from_bits(0x7fc0_1234),
    ]
    .into_iter()
    .enumerate()
    {
        expect(
            &format!("E8 bad(qNaN #{i} 0x{:08x})", x.to_bits()),
            INT_MIN_LINE,
            |api| unsafe { (api.bad)(x) },
        );
    }
}

/// ERRORS.md row E9 — signalling NaN divisor: `divsd` raises *invalid* and
/// quiets it; the conversion still yields `INT_MIN`.
fn e09_bad_signalling_nan() {
    for (i, x) in [
        SNAN,
        NEG_SNAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0x7fbf_ffff),
        f32::from_bits(0xff80_0001),
        f32::from_bits(0xffbf_ffff),
    ]
    .into_iter()
    .enumerate()
    {
        expect(
            &format!("E9 bad(sNaN #{i} 0x{:08x})", x.to_bits()),
            INT_MIN_LINE,
            |api| unsafe { (api.bad)(x) },
        );
    }
}

/// ERRORS.md row E10 — non-zero but tiny divisor: the quotient overflows `int`.
fn e10_bad_tiny_overflows_int() {
    for x in [
        1e-8f32,
        1e-20,
        1e-30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
        4.0e-8,
    ] {
        expect(
            &format!("E10 bad({x:e}) overflows int"),
            INT_MIN_LINE,
            |api| unsafe { (api.bad)(x) },
        );
    }
}

/// ERRORS.md row E11 — the negative counterpart: the quotient underflows below
/// `INT_MIN`, which `cvttsd2si` also maps to `INT_MIN`.
fn e11_bad_tiny_negative_underflows_int() {
    for x in [
        -1e-8f32,
        -1e-20,
        -1e-30,
        -f32::MIN_POSITIVE,
        -f32::from_bits(1),
        -4.0e-8,
    ] {
        expect(
            &format!("E11 bad({x:e}) underflows int"),
            INT_MIN_LINE,
            |api| unsafe { (api.bad)(x) },
        );
    }
}

/// ERRORS.md row E12 — one ULP either side of the `cvttsd2si` range boundary:
/// the C flips between a real value and `INT_MIN` and the Rust must flip at
/// exactly the same float.
fn e12_bad_cvttsd2si_boundary() {
    let l = libs();
    let mut saw_value = false;
    let mut saw_indefinite = false;
    for base in [
        100.0f64 / 2147483648.0,
        100.0f64 / 2147483647.0,
        -100.0f64 / 2147483648.0,
        -100.0f64 / 2147483647.0,
    ] {
        let mut x = base as f32;
        for _ in 0..6 {
            x = next_down(x);
        }
        for _ in 0..12 {
            let (c_out, ()) = capture(|| unsafe { (l.c.api.bad)(x) });
            let (r_out, ()) = capture(|| unsafe { (l.rs.api.bad)(x) });
            assert_eq!(
                c_out,
                r_out,
                "\nDIVERGENCE E12 bad(0x{:08x} = {:e})\n  C   : \"{}\"\n  Rust: \"{}\"\n",
                x.to_bits(),
                x,
                esc(&c_out),
                esc(&r_out)
            );
            if c_out == INT_MIN_LINE.as_bytes() {
                saw_indefinite = true;
            } else {
                saw_value = true;
            }
            x = next_up(x);
        }
    }
    assert!(
        saw_value && saw_indefinite,
        "E12 did not actually straddle the boundary (value={saw_value}, indefinite={saw_indefinite})"
    );

    // Pin the exact flip point observed in the C library: at 100/2^31 rounded to
    // f32 (0x33480000) the quotient is still just above INT_MAX so the C prints
    // the indefinite value, and one ULP up it becomes representable.
    let at = f32::from_bits(0x3348_0000);
    expect("E12 bad(0x33480000)", INT_MIN_LINE, |api| unsafe {
        (api.bad)(at)
    });
    expect("E12 bad(0x33480001)", "2147483484\n", |api| unsafe {
        (api.bad)(next_up(at))
    });
}

/// ERRORS.md row E13 — `bad(+INF)`: `100.0/INF` is `+0.0`, `(int)0.0` is `0`
/// (well defined -- must NOT be INT_MIN).
fn e13_bad_positive_infinity() {
    expect("E13 bad(+INF)", "0\n", |api| unsafe {
        (api.bad)(f32::INFINITY);
    });
}

/// ERRORS.md row E14 — `bad(-INF)`: `100.0/-INF` is `-0.0`, `(int)-0.0` is `0`.
fn e14_bad_negative_infinity() {
    expect("E14 bad(-INF)", "0\n", |api| unsafe {
        (api.bad)(f32::NEG_INFINITY);
    });
}

/// ERRORS.md row E15 — `good(+0.0)`: the `fabs(data) > 0.000001` guard rejects
/// the divisor and the diagnostic string is printed instead.
fn e15_good_positive_zero() {
    let expected = format!("{G2B}{REJECT}");
    expect("E15 good(+0.0)", &expected, |api| unsafe {
        (api.good)(0.0);
    });
}

/// ERRORS.md row E16 — `good(-0.0)`: `fabs` clears the sign, still rejected.
fn e16_good_negative_zero() {
    let expected = format!("{G2B}{REJECT}");
    expect("E16 good(-0.0)", &expected, |api| unsafe {
        (api.good)(-0.0);
    });
}

/// ERRORS.md row E17 — NaN: `comisd` is unordered so `jbe` is taken (C: any
/// comparison with NaN is false).  The one case where the guard rejects a value
/// that is not small.
fn e17_good_nan_unordered() {
    let expected = format!("{G2B}{REJECT}");
    for x in [
        f32::NAN,
        -f32::NAN,
        SNAN,
        NEG_SNAN,
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffff_ffff),
        f32::from_bits(0x7f80_0001),
    ] {
        expect(
            &format!("E17 good(NaN 0x{:08x})", x.to_bits()),
            &expected,
            |api| unsafe { (api.good)(x) },
        );
    }
}

/// ERRORS.md row E18 — subnormals / FLT_MIN / 1e-8 are all `<= 1e-6`, so unlike
/// `bad` the `good` path never divides by them.
fn e18_good_subnormals_rejected() {
    let expected = format!("{G2B}{REJECT}");
    for x in [
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
        -f32::from_bits(0x007f_ffff),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-8f32,
        -1e-8f32,
        1e-45f32,
    ] {
        expect(
            &format!("E18 good(0x{:08x} = {:e})", x.to_bits(), x),
            &expected,
            |api| unsafe { (api.good)(x) },
        );
    }
}

/// ERRORS.md row E19 — `1e-6f` is `9.99999997475e-7` as a double, i.e. strictly
/// below the `0.000001` literal, so `>` is false and the value is rejected.
fn e19_good_guard_boundary_below() {
    let expected = format!("{G2B}{REJECT}");
    for x in [1e-6f32, -1e-6f32, next_down(1e-6f32), -next_down(1e-6f32)] {
        expect(
            &format!("E19 good(0x{:08x} = {:e})", x.to_bits(), x),
            &expected,
            |api| unsafe { (api.good)(x) },
        );
    }
}

/// ERRORS.md row E20 — one ULP above: `nextafter(1e-6f, INF)` is strictly
/// greater than `0.000001`, so the guard accepts and the division happens.
fn e20_good_guard_boundary_above() {
    let x = next_up(1e-6f32);
    assert_eq!(x.to_bits(), 0x3586_37be, "test premise: nextafter(1e-6f, INF)");
    assert!(
        (x as f64) > 0.000001f64,
        "test premise broken: {x:e} is not above the guard"
    );
    // Exact bytes the C emits on the accept branch (verified against the C .so).
    expect(
        &format!("E20 good(0x{:08x})", x.to_bits()),
        &format!("{G2B}99999988\n"),
        |api| unsafe { (api.good)(x) },
    );
}

/// ERRORS.md row E21 — the negative side of the boundary: `fabs` accepts, the
/// quotient is negative.
fn e21_good_guard_boundary_negative() {
    let x = -next_up(1e-6f32);
    assert_eq!(x.to_bits(), 0xb586_37be, "test premise: -nextafter(1e-6f, INF)");
    expect(
        &format!("E21 good(0x{:08x})", x.to_bits()),
        &format!("{G2B}-99999988\n"),
        |api| unsafe { (api.good)(x) },
    );
}

/// ERRORS.md row E22 — ±INF passes the guard; `100.0/±INF` is `±0.0` so `0` is
/// printed.
fn e22_good_infinities() {
    let expected = format!("{G2B}0\n");
    expect("E22 good(+INF)", &expected, |api| unsafe {
        (api.good)(f32::INFINITY);
    });
    expect("E22 good(-INF)", &expected, |api| unsafe {
        (api.good)(f32::NEG_INFINITY);
    });
}

/// ERRORS.md row E23 — `driver` calls `bad` unguarded after `good`, so the whole
/// six-line transcript must match when `badData` is a rejected value.
fn e23_driver_bad_data_rejected() {
    for b in [
        0.0f32,
        -0.0,
        f32::NAN,
        SNAN,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-8,
        -1e-8,
    ] {
        let expected = format!(
            "Calling good()...\n{G2B}50\nFinished good()\nCalling bad()...\n{INT_MIN_LINE}Finished bad()\n"
        );
        expect(
            &format!("E23 driver(2.0, 0x{:08x})", b.to_bits()),
            &expected,
            |api| unsafe { (api.driver)(2.0, b) },
        );
    }
}

/// ERRORS.md row E24 — both guards fire in a single call.
fn e24_driver_both_rejected() {
    for g in [0.0f32, -0.0, f32::NAN, NEG_SNAN, f32::from_bits(1), 1e-8, 1e-6] {
        for b in [0.0f32, -0.0, f32::NAN, SNAN, f32::from_bits(1), 1e-8] {
            let expected = format!(
                "Calling good()...\n{G2B}{REJECT}Finished good()\nCalling bad()...\n{INT_MIN_LINE}Finished bad()\n"
            );
            expect(
                &format!(
                    "E24 driver(0x{:08x}, 0x{:08x})",
                    g.to_bits(),
                    b.to_bits()
                ),
                &expected,
                |api| unsafe { (api.driver)(g, b) },
            );
        }
    }
}

/// ERRORS.md row E25 — arbitrary 32-bit patterns reinterpreted as `float`
/// (the FFI analogue of an out-of-range enum: any bit pattern is a real input
/// the C accepts), and arbitrary `int` patterns for `printIntLine`.
fn e25_arbitrary_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0xE25);

    let floats: Vec<F> = (0..8192).map(|_| F(rng.f32_bits())).collect();
    compare_batch("E25 bad(arbitrary bits)", &floats, |api, x| unsafe {
        (api.bad)(x.0)
    });
    compare_batch("E25 good(arbitrary bits)", &floats, |api, x| unsafe {
        (api.good)(x.0)
    });

    let pairs: Vec<FF> = (0..4096)
        .map(|_| FF(rng.f32_bits(), rng.f32_bits()))
        .collect();
    compare_batch("E25 driver(arbitrary bits)", &pairs, |api, x| unsafe {
        (api.driver)(x.0, x.1)
    });

    let ints: Vec<c_int> = (0..8192).map(|_| rng.next_u32() as i32).collect();
    compare_batch("E25 printIntLine(arbitrary bits)", &ints, |api, x| unsafe {
        (api.print_int_line)(*x)
    });
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks (not tied to a single ERRORS.md row)
// ---------------------------------------------------------------------------

/// NULL passed repeatedly and mixed with valid pointers, to make sure the null
/// rejection has no side effect on subsequent calls.
fn g01_null_pointer_repeated_and_mixed() {
    let good = b"kept\0";
    expect("G1 NULL mixed with valid pointers", "kept\nkept\n", |api| unsafe {
        (api.print_line)(std::ptr::null());
        (api.print_line)(good.as_ptr() as *const c_char);
        (api.print_line)(std::ptr::null());
        (api.print_line)(std::ptr::null());
        (api.print_line)(good.as_ptr() as *const c_char);
        (api.print_line)(std::ptr::null());
    });

    // ...and through the composed entry point, interleaved with driver().
    let l = libs();
    let f = |api: &Api| unsafe {
        (api.print_line)(std::ptr::null());
        (api.driver)(0.0, 0.0);
        (api.print_line)(std::ptr::null());
    };
    let (c_out, ()) = capture(|| f(&l.c.api));
    let (r_out, ()) = capture(|| f(&l.rs.api));
    assert_eq!(c_out, r_out, "G1 divergence: C \"{}\" vs Rust \"{}\"", esc(&c_out), esc(&r_out));
}

/// Every `int` value one step past a range edge: `INT_MIN`/`INT_MAX`, and the
/// digit-count rollovers of `"%d"`.
fn g02_int_one_past_every_range() {
    let mut v: Vec<c_int> = vec![i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1];
    for p in 0..10u32 {
        let ten = 10i32.pow(p);
        v.extend_from_slice(&[ten - 1, ten, ten + 1, -(ten - 1), -ten]);
        if ten != 1 {
            v.push(-(ten + 1));
        }
    }
    for s in 0..31u32 {
        let two = 1i32 << s;
        v.extend_from_slice(&[two - 1, two, -two]);
        if s < 31 {
            v.push(two.wrapping_add(1));
        }
    }
    compare_print_int_line("G2 int range/digit boundaries", &v);
}

/// One ULP either side of every float bound the C source mentions or implies:
/// the `0.000001` guard, the `cvttsd2si` range, zero, `FLT_MIN`, `FLT_MAX`, and
/// the normal/subnormal transition.
fn g03_float_one_ulp_past_every_documented_bound() {
    let bounds = [
        0.0f32,
        -0.0f32,
        1e-6f32,
        -1e-6f32,
        (100.0f64 / 2147483648.0) as f32,
        -(100.0f64 / 2147483648.0) as f32,
        (100.0f64 / 2147483647.0) as f32,
        -(100.0f64 / 2147483647.0) as f32,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::MAX,
        f32::MIN,
        2.0f32,
        -2.0f32,
        1.0f32,
        100.0f32,
    ];
    let mut v = Vec::new();
    for b in bounds {
        v.push(b);
        let mut up = b;
        let mut down = b;
        for _ in 0..3 {
            up = next_up(up);
            down = next_down(down);
            v.push(up);
            v.push(down);
        }
    }
    compare_bad("G3 bad() one ULP past every bound", &v);
    compare_good("G3 good() one ULP past every bound", &v);
    let pairs: Vec<FF> = v.iter().map(|x| FF(*x, *x)).collect();
    compare_driver("G3 driver() one ULP past every bound", &pairs);
}

/// The API declares no `enum` parameters, so the equivalent "value with no valid
/// variant" is an arbitrary bit pattern in the scalar arguments.  This checks
/// every *class* of `f32` encoding exhaustively over the exponent field, plus
/// the four canonical "impossible" encodings.
fn g04_out_of_range_discriminants() {
    let mut v = Vec::new();
    // Sweep the whole exponent range with three different mantissas and both
    // signs: 256 * 3 * 2 = 1536 values covering every encoding class.
    for exp in 0u32..=255 {
        for mant in [0u32, 1, 0x0055_5555, 0x007f_ffff] {
            for sign in [0u32, 1] {
                v.push(f32::from_bits((sign << 31) | (exp << 23) | mant));
            }
        }
    }
    compare_bad("G4 exhaustive f32 encoding classes -> bad", &v);
    compare_good("G4 exhaustive f32 encoding classes -> good", &v);
}
