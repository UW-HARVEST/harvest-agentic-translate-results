//! Phase B — valid-path differential tests, rows **B1–B9** of `CONFIGS.md`.
//!
//! Axis 1 (14 low-level `<ctype.h>` entry points, compared line by line),
//! Axis 2 (input shape / value class, exhaustive over all 256 `char` patterns
//! plus seeded random draws) and Axis 3 (locale state: default, foreign global
//! locale, foreign *thread* locale, and thread context).
//!
//! Every case loads both `.so`s with `libloading` and calls the exported
//! `driver` symbol; nothing is linked directly.

mod common;

use common::*;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// B1 — default locale, exhaustive 256, whole-blob compare
// ---------------------------------------------------------------------------

#[test]
fn b1_default_locale_exhaustive_whole_blob() {
    let _serial = state_lock();
    for c in all_chars() {
        diff_case(
            "B1",
            &format!("char {}", show(c)),
            Opts::file(),
            Some(1),
            &reset_locale,
            &|d| unsafe { d(c) },
        );
    }
}

// ---------------------------------------------------------------------------
// B2 — the 14 interfaces individually
// ---------------------------------------------------------------------------

#[test]
fn b2_every_ctype_interface_line_by_line_exhaustive() {
    let _serial = state_lock();
    let b = both();
    for c in all_chars() {
        reset_locale();
        let out_c = capture_call(&b.c, c);
        reset_locale();
        let out_rust = capture_call(&b.rust, c);

        let recs_c = parse_records(&out_c)
            .unwrap_or_else(|e| panic!("[B2] char {}: malformed C output: {e}", show(c)));
        let recs_rust = parse_records(&out_rust)
            .unwrap_or_else(|e| panic!("[B2] char {}: malformed Rust output: {e}", show(c)));
        assert_eq!(recs_c.len(), 1, "[B2] char {}: expected one record", show(c));
        assert_eq!(recs_rust.len(), 1, "[B2] char {}: expected one record", show(c));

        for (i, label) in LABELS.iter().enumerate() {
            assert_eq!(
                escape(&recs_c[0][i]),
                escape(&recs_rust[0][i]),
                "[B2] char {}: interface `{label}` diverges",
                show(c)
            );
        }
        // The 12 classification interfaces must report an integer; the two
        // conversion interfaces exactly one byte.
        for i in 0..12 {
            assert!(
                !recs_c[0][i].is_empty() && recs_c[0][i].iter().all(|b| b.is_ascii_digit()),
                "[B2] char {}: `{}` is not an integer: {}",
                show(c),
                LABELS[i],
                escape(&recs_c[0][i])
            );
        }
        assert_eq!(recs_c[0][12].len(), 1);
        assert_eq!(recs_c[0][13].len(), 1);
    }
}

// ---------------------------------------------------------------------------
// B3 — seeded random draws
// ---------------------------------------------------------------------------

#[test]
fn b3_default_locale_randomized_draws() {
    let _serial = state_lock();
    let mut rng = Rng::new(SEED);
    for i in 0..2000 {
        let c = rng.char();
        diff_case(
            "B3",
            &format!("draw #{i} char {}", show(c)),
            Opts::file(),
            Some(1),
            &reset_locale,
            &|d| unsafe { d(c) },
        );
    }
}

// ---------------------------------------------------------------------------
// B4 — random sequences of calls inside ONE capture
// ---------------------------------------------------------------------------

