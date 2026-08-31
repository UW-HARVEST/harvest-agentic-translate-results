//! `driver`'s output does not depend on its `double` argument alone: glibc's
//! `%a` and `%f` conversions also read the process's `LC_NUMERIC` decimal point
//! and its current floating-point rounding direction. A translation that
//! reimplements the conversions in Rust silently diverges here even though it
//! agrees on every possible input under the default ambient state, so these
//! cases are checked explicitly.

mod common;

use common::*;

/// Values with a fractional part, so the decimal point is actually printed, and
/// with a fifth decimal digit near a rounding boundary.
fn probes() -> Vec<f64> {
    vec![
        1.5,
        -1.5,
        1234.0625,
        0.1,
        -0.1,
        1.00005,
        2.00005,
        -1.00005,
        0.000_05,
        std::f64::consts::PI,
        1.0 / 3.0,
        0.03125,
        0.09375,
        1e-7,
        -1e-7,
        1e17 + 0.5,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NAN,
    ]
}

/// Locales whose `LC_NUMERIC` uses a comma as the decimal separator. The first
/// one that is actually installed is used.
const COMMA_LOCALES: &[&str] = &[
    "de_DE.utf8",
    "de_DE.UTF-8",
    "de_DE",
    "fr_FR.utf8",
    "fr_FR.UTF-8",
    "fr_FR",
    "ru_RU.utf8",
    "es_ES.utf8",
    "it_IT.utf8",
    "nl_NL.utf8",
    "pt_BR.utf8",
];

#[test]
fn matches_under_a_comma_decimal_locale() {
    let inputs = probes();
    let mut tried = Vec::new();
    for loc in COMMA_LOCALES {
        match compare_with_env(&inputs, &[("DRIVER_LOCALE", loc)]) {
            Ok(()) => return, // matched under a real comma-decimal locale
            Err(e) if is_unsupported(&e) => {
                tried.push(*loc);
                continue;
            }
            Err(e) => panic!("{e}"),
        }
    }
    eprintln!(
        "note: skipping locale check, none of these locales are installed: {tried:?}\n\
         install one (e.g. `localedef`/`locale-gen de_DE.UTF-8`) to exercise it"
    );
}

/// The `C`/`POSIX` locale must of course also agree; this is the same as the
/// default configuration, stated explicitly.
#[test]
fn matches_under_the_c_locale() {
    let inputs = probes();
    for loc in ["C", "POSIX", "C.UTF-8"] {
        match compare_with_env(&inputs, &[("DRIVER_LOCALE", loc)]) {
            Ok(()) => {}
            Err(e) if is_unsupported(&e) => continue,
            Err(e) => panic!("{e}"),
        }
    }
}

/// Every IEEE-754 rounding direction. `%.4f` has to round the exact decimal
/// expansion the same way glibc does under each of them.
#[test]
fn matches_under_every_rounding_direction() {
    let mut inputs = probes();
    // Add a dense band of values whose fifth decimal digit decides the result.
    let mut rng = SplitMix64(0x5eed_1234_9876_0001);
    for _ in 0..5_000 {
        let r = rng.next_u64();
        let exp = (1023i64 - 20 + ((r >> 52) % 60) as i64) as u64;
        let sign = (r >> 51) & 1;
        inputs.push(f64::from_bits(
            (sign << 63) | (exp << 52) | (r & 0x000f_ffff_ffff_ffff),
        ));
    }
    for k in 0..2_000i64 {
        let v = (k as f64) * 1e-4 + 0.5e-4;
        inputs.push(v);
        inputs.push(-v);
    }

    let mut exercised = 0;
    for mode in ["tonearest", "downward", "upward", "towardzero"] {
        match compare_with_env(&inputs, &[("DRIVER_ROUNDING", mode)]) {
            Ok(()) => exercised += 1,
            Err(e) if is_unsupported(&e) => {
                eprintln!("note: rounding mode {mode} unavailable on this target: {e}")
            }
            Err(e) => panic!("{e}"),
        }
    }
    assert!(
        exercised > 0,
        "no rounding direction could be exercised on this target"
    );
}

/// Locale and rounding direction combined, since they affect different parts of
/// the same conversion.
#[test]
fn matches_under_locale_and_rounding_combined() {
    let inputs = probes();
    for loc in COMMA_LOCALES {
        for mode in ["downward", "upward", "towardzero", "tonearest"] {
            match compare_with_env(
                &inputs,
                &[("DRIVER_LOCALE", loc), ("DRIVER_ROUNDING", mode)],
            ) {
                Ok(()) => {}
                Err(e) if is_unsupported(&e) => break,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
