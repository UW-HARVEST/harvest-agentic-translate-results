//! Phase B — valid-path differential tests, rows **B10–B20** of `CONFIGS.md`.
//!
//! Axis 4 (output-stream shape: interleaving with the caller's own `printf`,
//! `setvbuf` modes, pipe vs regular file), the multiplicity axis (repeats,
//! locale switching mid-sequence), the observable side effects `driver` leaves
//! behind (global/thread locale), the ABI shape of the `char` argument, and the
//! full locale × char cross-product sweep.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---------------------------------------------------------------------------
// B10 — interleaved with the caller's own printf on the shared stdout
// ---------------------------------------------------------------------------

#[test]
fn b10_interleaved_with_caller_printf() {
    let _serial = state_lock();
    let b = both();
    let mut rng = Rng::new(SEED ^ 0xB10);

    for i in 0..100 {
        let c1 = rng.char();
        let c2 = rng.char();

        let body = |d: DriverChar| {
            caller_printf("begin");
            unsafe { d(c1) };
            caller_printf("mid");
            unsafe { d(c2) };
            caller_printf("end");
        };

        // The caller's own lines must bracket the driver records exactly, i.e.
        // both streams really do interleave in call order.  This is also the
        // structural validator, so a capture polluted by libtest's own progress
        // output is retried rather than mistaken for a divergence.
        let validate = |out: &[u8]| -> Result<(), String> {
            let (begin, mid, end) =
                (&b"caller[begin]\n"[..], &b"caller[mid]\n"[..], &b"caller[end]\n"[..]);
            if !out.starts_with(begin) {
                return Err(format!("does not start with caller[begin]: {}", escape(out)));
            }
            if !out.ends_with(end) {
                return Err(format!("does not end with caller[end]: {}", escape(out)));
            }
            let rest = &out[begin.len()..];
            let midpos = find(rest, mid).ok_or_else(|| "caller[mid] missing".to_string())?;
            let rec1 = &rest[..midpos];
            let rest2 = &rest[midpos + mid.len()..];
            let rec2 = &rest2[..rest2.len() - end.len()];
            for (which, seg) in [("first", rec1), ("second", rec2)] {
                let recs = parse_records(seg).map_err(|e| format!("{which} record: {e}"))?;
                if recs.len() != 1 {
                    return Err(format!("{which} segment has {} records", recs.len()));
                }
            }
            Ok(())
        };

        let out_c = capture_valid(
            Opts::file(),
            &format!("[B10] #{i} (C)"),
            &validate,
            &|| {
                reset_locale();
                body(b.c.driver)
            },
        );
        let out_rust = capture_valid(
            Opts::file(),
            &format!("[B10] #{i} (Rust)"),
            &validate,
            &|| {
                reset_locale();
                body(b.rust.driver)
            },
        );

        if let Err(why) = compare_lines(&out_c, &out_rust) {
            panic!("[B10] #{i} chars {} / {}: {why}", show(c1), show(c2));
        }
    }
}

// ---------------------------------------------------------------------------
// B11 / B12 — forced stdout buffering modes
// ---------------------------------------------------------------------------

fn buffering_row(row: &str, buffering: Buffering, n: usize, salt: u64) {
    let mut rng = Rng::new(SEED ^ salt);
    let seq: Vec<c_char> = (0..n).map(|_| rng.char()).collect();
    let seq_ref = &seq;
    diff_case(
        row,
        &format!("{n} chars with stdout buffering = {buffering:?}"),
        Opts::buffering(buffering),
        Some(n),
        &reset_locale,
        &move |d| {
            for &c in seq_ref {
                unsafe { d(c) }
            }
        },
    );
}

#[test]
fn b11_unbuffered_stdout() {
    let _serial = state_lock();
    buffering_row("B11", Buffering::None, 64, 0xB11);
}

#[test]
fn b12_line_buffered_stdout() {
    let _serial = state_lock();
    buffering_row("B12", Buffering::Line, 64, 0xB12);
}

#[test]
fn b12b_explicitly_fully_buffered_stdout() {
    let _serial = state_lock();
    buffering_row("B12b", Buffering::Full, 64, 0xB12B);
}

// ---------------------------------------------------------------------------
// B13 — stdout is a pipe, not a regular file
// ---------------------------------------------------------------------------

#[test]
fn b13_stdout_is_a_pipe() {
    let _serial = state_lock();
    let mut rng = Rng::new(SEED ^ 0xB13);
    let seq: Vec<c_char> = (0..32).map(|_| rng.char()).collect();
    let seq_ref = &seq;
    diff_case(
        "B13",
        "32 chars with stdout on a pipe",
        Opts::sink(Sink::Pipe),
        Some(32),
        &reset_locale,
        &move |d| {
            for &c in seq_ref {
                unsafe { d(c) }
            }
        },
    );
}

// ---------------------------------------------------------------------------
// B14 — side effect: the global locale after the call
// ---------------------------------------------------------------------------

