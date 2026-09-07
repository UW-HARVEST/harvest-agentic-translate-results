// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md. Both libraries are driven through their
// `.so` exports only.

mod common;

use common::*;
use std::ffi::c_int;

const IMAX: c_int = i32::MAX;
const IMIN: c_int = i32::MIN;

fn h(floors: c_int, bedrooms: c_int, bathrooms: f64) -> HouseT {
    HouseT {
        floors,
        bedrooms,
        bathrooms,
    }
}

// --- C1 ---------------------------------------------------------------------
#[test]
fn c1_run_baseline() {
    diff_run(h(2, 5, 2.5), 3, "C1");
    // Sanity: the output really is the 4 lines print_house emits.
    let l = libs();
    let mut hh = h(2, 5, 2.5);
    let out = {
        let f = l.c.run();
        capture_stdout(|| unsafe { f(&mut hh, 3) })
    };
    assert_eq!(show(&out).lines().count(), 4, "C1: expected 4 lines");
}

// --- C2 ---------------------------------------------------------------------
#[test]
fn c2_run_random_small() {
    let mut rng = Rng::new(0xC002);
    for i in 0..200 {
        let house = h(
            rng.range_i32(-50, 50),
            rng.range_i32(-50, 50),
            rng.range_i32(-40, 40) as f64 * 0.5,
        );
        diff_run(house, rng.range_i32(-20, 20), &format!("C2#{i}"));
    }
}

// --- C3 ---------------------------------------------------------------------
#[test]
fn c3_run_random_full_range() {
    let mut rng = Rng::new(0xC003);
    for i in 0..500 {
        let house = h(rng.next_i32(), rng.next_i32(), rng.finite_f64());
        diff_run(house, rng.next_i32(), &format!("C3#{i}"));
    }
}

// --- C4 ---------------------------------------------------------------------
#[test]
fn c4_run_rounding_ties() {
    let ties = [
        0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95, 1.05, 1.15, 2.25, 2.75, -0.05,
        -0.15, -0.25, -0.35, -1.05, -2.25, -2.75, 1.0 / 3.0, 2.0 / 3.0, 0.049999999999999996,
        0.050000000000000003,
    ];
    for (i, &b) in ties.iter().enumerate() {
        diff_run(h(1, 1, b), 1, &format!("C4#{i} bathrooms={b:?}"));
        diff_run(h(0, 0, b), 0, &format!("C4#{i}b"));
    }
}

// --- C5 ---------------------------------------------------------------------
#[test]
fn c5_run_signed_zero() {
    diff_run(h(2, 5, -0.0), 1, "C5 -0.0");
    diff_run(h(2, 5, 0.0), 1, "C5 +0.0");
    diff_run(h(2, 5, -1.0), 1, "C5 -1.0 -> -0.0 after += 1.0");
    diff_run(h(2, 5, -0.4), 1, "C5 -0.4 prints -0.4");
    diff_run(h(2, 5, -0.04), 1, "C5 -0.04 prints -0.0");
}

// --- C6 ---------------------------------------------------------------------
#[test]
fn c6_run_nan_inf() {
    let vals = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_0000_0000_0001), // another NaN payload
        f64::from_bits(0xfff0_0000_0000_0001), // signaling-ish NaN
    ];
    for (i, &b) in vals.iter().enumerate() {
        diff_run(h(2, 5, b), 3, &format!("C6#{i} bits={:#018x}", b.to_bits()));
    }
}

// --- C7 ---------------------------------------------------------------------
#[test]
fn c7_run_extreme_finite() {
    let vals = [
        f64::MAX,
        -f64::MAX,
        1e308,
        -1e308,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324, // smallest subnormal
        -5e-324,
        1e-1,
        1e16,
        1e17,
        9007199254740993.0, // 2^53 + 1 region
        -9007199254740993.0,
    ];
    for (i, &b) in vals.iter().enumerate() {
        diff_run(h(2, 5, b), 3, &format!("C7#{i} b={b:e}"));
    }
}

// --- C8 ---------------------------------------------------------------------
#[test]
fn c8_run_floors_overflow() {
    for (i, &f) in [IMAX, IMAX - 1, IMIN, IMIN + 1, 0, -1, 1].iter().enumerate() {
        diff_run(h(f, 5, 2.5), 1, &format!("C8#{i} floors={f}"));
    }
}

// --- C9 ---------------------------------------------------------------------
#[test]
fn c9_run_bedrooms_positive_overflow() {
    for (i, &e) in [1, 2, 7, IMAX, IMAX - 1].iter().enumerate() {
        diff_run(h(2, IMAX, 2.5), e, &format!("C9#{i} extra={e}"));
        diff_run(h(2, IMAX - 3, 2.5), e, &format!("C9#{i}b extra={e}"));
    }
}

// --- C10 --------------------------------------------------------------------
#[test]
fn c10_run_bedrooms_negative_overflow() {
    for (i, &e) in [-1, -2, -7, IMIN, IMIN + 1].iter().enumerate() {
        diff_run(h(2, IMIN, 2.5), e, &format!("C10#{i} extra={e}"));
        diff_run(h(2, IMIN + 3, 2.5), e, &format!("C10#{i}b extra={e}"));
    }
}

// --- C11 / C12 / C13 --------------------------------------------------------
#[test]
fn c11_c12_c13_extra_bedrooms_extremes() {
    let bedrooms = [0, 1, -1, IMIN, IMAX, IMAX - 1, IMIN + 1];
    for &e in &[IMAX, IMIN, 0] {
        for &b in &bedrooms {
            diff_run(h(2, b, 2.5), e, &format!("C11-13 bedrooms={b} extra={e}"));
        }
    }
}

