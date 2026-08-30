// Phase C — error-path differential tests.
//
// One case per row of ERRORS.md, plus the mandated generic FFI-boundary rows.
//
// Every function in this library returns `void`, so "the same error/rejection"
// is asserted as: identical stdout bytes from both `.so`s AND the specific
// sentinel behaviour the C source implies (for a rejection, exactly zero bytes
// of output) AND the call returning normally rather than crashing. Asserting
// only "both produced the same thing" would let a translation that prints on a
// rejected input pass if it were wrong in the same way, so every case also
// pins the absolute expected bytes via `assert_same_and_eq`.
//
// `harness = false` — the cases must run sequentially; see
// `tests/common/mod.rs::Runner`.

mod common;

use common::*;
use std::ffi::c_char;

fn main() {
    let mut r = Runner::new("errors (Phase C / ERRORS.md)");

    // --- the three real rejection branches in the C source -----------------
    r.case("e1_print_line_null", e1_print_line_null);
    r.case("e2_bad_prints_nothing", e2_bad_prints_nothing);
    r.case("e3_driver_zero_prints_nothing", e3_driver_zero_prints_nothing);

    // --- mandated generic FFI-boundary rows --------------------------------
    r.case("g2_print_line_empty_string", g2_print_line_empty_string);
    r.case("g3_print_line_oversized", g3_print_line_oversized);
    r.case("g4_print_line_interior_pointer", g4_print_line_interior_pointer);
    r.case("g5_print_line_format_specifiers", g5_print_line_format_specifiers);
    r.case("g6_print_line_invalid_utf8", g6_print_line_invalid_utf8);
    r.case("g6_print_line_control_bytes", g6_print_line_control_bytes);
    r.case("g8_driver_out_of_range_ints", g8_driver_out_of_range_ints);
    r.case("g10_repeated_and_interleaved_calls", g10_repeated_and_interleaved_calls);
    r.case("g11_print_line_random_fuzz", g11_print_line_random_fuzz);

    r.finish();
}

// ===========================================================================
// E1 — printLine(NULL): `if (line != NULL)` at driver.c:30 is false.
//      Expected C result: zero bytes written, returns normally.
//      (This row doubles as G1.)
// ===========================================================================

fn e1_print_line_null() {
    // Called many times so a translation that rejects only the first NULL, or
    // that leaves stdio in a broken state after a rejection, is caught.
    assert_same_and_eq("E1 printLine(NULL)", b"", |lib| unsafe {
        lib.print_line_raw(std::ptr::null())
    });

    assert_same_and_eq("E1 printLine(NULL) x1000", b"", |lib| unsafe {
        for _ in 0..1000 {
            lib.print_line_raw(std::ptr::null());
        }
    });

    // A rejection must not poison the library: a valid call right after a
    // rejected one still has to produce output.
    let payload = b"after a rejected NULL";
    let mut expected = Vec::new();
    expected.extend_from_slice(&expected_line(payload));
    with_cstr(payload, |p| {
        assert_same_and_eq("E1 NULL then valid", &expected, |lib| unsafe {
            lib.print_line_raw(std::ptr::null());
            lib.print_line_raw(p);
            lib.print_line_raw(std::ptr::null());
        });
    });
}

// ===========================================================================
// E2 — bad(): helperBad() returns the address of its automatic `charString`
//      (CWE-562). In the reference build the compiler emits `mov $0x0,%eax`,
//      so printLine receives NULL and takes the E1 rejection path.
//      Expected C result: zero bytes. In particular the text
//      "helperBad string" must NEVER appear.
// ===========================================================================