#[test]
fn b14_global_locale_after_the_call() {
    let _serial = state_lock();
    let b = both();
    let locales = available_locales();

    for name in &locales {
        reset_locale();
        assert!(set_global_locale(name));
        let before = global_locale();
        let _ = capture_call(&b.c, b'x' as c_char);
        let after_c = global_locale();

        reset_locale();
        assert!(set_global_locale(name));
        let _ = capture_call(&b.rust, b'x' as c_char);
        let after_rust = global_locale();

        println!("[B14] {name}: before={before:?} after C={after_c:?} after Rust={after_rust:?}");
        assert_eq!(
            after_c, after_rust,
            "[B14] pre-set global locale {name}: the locale left behind differs"
        );
        assert_eq!(
            after_c, "C",
            "[B14] pre-set global locale {name}: driver must leave the global locale at \"C\""
        );
    }
    reset_locale();
}

// ---------------------------------------------------------------------------
// B15 — side effect: the thread locale must survive the call
// ---------------------------------------------------------------------------

#[test]
fn b15_thread_locale_survives_the_call() {
    let _serial = state_lock();
    let b = both();
    for name in available_thread_locales() {
        let installed = ThreadLocale::install(name).expect("thread locale");
        let before = ThreadLocale::current();

        let _ = capture_call(&b.c, b'x' as c_char);
        let after_c = ThreadLocale::current();
        let _ = capture_call(&b.rust, b'x' as c_char);
        let after_rust = ThreadLocale::current();

        assert_eq!(
            before, after_c,
            "[B15] {name}: the C library changed the thread locale (unexpected)"
        );
        assert_eq!(
            after_c, after_rust,
            "[B15] {name}: C and Rust leave different thread locales installed"
        );
        assert_ne!(before, lc_global_locale(), "[B15] {name}: thread locale was not installed");
        drop(installed);
    }
    reset_locale();
}

// ---------------------------------------------------------------------------
// B16 — class-boundary values under every locale
// ---------------------------------------------------------------------------

/// Every documented class boundary and one step either side of it.
pub const BOUNDARIES: &[i32] = &[
    0, 1, 8, 9, 10, 11, 12, 13, 14, 31, 32, 33, 47, 48, 57, 58, 64, 65, 70, 71, 90, 91, 96, 97,
    102, 103, 122, 123, 126, 127, -128, -127, -2, -1,
];