// --- C14 --------------------------------------------------------------------
#[test]
fn c14_run_twice_state_accumulation() {
    let mut rng = Rng::new(0xC014);
    diff_run_twice(h(2, 5, 2.5), 3, "C14 baseline");
    diff_run_twice(h(IMAX, IMAX, 2.5), IMAX, "C14 overflow both");
    diff_run_twice(h(IMIN, IMIN, -0.0), IMIN, "C14 underflow both");
    for i in 0..100 {
        let house = h(rng.next_i32(), rng.next_i32(), rng.finite_f64());
        diff_run_twice(house, rng.next_i32(), &format!("C14#{i}"));
    }
}

// --- C15 --------------------------------------------------------------------
#[test]
fn c15_driver_plain_decimal() {
    for d in 0..10u32 {
        diff_driver(d.to_string().as_bytes(), "C15 single digit");
    }
    for s in ["7", "12345", "42", "999999", "1000000", "2147483646"] {
        diff_driver(s.as_bytes(), "C15");
    }
}

// --- C16 --------------------------------------------------------------------
#[test]
fn c16_driver_explicit_sign() {
    for s in ["+7", "-7", "+0", "-0", "+2147483647", "-2147483648", "+42", "-1"] {
        diff_driver(s.as_bytes(), "C16");
    }
}

// --- C17 --------------------------------------------------------------------
#[test]
fn c17_driver_leading_whitespace() {
    for s in [
        " 42",
        "\t42",
        "\n42",
        "\x0b42",
        "\x0c42",
        "\r42",
        "\n\x0b\x0c\r 42",
        "   -42",
        "  +42",
        "\t\t\t0",
    ] {
        diff_driver(s.as_bytes(), "C17");
    }
}

// --- C18 --------------------------------------------------------------------
#[test]
fn c18_driver_leading_zeros() {
    for s in [
        "007",
        "0000000000000000042",
        "-000042",
        "+000042",
        "00000000000000000000000000000000000000000",
        "0000000002147483647",
    ] {
        diff_driver(s.as_bytes(), "C18");
    }
}

// --- C19 --------------------------------------------------------------------
#[test]
fn c19_driver_trailing_garbage() {
    for s in [
        "42abc", "42 43", "42.", "42.9", "7-", "1e5", "3+4", "12,345", "5)", "-8xyz", "0 ",
        "2147483647junk", "1\n",
    ] {
        diff_driver(s.as_bytes(), "C19");
    }
}

// --- C20 --------------------------------------------------------------------
#[test]
fn c20_driver_base10_stops_at_x() {
    for s in ["0x10", "0X1f", "0b101", "0o17", "0xdeadbeef", "-0x10"] {
        diff_driver(s.as_bytes(), "C20");
    }
}

// --- C21 --------------------------------------------------------------------
#[test]
fn c21_driver_accepted_boundaries() {
    for s in [
        "2147483647",
        "-2147483648",
        "2147483646",
        "-2147483647",
        "0",
        "1073741824",
        "-1073741824",
    ] {
        diff_driver(s.as_bytes(), "C21");
    }
}

// --- C22 --------------------------------------------------------------------
#[test]
fn c22_driver_randomized_i32() {
    let mut rng = Rng::new(0xC022);
    for i in 0..500 {
        let v = rng.next_i32();
        let renderings: Vec<String> = vec![
            v.to_string(),
            if v >= 0 {
                format!("+{v}")
            } else {
                v.to_string()
            },
            format!("   {v}"),
            if v >= 0 {
                format!("00{v}")
            } else {
                format!("-00{}", v.unsigned_abs())
            },
            format!("{v}trailing"),
        ];
        for r in renderings {
            diff_driver(r.as_bytes(), &format!("C22#{i} v={v}"));
        }
    }
}

// --- C23 --------------------------------------------------------------------
#[test]
fn c23_driver_clears_preexisting_errno() {
    // C sets `errno = 0` before strtol, so a stale errno must NOT cause a
    // rejection. Set a hostile errno before each call.
    for &e in &[34 /*ERANGE*/, 22 /*EINVAL*/, 2 /*ENOENT*/, -1, i32::MAX] {
        for s in ["42", "-7", "0", "2147483647"] {
            let l = libs();
            let cs = std::ffi::CString::new(s).unwrap();
            let c_out = {
                let f = l.c.driver();
                capture_stdout(|| {
                    set_errno(e);
                    unsafe { f(cs.as_ptr()) }
                })
            };
            let r_out = {
                let f = l.rust.driver();
                capture_stdout(|| {
                    set_errno(e);
                    unsafe { f(cs.as_ptr()) }
                })
            };
            assert_eq!(
                show(&c_out),
                show(&r_out),
                "C23 mismatch errno={e} input={s:?}"
            );
            assert!(
                !show(&c_out).contains("An error occurred"),
                "C23: C must still accept {s:?} with stale errno {e}"
            );
        }
    }
    set_errno(0);
}

// --- C24 --------------------------------------------------------------------
#[test]
fn c24_driver_long_but_valid() {
    let mut s = String::from("+");
    s.push_str(&"0".repeat(300));
    s.push_str("2147483647");
    diff_driver(s.as_bytes(), "C24");

    let mut s2 = String::from("-");
    s2.push_str(&"0".repeat(4000));
    s2.push_str("2147483648");
    diff_driver(s2.as_bytes(), "C24 long INT_MIN");

    let mut s3 = " ".repeat(1000);
    s3.push_str("123");
    diff_driver(s3.as_bytes(), "C24 long whitespace");
}