fn e2_bad_prints_nothing() {
    assert_same_and_eq("E2 bad()", BAD_OUTPUT, |lib| unsafe { lib.bad_raw() });
    assert_same_and_eq("E2 bad() is empty", b"", |lib| unsafe { lib.bad_raw() });

    // Explicitly assert the dangling string never leaks out of either library.
    for lib in [c_lib(), rust_lib()] {
        let out = capture(|| unsafe { lib.bad_raw() });
        assert!(
            out.is_empty(),
            "{} bad() wrote {} bytes; helperBad()'s dangling pointer must be \
             rejected by printLine's NULL guard",
            lib.name,
            out.len()
        );
        assert!(
            !contains(&out, b"helperBad"),
            "{} bad() leaked the dangling automatic buffer's contents",
            lib.name
        );
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ===========================================================================
// E3 — driver(0): the `else` side of `if (useGood)` at driver.c:60 routes to
//      bad() and thus to the E2/E1 rejection. Expected C result: zero bytes.
// ===========================================================================

fn e3_driver_zero_prints_nothing() {
    assert_same_and_eq("E3 driver(0)", b"", |lib| unsafe { lib.driver_raw(0) });
    assert_same_and_eq("E3 driver(0) x1000", b"", |lib| unsafe {
        for _ in 0..1000 {
            lib.driver_raw(0);
        }
    });
    for lib in [c_lib(), rust_lib()] {
        let out = capture(|| unsafe { lib.driver_raw(0) });
        assert!(out.is_empty(), "{} driver(0) wrote {} bytes", lib.name, out.len());
        assert!(!contains(&out, b"helperBad"), "{} driver(0) leaked bad data", lib.name);
    }
}

// ===========================================================================
// G2 — zero length: a non-NULL pointer to an immediately-terminated buffer.
//      The guard passes, so stdio prints the empty string plus the newline.
//      Expected C result: exactly one byte, "\n".
// ===========================================================================

fn g2_print_line_empty_string() {
    with_cstr(b"", |p| {
        assert_same_and_eq("G2 empty", b"\n", |lib| unsafe { lib.print_line_raw(p) });
    });

    // Pointer to a large all-NUL buffer: still length zero.
    let zeros = vec![0u8; 4096];
    let p = zeros.as_ptr() as *const c_char;
    assert_same_and_eq("G2 all-NUL buffer", b"\n", |lib| unsafe { lib.print_line_raw(p) });
    assert_eq!(zeros.len(), 4096);

    // Ten empty lines in a row.
    assert_same_and_eq("G2 empty x10", b"\n\n\n\n\n\n\n\n\n\n", |lib| unsafe {
        with_cstr(b"", |p| {
            for _ in 0..10 {
                lib.print_line_raw(p);
            }
        })
    });
}

// ===========================================================================
// G3 — oversized lengths. There is no length limit in the C, so nothing may be
//      truncated at any stdio buffer boundary.
// ===========================================================================

fn g3_print_line_oversized() {
    for &len in &[1usize, 4095, 4096, 4097, 65536, 1 << 20] {
        let payload: Vec<u8> = (0..len).map(|k| 1 + (k % 255) as u8).collect();
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("G3 len={len}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }

    // Several huge payloads in one capture, to cross buffer boundaries mid-run.
    let a: Vec<u8> = vec![b'A'; (1 << 20) - 1];
    let b: Vec<u8> = vec![b'B'; 1 << 20];
    let mut expected = expected_line(&a);
    expected.extend_from_slice(&expected_line(&b));
    expected.extend_from_slice(&expected_line(&a));
    assert_same_and_eq("G3 3 MiB in one capture", &expected, |lib| unsafe {
        with_cstr(&a, |pa| {
            with_cstr(&b, |pb| {
                lib.print_line_raw(pa);
                lib.print_line_raw(pb);
                lib.print_line_raw(pa);
            })
        })
    });
}

// ===========================================================================
// G4 — interior / unaligned pointer one step inside a larger buffer.
// ===========================================================================

fn g4_print_line_interior_pointer() {
    let base: Vec<u8> = (0..256u32).map(|k| 1 + (k % 255) as u8).collect();
    let mut buf = base.clone();
    buf.push(0);
    for k in [0usize, 1, 3, 7, 15, 31, 63, 127, 200, 255] {
        let tail = buf[k..base.len()].to_vec();
        let p = unsafe { buf.as_ptr().add(k) } as *const c_char;
        assert_same_and_eq(&format!("G4 offset={k}"), &expected_line(&tail), |lib| unsafe {
            lib.print_line_raw(p)
        });
    }
    // One step past the last data byte: points exactly at the terminator.
    let p = unsafe { buf.as_ptr().add(base.len()) } as *const c_char;
    assert_same_and_eq("G4 offset=terminator", b"\n", |lib| unsafe { lib.print_line_raw(p) });
    assert_eq!(buf.len(), base.len() + 1);
}

// ===========================================================================
// G5 — printf conversion specifiers in the DATA. The C passes `line` as an
//      argument to a fixed "%s\n" format, so specifiers must be printed
//      literally. A translation that used the payload as a format string would
//      diverge (or crash on "%n"), which is exactly what this pins down.
// ===========================================================================

fn g5_print_line_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%%n",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"%099999d",
        b"%2147483647d",
        b"%.2147483647f",
        b"%1$s %2$s",
        b"%*d",
        b"%hhn",
        b"%lln",
        b"%p %x %X %o %e %g %a %c %S %C",
        b"100%",
        b"%",
        b"a%sb%nc%%d",
        b"\x25\x6e", // "%n" spelled with escapes
    ];
    for (i, payload) in cases.iter().enumerate() {
        with_cstr(payload, |p| {
            assert_same_and_eq(
                &format!("G5 #{i} {:?}", String::from_utf8_lossy(payload)),
                &expected_line(payload),
                |lib| unsafe { lib.print_line_raw(p) },
            );
        });
    }
}

// ===========================================================================
// G6 — arbitrary non-ASCII / invalid UTF-8 bytes. `char*` is a byte string;
//      a translation that validated UTF-8 would panic or substitute U+FFFD.
// ===========================================================================

fn g6_print_line_invalid_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],                         // lone continuation byte
        vec![0xbf],                         // lone continuation byte
        vec![0xc0, 0x80],                   // overlong NUL encoding (minus the NUL)
        vec![0xc2],                         // truncated 2-byte sequence
        vec![0xe0, 0xa0],                   // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92],             // truncated 4-byte sequence
        vec![0xf5, 0x80, 0x80, 0x80],       // beyond U+10FFFF
        vec![0xed, 0xa0, 0x80],             // UTF-16 surrogate half
        vec![0xfe, 0xff],                   // invalid lead bytes
        vec![0xff; 64],
        (0x80u8..=0xffu8).collect(),        // every high byte, in order
        (1u8..=0x7fu8).rev().collect(),     // every low non-NUL byte, reversed
        vec![0xf0, 0x9f, 0x92, 0xa9],       // valid UTF-8 (a 4-byte emoji), for contrast
    ];
    for (i, payload) in cases.iter().enumerate() {
        with_cstr(payload, |p| {
            assert_same_and_eq(&format!("G6 utf8 #{i}"), &expected_line(payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ===========================================================================
// G7 — embedded control bytes (recorded as `g6_print_line_control_bytes`).
// ===========================================================================

fn g6_print_line_control_bytes() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\r",
        b"\t",
        b"\x1b",
        b"a\nb",
        b"a\r\nb",
        b"\n\n\n",
        b"\r\r\r",
        b"line1\nline2\nline3",
        b"\x07\x08\x0b\x0c\x0e\x0f\x7f",
        b"\x1b[31mred\x1b[0m",
    ];
    for (i, payload) in cases.iter().enumerate() {
        with_cstr(payload, |p| {
            assert_same_and_eq(&format!("G7 ctrl #{i}"), &expected_line(payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }
}

// ===========================================================================
// G8 / G9 — out-of-range "enum" values for driver(int).
//
// C enums (and here, a plain `int` parameter) accept ANY int across the FFI
// boundary; there is no valid-variant set. The C only tests `useGood != 0`, so
// every non-zero bit pattern -- negative, extremal, one-past-the-documented
// {0,1} range -- must select good().
// ===========================================================================

fn g8_driver_out_of_range_ints() {
    let cases: &[i32] = &[
        0,
        1,
        -1,
        2,
        3,
        -2,
        42,
        -42,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x0001_0000,
        0x7FFF_FFFE,
        -0x8000_0000i64 as i32,
        0x0000_00FF,
        0x0000_FF00,
        i32::from_le_bytes([0, 0, 0, 0x80]),
        i32::from_le_bytes([0xff, 0xff, 0xff, 0x7f]),
    ];
    for &v in cases {
        let expected: &[u8] = if v != 0 { GOOD_OUTPUT } else { BAD_OUTPUT };
        assert_same_and_eq(&format!("G8 driver({v})"), expected, |lib| unsafe {
            lib.driver_raw(v)
        });
    }

    // Only exactly zero selects bad(): sweep the low bits to confirm no other
    // value is treated as false.
    for v in -64i32..=64 {
        let expected: &[u8] = if v != 0 { GOOD_OUTPUT } else { BAD_OUTPUT };
        assert_same_and_eq(&format!("G9 driver({v})"), expected, |lib| unsafe {
            lib.driver_raw(v)
        });
    }

    // A 64-bit value whose low 32 bits are zero is `0` to an `int` parameter.
    let truncated = 0x0000_0001_0000_0000u64 as u32 as i32;
    assert_eq!(truncated, 0);
    assert_same_and_eq("G8 driver(truncated 1<<32)", BAD_OUTPUT, |lib| unsafe {
        lib.driver_raw(truncated)
    });
}

// ===========================================================================
// G10 — repeated / interleaved calls. Catches a translation that consumes,
//       frees or mutates the helperGood1 static after first use, or that lets a
//       rejection put the library into a permanently silent state.
// ===========================================================================

fn g10_repeated_and_interleaved_calls() {
    let mut expected = Vec::new();
    for _ in 0..100 {
        expected.extend_from_slice(GOOD_OUTPUT); // good()
        expected.extend_from_slice(BAD_OUTPUT); // bad()      -> nothing
        expected.extend_from_slice(GOOD_OUTPUT); // driver(1)
        expected.extend_from_slice(BAD_OUTPUT); // driver(0)  -> nothing
        // printLine(NULL)                                    -> nothing
    }
    assert_same_and_eq("G10 100 interleaved rounds", &expected, |lib| unsafe {
        for _ in 0..100 {
            lib.good_raw();
            lib.bad_raw();
            lib.driver_raw(1);
            lib.driver_raw(0);
            lib.print_line_raw(std::ptr::null());
        }
    });

    // 1000 back-to-back good() calls must all print the identical line.
    let expected: Vec<u8> = GOOD_OUTPUT.repeat(1000);
    assert_same_and_eq("G10 good() x1000", &expected, |lib| unsafe {
        for _ in 0..1000 {
            lib.good_raw();
        }
    });
}

// ===========================================================================
// G11 — value-dependent fuzz over printLine, the only function that takes data.
// ===========================================================================

fn g11_print_line_random_fuzz() {
    let mut rng = Rng::new(11);
    for i in 0..2000 {
        let len = rng.below(513);
        let payload = rng.nonzero_bytes(len);
        with_cstr(&payload, |p| {
            assert_same_and_eq(&format!("G11 #{i} len={len}"), &expected_line(&payload), |lib| {
                unsafe { lib.print_line_raw(p) }
            });
        });
    }

    // Interleave fuzzed payloads with the NULL rejection so a stateful bug in
    // either path shows up.
    let mut rng = Rng::new(1111);
    let payloads: Vec<Option<Vec<u8>>> = (0..500)
        .map(|_| {
            if rng.below(3) == 0 {
                None
            } else {
                let n = rng.below(48);
                Some(rng.nonzero_bytes(n))
            }
        })
        .collect();
    let mut expected = Vec::new();
    for bytes in payloads.iter().flatten() {
        expected.extend_from_slice(&expected_line(bytes));
    }
    assert_same_and_eq("G11 fuzz interleaved with NULL", &expected, |lib| unsafe {
        for p in &payloads {
            match p {
                Some(bytes) => with_cstr(bytes, |q| lib.print_line_raw(q)),
                None => lib.print_line_raw(std::ptr::null()),
            }
        }
    });
}