#[test]
fn b16_class_boundaries_under_every_locale() {
    let _serial = state_lock();
    for name in available_thread_locales() {
        let installed = ThreadLocale::install(name).expect("thread locale");
        for &v in BOUNDARIES {
            let c = v as i8 as c_char;
            diff_case(
                "B16",
                &format!("thread locale {name}, boundary char {}", show(c)),
                Opts::file(),
                Some(1),
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
// B17 — the ABI shape of the `char` argument
// ---------------------------------------------------------------------------

#[test]
fn b17_char_argument_arriving_as_a_widened_int() {
    let _serial = state_lock();
    let b = both();

    // Every in-range bit pattern, presented both as 0..=255 and as -128..=-1.
    let mut values: Vec<c_int> = (0..=255).collect();
    values.extend(-128..=-1);

    for v in values {
        let expect = (v as u8) as c_char;

        reset_locale();
        let via_int_c = capture_records(Opts::file(), "C driver_int", 1, &|| unsafe { (b.c.driver_int)(v) });
        reset_locale();
        let via_int_rust = capture_records(Opts::file(), "Rust driver_int", 1, &|| unsafe { (b.rust.driver_int)(v) });
        reset_locale();
        let via_char_c = capture_call(&b.c, expect);

        if let Err(why) = compare_lines(&via_int_c, &via_int_rust) {
            panic!("[B17] int argument {v}: {why}");
        }
        assert_eq!(
            escape(&via_int_c),
            escape(&via_char_c),
            "[B17] int argument {v} was not truncated to char {} by the C library",
            show(expect)
        );
        let recs = parse_records(&via_int_rust).expect("[B17] malformed Rust output");
        assert_eq!(recs.len(), 1, "[B17] int argument {v}");
    }
}

// ---------------------------------------------------------------------------
// B18 — repetition / idempotence
// ---------------------------------------------------------------------------

#[test]
fn b18_repeated_identical_calls_are_idempotent() {
    let _serial = state_lock();
    let mut rng = Rng::new(SEED ^ 0xB18);
    for _ in 0..32 {
        let c = rng.char();
        let out_c = diff_case(
            "B18",
            &format!("char {} repeated 10x", show(c)),
            Opts::file(),
            Some(10),
            &reset_locale,
            &|d| {
                for _ in 0..10 {
                    unsafe { d(c) }
                }
            },
        );
        let recs = parse_records(&out_c).expect("[B18] malformed output");
        for k in 1..recs.len() {
            assert_eq!(
                recs[k].iter().map(|f| escape(f)).collect::<Vec<_>>(),
                recs[0].iter().map(|f| escape(f)).collect::<Vec<_>>(),
                "[B18] char {}: call #{k} differs from call #0 — state drifts between calls",
                show(c)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B19 — thread locale switched between calls inside one capture
// ---------------------------------------------------------------------------

#[test]
fn b19_thread_locale_switched_between_calls() {
    let _serial = state_lock();
    let names: Vec<&'static str> = available_thread_locales();
    assert!(names.len() >= 2, "need several locales for this row");
    let mut rng = Rng::new(SEED ^ 0xB19);

    for _ in 0..64 {
        let c = rng.char();
        let names_ref = &names;
        diff_case(
            "B19",
            &format!("char {} with the thread locale switched between calls", show(c)),
            Opts::file(),
            Some(names.len()),
            &reset_locale,
            &move |d| {
                for name in names_ref {
                    let tl = ThreadLocale::install(name).expect("thread locale");
                    unsafe { d(c) };
                    drop(tl);
                }
            },
        );
    }
    reset_locale();
}

// ---------------------------------------------------------------------------
// B20 — the systematic locale × char sweep, both application modes
// ---------------------------------------------------------------------------

#[test]
fn b20_full_locale_char_cross_product() {
    let _serial = state_lock();
    let global = available_locales();
    let thread = available_thread_locales();
    println!("[B20] {} global locales x 256 chars, both modes", global.len());

    // Mode 1: locale applied globally (driver's own setlocale wins).
    for name in &global {
        for c in all_chars() {
            diff_case(
                "B20/global",
                &format!("global locale {name}, char {}", show(c)),
                Opts::file(),
                Some(1),
                &|| {
                    reset_locale();
                    assert!(set_global_locale(name));
                },
                &|d| unsafe { d(c) },
            );
        }
    }

    // Mode 2: locale applied to the thread (uselocale wins over setlocale).
    for name in &thread {
        let installed = ThreadLocale::install(name).expect("thread locale");
        for c in all_chars() {
            diff_case(
                "B20/thread",
                &format!("thread locale {name}, char {}", show(c)),
                Opts::file(),
                Some(1),
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
// B21 — concurrent calls from several threads
// ---------------------------------------------------------------------------

/// `driver` mutates process-global state (`setlocale`) and writes to the shared
/// `stdout`, so concurrent use is a distinct configuration.  Line order is
/// nondeterministic, but the *multiset* of lines is not: every call contributes
/// its 14 lines.  Chars are restricted to printable ASCII so that no `%c` result
/// is itself a newline and line splitting stays well defined.
#[test]
fn b21_concurrent_calls_from_many_threads() {
    let _serial = state_lock();
    const THREADS: usize = 4;
    const CALLS: usize = 50;

    let mut rng = Rng::new(SEED ^ 0xB21);
    let plan: Vec<Vec<c_char>> = (0..THREADS)
        .map(|_| (0..CALLS).map(|_| (0x21 + rng.below(0x5E) as u8) as c_char).collect())
        .collect();

    let run = |d: DriverChar| {
        std::thread::scope(|s| {
            for lane in &plan {
                s.spawn(move || {
                    for &c in lane {
                        unsafe { d(c) }
                    }
                });
            }
        });
    };

    let validate = |out: &[u8]| -> Result<(), String> {
        let n = lines_of(out).len();
        if n == THREADS * CALLS * 14 { Ok(()) } else { Err(format!("{n} lines")) }
    };

    let b = both();
    let out_c =
        capture_valid(Opts::file(), "[B21] (C)", &validate, &|| {
            reset_locale();
            run(b.c.driver)
        });
    let out_rust =
        capture_valid(Opts::file(), "[B21] (Rust)", &validate, &|| {
            reset_locale();
            run(b.rust.driver)
        });

    let sorted = |out: &[u8]| -> Vec<String> {
        let mut v: Vec<String> = lines_of(out).iter().map(|l| escape(l)).collect();
        v.sort();
        v
    };
    let (sc, sr) = (sorted(&out_c), sorted(&out_rust));
    assert_eq!(
        sc.len(),
        THREADS * CALLS * 14,
        "[B21] expected {} lines from {THREADS} threads x {CALLS} calls",
        THREADS * CALLS * 14
    );
    assert_eq!(
        sc, sr,
        "[B21] the multiset of lines produced under concurrency differs between C and Rust"
    );
    reset_locale();
}

// ---------------------------------------------------------------------------
// B22 — enough output to cross the stdio buffer boundary many times
// ---------------------------------------------------------------------------

#[test]
fn b22_large_volume_crosses_buffer_boundaries() {
    let _serial = state_lock();
    let mut rng = Rng::new(SEED ^ 0xB22);
    let seq: Vec<c_char> = (0..2000).map(|_| rng.char()).collect();
    let seq_ref = &seq;
    diff_case(
        "B22",
        "2000 calls in one capture (~370 KiB, hundreds of buffer flushes)",
        Opts::file(),
        Some(seq.len()),
        &reset_locale,
        &move |d| {
            for &c in seq_ref {
                unsafe { d(c) }
            }
        },
    );
}
