//! Phase C — error-path differential tests, one test per ERRORS.md row.
//!
//! Both libraries are driven through their `.so` exports. Because every public
//! function returns `void`, the observable "error result" is the byte stream on
//! stdout (empty == rejected) plus "did the call return normally at all".

mod common;
use common::*;
use std::ffi::c_char;

/// Assert both libraries produce byte-identical output AND that the output is
/// exactly `expect`.
fn assert_same_and_eq<F>(what: &str, expect: &[u8], mut op: F)
where
    F: FnMut(&Api),
{
    let l = libs();
    let c_out = capture(|| op(&l.c));
    let r_out = capture(|| op(&l.rust));
    assert_eq!(
        c_out,
        r_out,
        "[{what}] C/Rust diverge: C={} Rust={}",
        show(&c_out),
        show(&r_out)
    );
    assert_eq!(
        c_out,
        expect.to_vec(),
        "[{what}] C did not produce the documented result: got {} want {}",
        show(&c_out),
        show(expect)
    );
}

// ===================== row 1: printLine(NULL) is rejected ==================
#[test]
fn err01_print_line_null_is_rejected_silently() {
    // `if (line != NULL)` is false -> zero bytes, no crash.
    assert_same_and_eq("err01 printLine(NULL)", b"", |api| unsafe {
        (api.print_line)(std::ptr::null())
    });
}

#[test]
fn err01b_print_line_null_repeated_and_interleaved() {
    let ok = cstr(b"ok");
    assert_same("err01b NULL interleaved with valid", |api| unsafe {
        for _ in 0..50 {
            (api.print_line)(std::ptr::null());
            (api.print_line)(ok.as_ptr() as *const c_char);
            (api.print_line)(std::ptr::null());
        }
    });
}

// ===================== row 2: empty string is NOT rejected ================
#[test]
fn err02_print_line_empty_string_prints_newline() {
    let buf = cstr(b"");
    assert_same_and_eq("err02 printLine(\"\")", b"\n", |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });
}

// ===================== row 3: leading NUL in a larger buffer ==============
#[test]
fn err03_print_line_buffer_whose_first_byte_is_nul() {
    // Non-null pointer, but zero-length string: guard passes, puts("") runs.
    let mut buf = vec![0u8; 64];
    buf[1..].fill(b'A');
    buf[0] = 0;
    assert_same_and_eq("err03 printLine(first byte NUL)", b"\n", |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });
}

// ===================== row 4: arbitrary interior pointer ==================
#[test]
fn err04_print_line_misaligned_interior_pointer() {
    let buf = cstr(b"0123456789abcdef");
    for k in 0..16usize {
        let expect = {
            let mut e = buf[k..buf.len() - 1].to_vec();
            e.push(b'\n');
            e
        };
        assert_same_and_eq(&format!("err04 offset {k}"), &expect, |api| unsafe {
            (api.print_line)(buf.as_ptr().add(k) as *const c_char)
        });
    }
    // One-past-the-NUL is the empty string, still valid and not rejected.
    let last = buf.len() - 1;
    assert_same_and_eq("err04 offset at NUL", b"\n", |api| unsafe {
        (api.print_line)(buf.as_ptr().add(last) as *const c_char)
    });
}

