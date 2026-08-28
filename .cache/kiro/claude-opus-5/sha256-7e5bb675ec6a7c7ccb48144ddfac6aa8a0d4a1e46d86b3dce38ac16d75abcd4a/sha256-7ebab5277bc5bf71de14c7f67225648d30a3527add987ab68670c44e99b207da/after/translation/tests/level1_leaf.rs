//! Level 1: leaf functions with no internal dependencies.

mod common;

use common::{assert_f64_bits_eq, both};

#[test]
fn convert_double_to_int_matches() {
    let (c, rust) = both();

    let mut cases: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        42.0,
        -42.0,
        f64::from(i32::MAX),
        f64::from(i32::MIN),
        2147483646.5,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        -2147483647.5,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        1e9,
        -1e9,
        1e18,
        -1e18,
        1e300,
        -1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        -f64::EPSILON,
        2.0f64.powi(31),
        -(2.0f64.powi(31)),
        2.0f64.powi(40),
        -(2.0f64.powi(40)),
        2.0f64.powi(63),
        5e-324,
        -5e-324,
    ];

    // A deterministic spread of magnitudes and fractions.
    let mut state = 0x1234_5678_9abc_def0u64;
    for _ in 0..4000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let sign = if state & 1 == 0 { 1.0 } else { -1.0 };
        let mag = ((state >> 11) % 5_000_000_000u64) as f64 / 1000.0;
        cases.push(sign * mag);
    }

    for &v in &cases {
        let a = c.convert_double_to_int(v);
        let b = rust.convert_double_to_int(v);
        assert_eq!(
            a, b,
            "convert_double_to_int({v:?}) (bits {:#018x}): C={a}, Rust={b}",
            v.to_bits()
        );
    }
}

#[test]
fn process_negation_matches() {
    let (c, rust) = both();

    let mut cases: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        255,
        256,
        -255,
        -256,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7fff,
        -0x8000,
        0x10000,
        -0x10000,
    ];
    let mut state = 0xdead_beef_cafe_1234u64;
    for _ in 0..2000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        cases.push((state >> 21) as i32);
    }

    for &v in &cases {
        let a = c.process_negation(v);
        let b = rust.process_negation(v);
        assert_eq!(a, b, "process_negation({v}): C={a}, Rust={b}");
    }
}

#[test]
fn calculate_with_doubles_matches() {
    let (c, rust) = both();

    let interesting: [i32; 27] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        9,
        -9,
        10,
        -10,
        11,
        -11,
        19,
        -19,
        100,
        -100,
        255,
        256,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];

    for &a in &interesting {
        for &b in &interesting {
            for &cc in &interesting {
                let x = c.calculate_with_doubles(a, b, cc);
                let y = rust.calculate_with_doubles(a, b, cc);
                assert_f64_bits_eq(x, y, &format!("calculate_with_doubles({a}, {b}, {cc})"));
            }
        }
    }

    let mut state = 0x0bad_c0de_1337_4242u64;
    for _ in 0..20000 {
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 19) as i32
        };
        let (a, b, cc) = (next(), next(), next());
        let x = c.calculate_with_doubles(a, b, cc);
        let y = rust.calculate_with_doubles(a, b, cc);
        assert_f64_bits_eq(x, y, &format!("calculate_with_doubles({a}, {b}, {cc})"));
    }
}
