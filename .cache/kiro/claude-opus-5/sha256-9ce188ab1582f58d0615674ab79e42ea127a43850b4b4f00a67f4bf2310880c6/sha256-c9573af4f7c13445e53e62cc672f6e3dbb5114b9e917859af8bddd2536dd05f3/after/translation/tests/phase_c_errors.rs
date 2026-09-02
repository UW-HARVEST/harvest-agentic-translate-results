// Phase C — error / rejection path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI boundary cases. Each test
// constructs the exact rejection condition, calls BOTH shared objects, and
// asserts the same rejection is observed. The library returns `void` everywhere,
// so the observable rejection signal is "no bytes written to stdout" — the tests
// assert the specific sentinel (empty output), not merely "both did something".

mod common;

use common::*;
use std::ffi::c_char;

// --- ERRORS row 1: printLine(NULL) ---------------------------------------

#[test]
fn err01_print_line_null_pointer() {
    assert_same("printLine(NULL)", |api| unsafe {
        (api.print_line)(std::ptr::null())
    });
    let c = capture(|| unsafe { (c_api().print_line)(std::ptr::null()) });
    let r = capture(|| unsafe { (rust_api().print_line)(std::ptr::null()) });
    assert!(c.is_empty(), "C printLine(NULL) wrote {c:?}");
    assert!(r.is_empty(), "Rust printLine(NULL) wrote {r:?}");
}

#[test]
fn err01b_print_line_null_repeated_and_around_valid_output() {
    // The rejection must be inert: it may not swallow, reorder or corrupt the
    // neighbouring successful writes.
    let s = b"between\0";
    assert_same("NULL x1000 interleaved", |api| unsafe {
        for _ in 0..1000 {
            (api.print_line)(std::ptr::null());
            (api.print_line)(s.as_ptr() as *const c_char);
            (api.print_line)(std::ptr::null());
        }
    });
    let c = capture(|| unsafe {
        for _ in 0..1000 {
            (c_api().print_line)(std::ptr::null());
            (c_api().print_line)(s.as_ptr() as *const c_char);
            (c_api().print_line)(std::ptr::null());
        }
    });
    assert_eq!(c.len(), b"between\n".len() * 1000);
}

// --- ERRORS row 2: zero-length payload -----------------------------------

#[test]
fn err02_print_line_zero_length_string() {
    let empty = [0u8];
    assert_same("printLine(\"\")", |api| unsafe {
        (api.print_line)(empty.as_ptr() as *const c_char)
    });
    let c = capture(|| unsafe { (c_api().print_line)(empty.as_ptr() as *const c_char) });
    let r = capture(|| unsafe { (rust_api().print_line)(empty.as_ptr() as *const c_char) });
    assert_eq!(c, b"\n", "the null guard passes; only the newline is written");
    assert_eq!(r, c);
}

// --- ERRORS row 3: leading / embedded NUL --------------------------------

#[test]
fn err03_print_line_leading_nul_in_larger_buffer() {
    // Non-null pointer, but the very first byte terminates the string.
    let mut buf = vec![b'X'; 1024];
    buf[0] = 0;
    assert_same("printLine(leading NUL)", |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const c_char)
    });
    let c = capture(|| unsafe { (c_api().print_line)(buf.as_ptr() as *const c_char) });
    assert_eq!(c, b"\n");

    // And the same for a cut at every offset in a small buffer.
    for cut in 0..64usize {
        let mut b2 = vec![b'q'; 64];
        b2[cut] = 0;
        assert_same(&format!("printLine(NUL at {cut})"), |api| unsafe {
            (api.print_line)(b2.as_ptr() as *const c_char)
        });
    }
}

// --- ERRORS row 4: bad() — dead stack address becomes NULL --------------

#[test]
fn err04_bad_rejects_via_null_from_helper_bad() {
    assert_same("bad()", |api| unsafe { (api.bad)() });
    let c = capture(|| unsafe { (c_api().bad)() });
    let r = capture(|| unsafe { (rust_api().bad)() });
    assert!(
        c.is_empty(),
        "C bad(): helperBad must yield NULL, got output {c:?}"
    );
    assert!(
        r.is_empty(),
        "Rust bad(): must reproduce the NULL rejection, got output {r:?}"
    );
}

// --- ERRORS row 5: driver(0) reaches the same rejection ----------------

#[test]
fn err05_driver_zero_selects_rejecting_path() {
    assert_same("driver(0)", |api| unsafe { (api.driver)(0) });
    let c = capture(|| unsafe { (c_api().driver)(0) });
    let r = capture(|| unsafe { (rust_api().driver)(0) });
    assert!(c.is_empty(), "C driver(0) wrote {c:?}");
    assert!(r.is_empty(), "Rust driver(0) wrote {r:?}");

    // Repeated, to be sure the silent path stays silent.
    assert_same("driver(0) x1000", |api| unsafe {
        for _ in 0..1000 {
            (api.driver)(0)
        }
    });
    assert!(capture(|| unsafe {
        for _ in 0..1000 {
            (c_api().driver)(0)
        }
    })
    .is_empty());
}

// --- ERRORS row 6 / boundary 9: out-of-range "enum-like" ints ----------

