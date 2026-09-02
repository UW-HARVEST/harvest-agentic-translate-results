//! Locale-state differential tests (`CONFIGS.md` rows 27-31).
//!
//! `driver` calls `setlocale(LC_ALL, "C")` before every classification, and the
//! `isXXX` macros then read `(*__ctype_b_loc())[c]` — the *live* table. Two
//! consequences drive these tests:
//!
//!  * `setlocale` sets the **global** locale, so a caller's global locale must
//!    not affect the result — but proving that requires re-establishing the
//!    caller's locale before *each* side of the comparison, because the C call
//!    (which runs first) already resets it. `diff_*_prepared` does that; a plain
//!    `diff_*` would silently let the Rust side run under `"C"` and would not
//!    notice a Rust that omitted `setlocale` altogether.
//!  * `setlocale` cannot displace a **thread** locale installed with
//!    `uselocale()`, so in that state the live table is *not* the `"C"` one and
//!    the C classifies accordingly. A translation with frozen `"C"` tables
//!    diverges here.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Bytes that differ between "C" and an 8-bit Latin-1 locale: high-bit letters.
const HIGH_BYTES: [u8; 8] = [0x80, 0xA0, 0xB5, 0xC0, 0xC9, 0xDF, 0xE9, 0xFF];
const MIXED_BYTES: [u8; 8] = [b'a', b'Z', b'0', b' ', b'\t', 0x7F, 0x00, b'I'];

const UTF8_LOCALES: [&str; 3] = ["C.utf8", "en_US.utf8", "de_DE.utf8"];
const LATIN1_LOCALES: [&str; 3] = ["en_US.iso88591", "de_DE.iso88591", "fr_FR.iso88591"];

// ---------------------------------------------------------------------------
// Rows 27-28: the caller's GLOBAL locale, re-established before each side.
// ---------------------------------------------------------------------------

#[test]
fn cfg_27_global_locale_utf8() {
    for name in UTF8_LOCALES {
        if !set_global_locale(name) {
            eprintln!("skip: global locale {name} unavailable");
            continue;
        }
        let prep = || {
            assert!(set_global_locale(name));
        };
        for b in HIGH_BYTES.iter().chain(MIXED_BYTES.iter()) {
            diff_char_prepared(
                *b as c_char,
                &format!("row 27: global {name}, byte {b:#04x}"),
                &prep,
            );
        }
        reset_global_locale();
    }
    reset_global_locale();
}

#[test]
fn cfg_28_global_locale_latin1_all_256() {
    for name in LATIN1_LOCALES {
        if !set_global_locale(name) {
            eprintln!("skip: global locale {name} unavailable");
            continue;
        }
        let prep = || {
            assert!(set_global_locale(name));
        };
        diff_all_chars_prepared(&format!("row 28: global {name}, all 256"), &prep);
        reset_global_locale();
    }
    reset_global_locale();
}

/// The direct check of `driver`'s only side effect: after the call, the global
/// locale must be `"C"` for both implementations, whatever it was before.
///
/// This is what catches a Rust that simply omits `setlocale(LC_ALL, "C")` —
/// output comparison alone can miss it, because the C's own `setlocale` has
/// already normalised the state by the time the Rust side runs.
#[test]
fn cfg_27b_setlocale_side_effect_is_observable() {
    let cd = c_driver();
    let rd = rust_driver();
    for name in UTF8_LOCALES.iter().chain(LATIN1_LOCALES.iter()) {
        if !set_global_locale(name) {
            eprintln!("skip: global locale {name} unavailable");
            continue;
        }
        let before = query_global_locale(LC_ALL);
        let _ = capture(|| unsafe { cd(b'A' as c_char) });
        let after_c = query_global_locale(LC_ALL);
        let after_c_ctype = query_global_locale(LC_CTYPE);

        assert!(set_global_locale(name), "re-establish {name}");
        assert_eq!(query_global_locale(LC_ALL), before, "setup is reproducible");
        let _ = capture(|| unsafe { rd(b'A' as c_char) });
        let after_r = query_global_locale(LC_ALL);
        let after_r_ctype = query_global_locale(LC_CTYPE);

        assert_eq!(
            after_c, after_r,
            "row 27b: LC_ALL after the call differs (from {name}): \
             C left {after_c:?}, Rust left {after_r:?}"
        );
        assert_eq!(
            after_c_ctype, after_r_ctype,
            "row 27b: LC_CTYPE after the call differs (from {name})"
        );
        // Ground truth, read off the C: it really is "C" afterwards.
        assert_eq!(after_c, "C", "row 27b: C leaves LC_ALL == \"C\"");
        reset_global_locale();
    }
    reset_global_locale();
}

// ---------------------------------------------------------------------------
// Rows 29-30: a THREAD locale, which `setlocale` cannot displace.
// ---------------------------------------------------------------------------

#[test]
fn cfg_29_thread_locale_latin1_all_256() {
    for name in LATIN1_LOCALES {
        let Some(h) = push_thread_locale(name) else {
            eprintln!("skip: thread locale {name} unavailable");
            continue;
        };
        diff_all_chars(&format!("row 29: thread locale {name}, all 256"));
        for b in HIGH_BYTES {
            diff_char(b as c_char, &format!("row 29: thread {name}, {b:#04x}"));
        }
        pop_thread_locale(h);
    }
    reset_global_locale();
}

