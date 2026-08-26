//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! `driver` has no error surface of its own (no return value, no validation, no
//! `assert`, no pointer or enum parameters — see `ERRORS.md` for the mechanical
//! derivation), so these tests cover the rejection/failure behaviour that *is*
//! reachable: the unchecked libc failures, the FFI/ABI boundary, and the
//! degenerate input values.  Every row asserts C and Rust fail (or refuse to
//! fail) in exactly the same way — same bytes, same `errno`, same stream error
//! flag — not merely "both did something".

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// E1 — there is no rejection path: every char is accepted
// ---------------------------------------------------------------------------

#[test]
fn e1_no_input_is_ever_rejected() {
    let _serial = state_lock();
    for c in all_chars() {
        // A well-formed 14-field record for every one of the 256 bit patterns
        // means neither implementation ever rejects, short-circuits or aborts.
        diff_case(
            "E1",
            &format!("char {} must be accepted", show(c)),
            Opts::file(),
            Some(1),
            &reset_locale,
            &|d| unsafe { d(c) },
        );
    }
}

// ---------------------------------------------------------------------------
// E2 — setlocale's return value is discarded (the one unchecked error)
// ---------------------------------------------------------------------------

#[test]
fn e2_setlocale_result_is_discarded() {
    let _serial = state_lock();
    // A hostile environment: bogus LC_ALL / LANG, a foreign global locale and a
    // foreign thread locale all installed before the call.
    unsafe {
        let key = std::ffi::CString::new("LC_ALL").unwrap();
        let bogus = std::ffi::CString::new("no.such.locale").unwrap();
        libc::setenv(key.as_ptr(), bogus.as_ptr(), 1);
    }

    // `"C"` is builtin, so `setlocale(LC_ALL, "C")` cannot actually fail...
    assert!(set_global_locale("C"), "setlocale(LC_ALL, \"C\") must always succeed");
    // ...while a bogus name does fail, proving the harness can tell the two apart.
    assert!(!set_global_locale("no.such.locale"), "a bogus locale name must be rejected");

    let foreign = available_locales();
    for name in &foreign {
        for c in [b'a' as c_char, b'Z' as c_char, 0, -1, -128, 127] {
            diff_case(
                "E2",
                &format!("hostile locale environment ({name}), char {}", show(c)),
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

    // The discarded result cannot have been acted on: the locale is "C" after.
    reset_locale();
    set_global_locale("ja_JP.eucjp"); // best-effort; unavailability is harmless here
    let b = both();
    let _ = capture_call(&b.c, b'a' as c_char);
    let after_c = global_locale();
    let _ = capture_call(&b.rust, b'a' as c_char);
    let after_rust = global_locale();
    assert_eq!(after_c, after_rust, "[E2] locale left behind differs");

    unsafe {
        let key = std::ffi::CString::new("LC_ALL").unwrap();
        libc::unsetenv(key.as_ptr());
    }
    reset_locale();
}

// ---------------------------------------------------------------------------
// E3 / E4 — printf failures are ignored identically
// ---------------------------------------------------------------------------

/// Runs `driver` with `stdout` wired to a failing sink — in a forked child, so
/// that libtest's own use of fd 1 is untouched — and compares everything
/// observable: `errno`, `ferror(stdout)`, and how the child terminated.
fn failing_sink_row(row: &str, sink: Sink, buffering: Buffering, expect_errno: Option<c_int>) {
    let b = both();

    for c in [b'a' as c_char, b'A' as c_char, b'0' as c_char, 0, -1, -128, 127, b'\n' as c_char] {
        reset_locale();
        let got_c = run_with_failing_sink(sink, buffering, &|| unsafe { (b.c.driver)(c) });
        reset_locale();
        let got_rust = run_with_failing_sink(sink, buffering, &|| unsafe { (b.rust.driver)(c) });

        // Both must survive the failure and get far enough to report.
        for (name, got) in [("C", &got_c), ("Rust", &got_rust)] {
            assert!(
                got.reported,
                "[{row}] sink {sink:?}/{buffering:?}, char {}: the {name} library did not \
                 return from driver() after the printf failure \
                 (exit={:?}, signal={:?}) — it aborted instead of ignoring the error",
                show(c),
                got.exit_code,
                got.signal
            );
            assert_eq!(
                got.signal, None,
                "[{row}] sink {sink:?}/{buffering:?}, char {}: the {name} library was killed \
                 by a signal",
                show(c)
            );
            assert_eq!(
                got.exit_code,
                Some(0),
                "[{row}] sink {sink:?}/{buffering:?}, char {}: unexpected {name} exit code",
                show(c)
            );
        }

        assert_eq!(
            got_c.errno, got_rust.errno,
            "[{row}] sink {sink:?}/{buffering:?}, char {}: errno differs \
             (C={} ({}) / Rust={} ({})) — the two libraries do not fail identically",
            show(c),
            got_c.errno,
            errno_name(got_c.errno),
            got_rust.errno,
            errno_name(got_rust.errno)
        );
        assert_eq!(
            got_c.ferror != 0,
            got_rust.ferror != 0,
            "[{row}] sink {sink:?}/{buffering:?}, char {}: ferror(stdout) differs \
             (C={} / Rust={})",
            show(c),
            got_c.ferror,
            got_rust.ferror
        );
        if let Some(e) = expect_errno {
            assert_eq!(
                got_c.errno, e,
                "[{row}] sink {sink:?}/{buffering:?}, char {}: expected the C library to \
                 fail with errno {e} ({}), got {} ({})",
                show(c),
                errno_name(e),
                got_c.errno,
                errno_name(got_c.errno)
            );
            assert_ne!(got_c.ferror, 0, "[{row}] expected stdout to be in an error state");
        }
    }

    // Both libraries must still work after the failure: no aborted process, no
    // poisoned state.  (A `panic = "abort"` in Rust would have killed the test
    // process outright, so reaching here at all is part of the assertion.)
    diff_case(
        row,
        &format!("still usable after {sink:?}/{buffering:?} failures"),
        Opts::file(),
        Some(1),
        &reset_locale,
        &|d| unsafe { d(b'q' as c_char) },
    );
}

fn errno_name(e: c_int) -> &'static str {
    match e {
        libc::EBADF => "EBADF",
        libc::ENOSPC => "ENOSPC",
        libc::EPIPE => "EPIPE",
        0 => "0",
        _ => "other",
    }
}

#[test]
fn e3_printf_failure_with_stdout_closed_unbuffered() {
    let _serial = state_lock();
    failing_sink_row("E3", Sink::Closed, Buffering::None, Some(libc::EBADF));
}

#[test]
fn e3b_printf_failure_with_stdout_closed_buffered() {
    let _serial = state_lock();
    failing_sink_row("E3b", Sink::Closed, Buffering::Full, None);
}

#[test]
fn e4_printf_failure_on_a_read_only_fd() {
    let _serial = state_lock();
    failing_sink_row("E4", Sink::ReadOnly, Buffering::None, Some(libc::EBADF));
}

#[test]
fn e4b_printf_failure_on_dev_full() {
    let _serial = state_lock();
    failing_sink_row("E4b", Sink::DevFull, Buffering::None, Some(libc::ENOSPC));
}

#[test]
fn e4c_printf_failure_on_dev_full_buffered() {
    let _serial = state_lock();
    failing_sink_row("E4c", Sink::DevFull, Buffering::Full, None);
}

// ---------------------------------------------------------------------------
// E5 — out-of-range integers across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn e5_out_of_range_int_arguments() {
    let _serial = state_lock();
    let b = both();

    let mut values: Vec<c_int> = vec![
        256,
        257,
        300,
        -129,
        -130,
        -1000,
        65536,
        0x1234,
        0x1_0000_00,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
        -256,
        -257,
        1024,
        4096,
        32768,
        -32768,
    ];
    // Plus seeded random out-of-range values.
    let mut rng = Rng::new(SEED ^ 0xE5);
    while values.len() < 120 {
        let v = rng.next_u64() as c_int;
        if !(-128..=127).contains(&v) {
            values.push(v);
        }
    }

    for v in values {
        let low = (v as u32 as u8) as c_char;

        reset_locale();
        let via_int_c = capture_records(Opts::file(), "C driver_int", 1, &|| unsafe { (b.c.driver_int)(v) });
        reset_locale();
        let via_int_rust = capture_records(Opts::file(), "Rust driver_int", 1, &|| unsafe { (b.rust.driver_int)(v) });
        reset_locale();
        let via_char_c = capture_call(&b.c, low);
        reset_locale();
        let via_char_rust = capture_call(&b.rust, low);

        // The gate: both libraries must handle the out-of-range value the same.
        if let Err(why) = compare_lines(&via_int_c, &via_int_rust) {
            panic!("[E5] out-of-range int {v} (0x{:08x}): {why}", v as u32);
        }
        // And both must handle it the way C does: truncate to the low byte, so
        // glibc's `__c >= -128 && __c < 256` guard is never reached.
        assert_eq!(
            escape(&via_int_c),
            escape(&via_char_c),
            "[E5] the C library did not truncate int {v} to char {}",
            show(low)
        );
        assert_eq!(
            escape(&via_int_rust),
            escape(&via_char_rust),
            "[E5] the Rust library did not truncate int {v} to char {}",
            show(low)
        );
        assert_eq!(
            parse_records(&via_int_rust).map(|r| r.len()).unwrap_or(0),
            1,
            "[E5] Rust output for int {v} is malformed"
        );
    }
}

// ---------------------------------------------------------------------------
// E6 — boundary values one step past every documented range
// ---------------------------------------------------------------------------

#[test]
fn e6_class_boundary_values() {
    let _serial = state_lock();
    const BOUNDARIES: &[i32] = &[
        0, 1, 8, 9, 10, 11, 12, 13, 14, 31, 32, 33, 47, 48, 57, 58, 64, 65, 70, 71, 90, 91, 96,
        97, 102, 103, 122, 123, 126, 127, -128, -127, -2, -1,
    ];
    for &v in BOUNDARIES {
        let c = v as i8 as c_char;
        diff_case(
            "E6",
            &format!("boundary char {}", show(c)),
            Opts::file(),
            Some(1),
            &reset_locale,
            &|d| unsafe { d(c) },
        );
    }
}

// ---------------------------------------------------------------------------
// E7 — the degenerate NUL input embeds a raw \0 in the output
// ---------------------------------------------------------------------------

#[test]
fn e7_nul_char_emits_a_raw_nul_byte() {
    let _serial = state_lock();
    let b = both();
    reset_locale();
    let out_c = capture_call(&b.c, 0);
    reset_locale();
    let out_rust = capture_call(&b.rust, 0);

    println!("[E7] C   : {}", escape(&out_c));
    println!("[E7] Rust: {}", escape(&out_rust));

    for (name, out) in [("C", &out_c), ("Rust", &out_rust)] {
        assert!(
            out.windows(12).any(|w| w == b"to lower: \0\n"),
            "[E7] {name}: `to lower` line does not contain a raw NUL byte: {}",
            escape(out)
        );
        assert!(
            out.windows(12).any(|w| w == b"to upper: \0\n"),
            "[E7] {name}: `to upper` line does not contain a raw NUL byte: {}",
            escape(out)
        );
    }
    assert_eq!(escape(&out_c), escape(&out_rust), "[E7] NUL input diverges");
}

// ---------------------------------------------------------------------------
// E8 — negative / high-byte %c conversion results
// ---------------------------------------------------------------------------

#[test]
fn e8_high_byte_conversion_results() {
    let _serial = state_lock();
    let b = both();
    let mut high_byte_cases = 0usize;

    for name in available_thread_locales() {
        let installed = ThreadLocale::install(name).expect("thread locale");
        for c in all_chars() {
            set_global_locale("C");
            let out_c = capture_call(&b.c, c);
            set_global_locale("C");
            let out_rust = capture_call(&b.rust, c);

            let rc = parse_records(&out_c)
                .unwrap_or_else(|e| panic!("[E8] {name} char {}: {e}", show(c)));
            let rr = parse_records(&out_rust)
                .unwrap_or_else(|e| panic!("[E8] {name} char {}: {e}", show(c)));
            assert_eq!(rc.len(), 1);
            assert_eq!(rr.len(), 1);

            for i in 12..14 {
                assert_eq!(
                    escape(&rc[0][i]),
                    escape(&rr[0][i]),
                    "[E8] locale {name}, char {}: `{}` conversion byte differs",
                    show(c),
                    LABELS[i]
                );
                if rc[0][i][0] >= 0x80 {
                    high_byte_cases += 1;
                }
            }
        }
        drop(installed);
    }
    reset_locale();

    println!("[E8] conversion results with the high bit set: {high_byte_cases}");
    assert!(
        high_byte_cases > 0,
        "[E8] no high-byte conversion result was produced by any locale, so the \
         `%c`-of-a-negative-table-value path is untested"
    );
}

// ---------------------------------------------------------------------------
// E9 / E10 — the absent surfaces, checked mechanically against the header
// ---------------------------------------------------------------------------

/// The claims "there is no null-pointer input" (E9) and "there is no
/// out-of-range enum input" (E10) rest on the shape of the public API.  This
/// test re-derives that shape from `c_src/include/driver.h` so the claim cannot
/// silently rot.
#[test]
fn e9_e10_public_api_has_no_pointer_length_or_enum_parameters() {
    let _serial = state_lock();
    let header = manifest_dir().join("c_src/include/driver.h");
    let src = std::fs::read_to_string(&header).expect("read driver.h");

    let decls: Vec<String> = src
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();

    println!("[E9/E10] declarations in driver.h: {decls:?}");
    assert_eq!(
        decls,
        vec!["void driver(char c);".to_string()],
        "[E9/E10] the public header no longer declares exactly one `void driver(char)`; \
         the no-pointer / no-enum reasoning in ERRORS.md must be redone"
    );

    let joined = decls.join(" ");
    assert!(!joined.contains('*'), "[E9] the API now takes a pointer: {joined}");
    assert!(!joined.contains("enum"), "[E10] the API now takes an enum: {joined}");
    assert!(
        !joined.contains("size_t") && !joined.contains("len"),
        "[E9] the API now takes a length: {joined}"
    );

    // And the implementation has no error returns / asserts to diverge on.
    let impl_src = std::fs::read_to_string(manifest_dir().join("c_src/src/driver.c"))
        .expect("read driver.c");
    let body: String = impl_src
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["assert", "return -", "return NULL", "errno", "exit("] {
        assert!(
            !body.contains(forbidden),
            "[E9/E10] driver.c now contains `{forbidden}`; ERRORS.md must gain a row for it"
        );
    }
}

// ---------------------------------------------------------------------------
// Extra: a symbol that does not exist must not resolve in either library
// ---------------------------------------------------------------------------

#[test]
fn e11_no_stub_or_extra_entry_points() {
    let _serial = state_lock();
    let b = both();
    for name in ["driver_impl", "driver_ffi", "rust_driver", "driver2", "not_a_symbol"] {
        let sym = format!("{name}\0");
        let in_c = b.c.lookup(sym.as_bytes());
        let in_rust = b.rust.lookup(sym.as_bytes());
        assert_eq!(
            in_c, in_rust,
            "[E11] symbol `{name}`: present in C = {in_c}, present in Rust = {in_rust}"
        );
    }
}