#[test]
fn err06_driver_out_of_range_enum_values() {
    // `useGood` is a bare `int`: C accepts any bit pattern, including values
    // that would have no valid variant if this were an enum. Every non-zero one
    // must take the good path in both implementations, and 0 must reject.
    let cases: &[i32] = &[
        0,
        1,
        -1,
        2,
        -2,
        3,
        42,
        -42,
        255,
        256,
        -256,
        0x7FFF,
        -0x8000,
        0x0001_0000,
        0x00FF_FF00,
        0x7FFF_FFFE,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        0xFFFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
    ];
    for &v in cases {
        assert_same(&format!("driver({v})"), |api| unsafe { (api.driver)(v) });
        let c = capture(|| unsafe { (c_api().driver)(v) });
        let r = capture(|| unsafe { (rust_api().driver)(v) });
        // Assert the *specific* sentinel, not just equality.
        if v == 0 {
            assert!(c.is_empty() && r.is_empty(), "driver(0) must be silent");
        } else {
            assert_eq!(
                c, b"helperGood1 string\n",
                "C driver({v}) must take the good path"
            );
            assert_eq!(r, c, "Rust driver({v}) diverged");
        }
    }
}

#[test]
fn err06b_driver_randomised_out_of_range() {
    let mut rng = Rng::new(SEED ^ 0x6b);
    for i in 0..2048 {
        let v = rng.next_u32() as i32;
        let c = capture(|| unsafe { (c_api().driver)(v) });
        let r = capture(|| unsafe { (rust_api().driver)(v) });
        assert_eq!(c, r, "divergence at driver({v}) iteration {i}");
        let expected: &[u8] = if v == 0 { b"" } else { b"helperGood1 string\n" };
        assert_eq!(c, expected, "C driver({v}) unexpected");
    }
}

// --- ERRORS row 7 / boundary: terminator position, no length argument ---

#[test]
fn err07_print_line_has_no_length_bound() {
    // printLine performs no bound check; both implementations must scan to the
    // same terminator. Exercised from a 1-byte payload up to 64 KiB, including
    // one-past every power-of-two boundary.
    let mut rng = Rng::new(SEED ^ 7);
    let mut lens: Vec<usize> = vec![];
    let mut n = 1usize;
    while n <= 1 << 16 {
        lens.push(n.saturating_sub(1).max(1));
        lens.push(n);
        lens.push(n + 1);
        n <<= 1;
    }
    for len in lens {
        let buf = random_cstr(&mut rng, len, true);
        assert_same(&format!("printLine(len {len}) terminator scan"), |api| {
            unsafe { (api.print_line)(buf.as_ptr() as *const c_char) }
        });
        let c = capture(|| unsafe { (c_api().print_line)(buf.as_ptr() as *const c_char) });
        assert_eq!(c.len(), len + 1, "C wrote len+newline for len {len}");
    }
}

// --- boundary 11: interior / oddly aligned pointers -------------------

#[test]
fn err11_print_line_misaligned_and_interior_pointers() {
    // `char *` has no alignment requirement; every byte offset is a legal
    // argument. Walk all 64 offsets of a 64-byte-aligned-ish buffer.
    let mut rng = Rng::new(SEED ^ 11);
    let backing = random_cstr(&mut rng, 512, true);
    for off in 0..512usize {
        let s = &backing[off..];
        assert_same(&format!("printLine(interior +{off})"), |api| unsafe {
            (api.print_line)(s.as_ptr() as *const c_char)
        });
    }
    // Pointer to the terminator itself: non-null, empty payload.
    let end = &backing[512..];
    let c = capture(|| unsafe { (c_api().print_line)(end.as_ptr() as *const c_char) });
    let r = capture(|| unsafe { (rust_api().print_line)(end.as_ptr() as *const c_char) });
    assert_eq!(c, b"\n");
    assert_eq!(r, c);
}

// --- boundary 12: format-string injection must not happen -------------

#[test]
fn err12_print_line_format_string_is_data_not_format() {
    // If either implementation ever passed `line` as the *format* argument,
    // "%n" would write through a bogus pointer / "%s" would read one. Both must
    // print the bytes verbatim instead. A divergence here is a real bug; a
    // crash in either object would fail the test by aborting the process.
    let cases: &[&[u8]] = &[
        b"%n\0",
        b"%n%n%n%n%n%n%n%n%n%n\0",
        b"%s%s%s%s%s%s%s%s%s%s%s%s%s%s%s%s\0",
        b"%99999999d\0",
        b"%.*s\0",
        b"%!\0",
        b"%\0",
        b"abc%\0",
    ];
    for c in cases {
        let expected: Vec<u8> = {
            let mut v = c[..c.len() - 1].to_vec();
            v.push(b'\n');
            v
        };
        let cout = capture(|| unsafe { (c_api().print_line)(c.as_ptr() as *const c_char) });
        let rout = capture(|| unsafe { (rust_api().print_line)(c.as_ptr() as *const c_char) });
        assert_eq!(
            cout,
            expected,
            "C must print {:?} verbatim",
            String::from_utf8_lossy(c)
        );
        assert_eq!(rout, cout, "Rust diverged on {:?}", String::from_utf8_lossy(c));
    }
}

// --- boundary: good()/bad() take no arguments, so nothing to reject ----

#[test]
fn err_good_bad_have_no_rejectable_input() {
    // Documented for completeness: `good` and `bad` are `void(void)`, so their
    // only "input" is program state. Both must be unconditionally identical.
    assert_same("good() x100 / bad() x100 alternating", |api| unsafe {
        for _ in 0..100 {
            (api.good)();
            (api.bad)();
        }
    });
}