#[test]
fn b4_random_sequences_in_one_capture() {
    let _serial = state_lock();
    let mut rng = Rng::new(SEED ^ 0xB4);
    for i in 0..200 {
        let len = 1 + rng.below(16) as usize;
        let seq: Vec<c_char> = (0..len).map(|_| rng.char()).collect();
        let seq_ref = &seq;
        diff_case(
            "B4",
            &format!("sequence #{i} of {len} chars"),
            Opts::file(),
            Some(len),
            &reset_locale,
            &move |d| {
                for &c in seq_ref {
                    unsafe { d(c) }
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// B5 — foreign GLOBAL locale pre-set; driver must fall back to "C"
// ---------------------------------------------------------------------------

#[test]
fn b5_foreign_global_locale_falls_back_to_c() {
    let _serial = state_lock();
    let locales = available_locales();
    assert!(locales.len() >= 2, "need more than the C locale to test this row: {locales:?}");
    println!("[B5] global locales exercised: {locales:?}");

    // Baseline: the `"C"`-locale output of the *C* implementation.
    let b = both();
    let baseline: Vec<Vec<u8>> = all_chars()
        .into_iter()
        .map(|c| {
            reset_locale();
            capture_call(&b.c, c)
        })
        .collect();

    for name in &locales {
        for (i, c) in all_chars().into_iter().enumerate() {
            let out_c = diff_case(
                "B5",
                &format!("global locale {name}, char {}", show(c)),
                Opts::file(),
                Some(1),
                &|| {
                    // Thread locale stays global so that `setlocale` wins.
                    reset_locale();
                    assert!(set_global_locale(name), "setlocale({name}) failed");
                },
                &|d| unsafe { d(c) },
            );
            assert_eq!(
                escape(&out_c),
                escape(&baseline[i]),
                "[B5] global locale {name}, char {}: driver's setlocale(LC_ALL,\"C\") \
                 did not override the pre-set global locale",
                show(c)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B6 — foreign THREAD locale (uselocale wins over setlocale)
// ---------------------------------------------------------------------------

#[test]
fn b6_foreign_thread_locale_cross_product() {
    let _serial = state_lock();
    let locales = available_thread_locales();
    assert!(locales.len() >= 2, "need more than the C locale to test this row: {locales:?}");
    println!("[B6] thread locales exercised: {locales:?}");

    for name in &locales {
        let installed = ThreadLocale::install(name).expect("thread locale");
        for c in all_chars() {
            diff_case(
                "B6",
                &format!("thread locale {name}, char {}", show(c)),
                Opts::file(),
                Some(1),
                // Only the *global* locale is normalised here; resetting the
                // thread locale would uninstall the very thing under test.
                &|| {
                    set_global_locale("C");
                },
                &|d| unsafe { d(c) },
            );
        }
        drop(installed);
    }
    reset_locale();
}

// ---------------------------------------------------------------------------
// B7 — tr_TR.iso88599: proves the thread-locale row is not a no-op
// ---------------------------------------------------------------------------

#[test]
fn b7_turkish_thread_locale_differs_from_c_baseline() {
    let _serial = state_lock();
    let b = both();
    let Some(installed) = ThreadLocale::install("tr_TR.iso88599") else {
        panic!("tr_TR.iso88599 is unavailable; this row cannot be verified");
    };

    let mut differing = 0usize;
    let mut high_byte_outputs = 0usize;

    for c in all_chars() {
        set_global_locale("C");
        let out_c = capture_call(&b.c, c);
        set_global_locale("C");
        let out_rust = capture_call(&b.rust, c);

        if let Err(why) = compare_lines(&out_c, &out_rust) {
            drop(installed);
            reset_locale();
            panic!("[B7] thread locale tr_TR.iso88599, char {}: {why}", show(c));
        }

        // Compare against the "C"-locale answer for the same char: temporarily
        // swap the thread locale back to "C", ask again, then let the Turkish
        // one be reinstated by `Drop`.
        {
            let c_locale = ThreadLocale::install("C").expect("C thread locale");
            let c_locale_out = capture_call(&b.c, c);
            if c_locale_out != out_c {
                differing += 1;
            }
            drop(c_locale);
        }

        if lines_of(&out_c).iter().any(|l| l.iter().any(|&x| x >= 0x80)) {
            high_byte_outputs += 1;
        }
    }

    drop(installed);
    reset_locale();

    println!(
        "[B7] chars whose output differs from the C locale: {differing}; \
         chars producing high-byte output: {high_byte_outputs}"
    );
    assert!(
        differing > 0,
        "[B7] tr_TR.iso88599 produced the same output as the C locale for every \
         char — the thread-locale row is not exercising a different table"
    );
    assert!(
        high_byte_outputs > 0,
        "[B7] no high-byte (>= 0x80) conversion result was produced, so the \
         negative-`%c` path is untested by this row"
    );
}

// ---------------------------------------------------------------------------
// B8 — called from a spawned (non-main) thread
// ---------------------------------------------------------------------------

#[test]
fn b8_called_from_non_main_thread() {
    let handle = std::thread::spawn(|| {
        let mut rng = Rng::new(SEED ^ 0xB8);
        for i in 0..256 {
            let c = rng.char();
            diff_case(
                "B8",
                &format!("worker thread draw #{i} char {}", show(c)),
                Opts::file(),
                Some(1),
                &reset_locale,
                &|d| unsafe { d(c) },
            );
        }
    });
    handle.join().expect("worker thread panicked");
}

// ---------------------------------------------------------------------------
// B9 — non-main thread with its own uselocale
// ---------------------------------------------------------------------------

#[test]
fn b9_non_main_thread_with_own_thread_locale() {
    let handle = std::thread::spawn(|| {
        let Some(installed) = ThreadLocale::install("ru_RU.koi8r") else {
            panic!("ru_RU.koi8r is unavailable; this row cannot be verified");
        };
        for c in all_chars() {
            diff_case(
                "B9",
                &format!("worker thread, thread locale ru_RU.koi8r, char {}", show(c)),
                Opts::file(),
                Some(1),
                &|| {
                    set_global_locale("C");
                },
                &|d| unsafe { d(c) },
            );
        }
        drop(installed);
    });
    handle.join().expect("worker thread panicked");
    reset_locale();
}