// ===================== row 5: oversized length ============================
#[test]
fn err05_print_line_oversized_length_not_truncated() {
    for &len in &[64 * 1024usize, 1024 * 1024] {
        let body = vec![b'Q'; len];
        let buf = cstr(&body);
        let mut expect = body.clone();
        expect.push(b'\n');
        assert_same_and_eq(&format!("err05 len {len}"), &expect, |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// ===================== row 6: format-string payloads ======================
#[test]
fn err06_print_line_format_directives_are_data_not_format() {
    // If a translation ever routed `line` into the *format* position this test
    // would diverge (or crash on %n). Both must echo verbatim.
    for c in [
        &b"%n"[..],
        &b"%s"[..],
        &b"%99999999d"[..],
        &b"%%%%"[..],
        &b"%1$s%2$s"[..],
        &b"%hhn%hn%lln"[..],
    ] {
        let buf = cstr(c);
        let mut expect = c.to_vec();
        expect.push(b'\n');
        assert_same_and_eq(
            &format!("err06 {:?}", String::from_utf8_lossy(c)),
            &expect,
            |api| unsafe { (api.print_line)(buf.as_ptr() as *const c_char) },
        );
    }
}

// ===================== row 7: high / non-UTF-8 bytes ======================
#[test]
fn err07_print_line_high_bytes_pass_through_verbatim() {
    // 0x80..=0xFF plus deliberately invalid UTF-8 sequences.
    let body: Vec<u8> = (0x80u8..=0xff).collect();
    let buf = cstr(&body);
    let mut expect = body.clone();
    expect.push(b'\n');
    assert_same_and_eq("err07 0x80..0xff", &expect, |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });

    for bad_utf8 in [
        &b"\xC3"[..],       // truncated 2-byte seq
        &b"\xE2\x82"[..],   // truncated 3-byte seq
        &b"\xF0\x9F\x98"[..], // truncated 4-byte seq
        &b"\xFF\xFE\xFD"[..], // never-valid bytes
        &b"\xED\xA0\x80"[..], // surrogate half
        &b"\xC0\x80"[..],   // overlong NUL encoding
    ] {
        let buf = cstr(bad_utf8);
        let mut expect = bad_utf8.to_vec();
        expect.push(b'\n');
        assert_same_and_eq("err07 invalid utf8", &expect, |api| unsafe {
            (api.print_line)(buf.as_ptr() as *const c_char)
        });
    }
}

// ===================== row 8: driver(0) selects the bad path ==============
#[test]
fn err08_driver_zero_selects_bad_and_never_aborts() {
    // The C `if (useGood)` false branch. Output is UB-dependent (ERRORS.md row
    // 11) and the C actually SIGSEGVs for this call shape, so the call is
    // crash-isolated in a child. The checkable contract: Rust returns normally
    // with a complete line, and 0 is genuinely routed to the `bad` branch
    // (i.e. it does NOT print "string\n" the way every non-zero value does).
    let l = libs();
    for _ in 0..64 {
        let r = run_in_child(|| unsafe { (l.rust.driver)(0) });
        assert!(r.clean_line(), "Rust driver(0) not clean: {r:?}");
        assert_ne!(
            r.out,
            b"string\n".to_vec(),
            "driver(0) must take the bad() branch, not good()"
        );
        let c = run_in_child(|| unsafe { (l.c.driver)(0) });
        assert_ne!(
            c.out,
            b"string\n".to_vec(),
            "C driver(0) must take the bad() branch, not good()"
        );
    }
}

// ===================== row 9/10: no value of useGood is rejected ==========
#[test]
fn err09_driver_out_of_range_enumlike_values_are_not_rejected() {
    // C enums accept any int. `useGood` has no valid-range check at all, so
    // every non-zero int — including values no sane enum would have — must
    // take the `good()` path in BOTH libraries and print exactly "string\n".
    let vals: &[i32] = &[
        1, 2, 3, 7, 42, 99, 1000, -1, -2, -7, -42, -99999, i32::MAX, i32::MIN,
        i32::MAX - 1, i32::MIN + 1, 0x0000_0100, 0x0001_0000, 0x4000_0000,
        -0x4000_0000,
    ];
    for &v in vals {
        assert_same_and_eq(&format!("err09 driver({v})"), b"string\n", |api| unsafe {
            (api.driver)(v)
        });
    }
}

#[test]
fn err10_driver_zero_is_the_only_falsy_value() {
    // Exhaustively probe the neighbourhood of 0 and the sign boundary: only
    // exactly 0 must diverge from "string\n".
    let l = libs();
    for v in -4096i32..=4096 {
        if v == 0 {
            // Crash-isolated: the C's bad() path is UB (see err08).
            let c = run_in_child(|| unsafe { (l.c.driver)(0) });
            let r = run_in_child(|| unsafe { (l.rust.driver)(0) });
            assert_ne!(c.out, b"string\n".to_vec(), "C driver(0) took good() path");
            assert_ne!(r.out, b"string\n".to_vec(), "Rust driver(0) took good() path");
            assert!(r.clean_line(), "Rust driver(0) not clean: {r:?}");
            continue;
        }
        let c_out = capture(|| unsafe { (l.c.driver)(v) });
        let r_out = capture(|| unsafe { (l.rust.driver)(v) });
        assert_eq!(c_out, r_out, "driver({v}) diverged");
        assert_eq!(c_out, b"string\n".to_vec(), "driver({v}) unexpected C output");
    }
}

// ===================== rows 11/12: bad() and good() ======================
#[test]
fn err11_bad_returns_normally_from_many_call_depths() {
    // `bad()` reads an uninitialised stack slot. Exercise it from several
    // distinct stack depths: neither library may abort, segfault, or emit a
    // partial line under any of them.
    fn deep(n: u32, f: &mut dyn FnMut()) {
        if n == 0 {
            f();
        } else {
            deep(n - 1, f);
            std::hint::black_box(n);
        }
    }
    let l = libs();
    for depth in [0u32, 1, 2, 5, 13, 40] {
        // Rust: must be robust at EVERY depth — no crash, no partial line.
        let mut r = None;
        deep(depth, &mut || {
            r = Some(run_in_child(|| unsafe { (l.rust.bad)() }));
        });
        let r = r.unwrap();
        assert!(
            r.clean_line(),
            "Rust bad() at depth {depth} must not crash / must emit a complete line, got {r:?}"
        );
        // C: crash-isolated observation only. Its behaviour is UB and is
        // recorded, not asserted equal (it SIGSEGVs for some call shapes).
        let mut c = None;
        deep(depth, &mut || {
            c = Some(run_in_child(|| unsafe { (l.c.bad)() }));
        });
        let c = c.unwrap();
        eprintln!(
            "err11 depth {depth}: C signal={:?} out={}  |  Rust out={}",
            c.signal,
            show(&c.out),
            show(&r.out)
        );
    }
}

#[test]
fn err12_good_has_no_failure_mode() {
    assert_same_and_eq("err12 good()", b"string\n", |api| unsafe { (api.good)() });
    // …and is still exactly "string\n" after being hammered.
    let l = libs();
    for api in [&l.c, &l.rust] {
        for _ in 0..500 {
            let out = capture(|| unsafe { (api.good)() });
            assert_eq!(out, b"string\n".to_vec(), "{} good() drifted", api.name);
        }
    }
}

// ===================== generic FFI boundary sweep =========================
#[test]
fn err_x_generic_boundary_pointers() {
    let l = invalid_but_legal_pointers();
    for (label, p) in l {
        assert_same_and_eq(&format!("errX {label}"), b"", |api| unsafe {
            // Only NULL is dereference-safe to compare; the guard rejects it.
            (api.print_line)(p)
        });
    }
}

/// Pointers that the C guard is guaranteed to reject (so no UB is introduced
/// by the test itself).
fn invalid_but_legal_pointers() -> Vec<(&'static str, *const c_char)> {
    vec![("NULL", std::ptr::null())]
}
