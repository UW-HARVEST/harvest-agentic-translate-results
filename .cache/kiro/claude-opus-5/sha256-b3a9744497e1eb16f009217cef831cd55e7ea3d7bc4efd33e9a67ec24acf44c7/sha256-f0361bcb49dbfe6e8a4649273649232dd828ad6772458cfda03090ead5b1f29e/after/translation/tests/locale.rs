//! Extra configuration axis: the process `LC_NUMERIC` locale.
//!
//! The C hard-codes `decimal_point = '.'`, so its "localise the decimal
//! separator" loop is a no-op — but it then hands the string to `strtod(3)`,
//! whose notion of the radix character comes from the *process* locale. Both
//! `.so`s call the same libc `strtod` in the same process, so switching the
//! locale must move both implementations identically.
//!
//! This lives in its own test binary because `setlocale` mutates process-global
//! state and would otherwise race the parallel tests in the other files.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

const LC_ALL: c_int = 6;
const LC_NUMERIC: c_int = 1;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

fn try_setlocale(cat: c_int, name: &str) -> bool {
    let c = std::ffi::CString::new(name).unwrap();
    // Safety: standard libc call with a NUL-terminated string.
    !unsafe { setlocale(cat, c.as_ptr()) }.is_null()
}

fn sweep_under_current_locale(tag: &str) {
    // Deterministic shapes that straddle the radix character.
    for s in [
        "1.5", "0.5", "-2.25", ".5", "5.", "1.2.3", "1,5", "0,5", "-2,25", ",5", "5,", "1.5e3",
        "1,5e3", "12", "-12", "1e5", "2147483647", "-2147483648", "1e999", "-1e999", "1e-999",
        "-0", "-0.0", "-0,0", "+.5", "+,5",
    ] {
        diff(&format!("locale[{tag}]/fixed"), s.as_bytes());
    }

    // Exhaustive length-1..=4 sweep over an alphabet that contains BOTH radix
    // candidates, so any locale-dependent divergence in where `strtod` stops
    // shows up as an `offset` mismatch.
    const ALPHA: &[u8] = b"012+-eE.,}";
    let base = ALPHA.len();
    for len in 1..=4usize {
        let mut s = vec![0u8; len];
        for n in 0..base.pow(len as u32) {
            let mut k = n;
            for slot in s.iter_mut() {
                *slot = ALPHA[k % base];
                k /= base;
            }
            diff(&format!("locale[{tag}]/sweep{len}"), &s);
        }
    }

    // Randomized fractional values.
    let mut rng = Rng::new(SEED ^ 0x10C4);
    for _ in 0..(N * 2) {
        let mut v: Vec<u8> = Vec::new();
        if rng.bool() {
            v.push(b'-');
        }
        v.extend_from_slice(&rng.digits_range(1, 10));
        v.push(if rng.bool() { b'.' } else { b',' });
        v.extend_from_slice(&rng.digits_range(1, 10));
        diff(&format!("locale[{tag}]/rand"), &v);
    }
}

#[test]
fn locale_axis_c_de_and_fr() {
    // Baseline: the default "C" locale the other test binaries run in.
    assert!(try_setlocale(LC_ALL, "C"), "setlocale(LC_ALL, \"C\") must work");
    sweep_under_current_locale("C");

    // A comma-radix locale, if the system has one. In such a locale libc
    // `strtod` stops at '.' instead of consuming it, which changes both the
    // parsed value AND the resulting `offset` — a strong differential signal.
    let mut switched = 0;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "de_DE", "fr_FR.utf8", "fr_FR"] {
        if try_setlocale(LC_NUMERIC, name) {
            eprintln!("[locale] LC_NUMERIC = {name}");
            sweep_under_current_locale(name);
            switched += 1;
            break;
        }
    }
    if switched == 0 {
        eprintln!("[locale] no comma-radix locale installed; only the C locale was exercised");
    }

    // Back to C, and confirm the baseline still holds (no sticky state).
    assert!(try_setlocale(LC_ALL, "C"));
    sweep_under_current_locale("C/again");
}