#[test]
fn cfg_30_thread_locale_utf8_all_256() {
    for name in ["C.utf8", "en_US.utf8", "tr_TR.utf8"] {
        let Some(h) = push_thread_locale(name) else {
            eprintln!("skip: thread locale {name} unavailable");
            continue;
        };
        diff_all_chars(&format!("row 30: thread locale {name}, all 256"));
        pop_thread_locale(h);
    }
    reset_global_locale();
}

/// Pin the Turkish case-mapping ground truth explicitly: under `tr_TR`, glibc's
/// `tolower('I')` is `'I'`, because dotless `ı` is not a single byte. An
/// ASCII-only or frozen-"C"-table translation would answer `'i'`.
#[test]
fn cfg_30b_turkish_dotless_i_ground_truth() {
    let Some(h) = push_thread_locale("tr_TR.utf8") else {
        eprintln!("skip: tr_TR.utf8 unavailable");
        return;
    };
    let cd = c_driver();
    let out = capture(|| unsafe { cd(b'I' as c_char) });
    let prefix = b"to lower: ";
    let at = out
        .windows(prefix.len())
        .position(|w| w == prefix)
        .expect("to lower line");
    let got = out[at + prefix.len()];
    assert_eq!(
        got, b'I',
        "row 30b: under tr_TR the C's tolower('I') must stay 'I', got {:?}",
        got as char
    );
    diff_char(b'I' as c_char, "row 30b: tr_TR 'I'");
    diff_char(b'i' as c_char, "row 30b: tr_TR 'i'");
    pop_thread_locale(h);
    reset_global_locale();
}

// ---------------------------------------------------------------------------
// Row 31: a thread locale AND a different global locale at the same time.
// ---------------------------------------------------------------------------

#[test]
fn cfg_31_thread_and_global_locale_differ() {
    for (thread_loc, global_loc) in [
        ("de_DE.iso88591", "en_US.utf8"),
        ("tr_TR.utf8", "de_DE.iso88591"),
        ("fr_FR.iso88591", "C.utf8"),
    ] {
        let Some(h) = push_thread_locale(thread_loc) else {
            eprintln!("skip: thread locale {thread_loc} unavailable");
            continue;
        };
        if !set_global_locale(global_loc) {
            eprintln!("skip: global locale {global_loc} unavailable");
            pop_thread_locale(h);
            continue;
        }
        let prep = || {
            assert!(set_global_locale(global_loc));
        };
        diff_all_chars_prepared(
            &format!("row 31: thread={thread_loc}, global={global_loc}"),
            &prep,
        );
        pop_thread_locale(h);
        reset_global_locale();
    }
    reset_global_locale();
}

/// Row 31 driven through the `int` view, so narrowing is exercised while a
/// non-`"C"` table is live.
#[test]
fn cfg_31b_thread_locale_with_wide_int_args() {
    for thread_loc in ["de_DE.iso88591", "tr_TR.utf8"] {
        let Some(h) = push_thread_locale(thread_loc) else {
            eprintln!("skip: thread locale {thread_loc} unavailable");
            continue;
        };
        let prep = || {
            let _ = set_global_locale("de_DE.iso88591");
        };
        let mut rng = Rng::new(SEED ^ 0x31B);
        for v in [c_int::MIN, c_int::MAX, 0xDEAD_BE41u32 as c_int, 128, 256, -1] {
            diff_int_prepared(
                v,
                &format!("row 31b: thread={thread_loc}, int {v:#010x}"),
                &prep,
            );
        }
        for i in 0..96 {
            let v = rng.next_u32() as c_int;
            diff_int_prepared(
                v,
                &format!("row 31b: thread={thread_loc}, random [{i}]"),
                &prep,
            );
        }
        pop_thread_locale(h);
        reset_global_locale();
    }
    reset_global_locale();
}

/// Locale switched between consecutive calls in a seeded random order, so no
/// implementation can cache the table across calls (the C re-reads
/// `__ctype_b_loc()` for every one of the twelve classifiers).
#[test]
fn cfg_31c_locale_churn_between_calls() {
    let names = [
        "de_DE.iso88591",
        "tr_TR.utf8",
        "C",
        "C.utf8",
        "fr_FR.iso88591",
    ];
    let mut rng = Rng::new(SEED ^ 0x31C);
    for i in 0..96 {
        let name = names[(rng.next_u64() % names.len() as u64) as usize];
        let Some(handle) = push_thread_locale(name) else {
            continue;
        };
        let global = names[(rng.next_u64() % names.len() as u64) as usize];
        let prep = || {
            let _ = set_global_locale(global);
        };
        let v = rng.next_u8();
        diff_char_prepared(
            v as c_char,
            &format!("row 31c: churn [{i}] thread={name} global={global} byte={v:#04x}"),
            &prep,
        );
        pop_thread_locale(handle);
    }
    reset_global_locale();
}
