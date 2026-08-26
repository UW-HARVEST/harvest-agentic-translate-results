// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md. Every public function in this library returns
// `void`, so its only rejection signal is "no bytes written to stdout". Each
// test therefore asserts the *same* sentinel (the exact byte stream, including
// emptiness) from C and from Rust -- never merely "both survived".

mod harness;

use harness::{capture, diff_exact, libs, CBuf, Rng, SEED};
use std::ptr;

const GOOD_LINE: &[u8] = b"helperGood1 string\n";

// ---------------------------------------------------------------------------
// E1 — printLine(NULL): the `if (line != NULL)` guard at driver.c:30
// ---------------------------------------------------------------------------

#[test]
fn err_e1_print_line_null() {
    diff_exact("E1 printLine(NULL)", b"", |lib| {
        lib.print_line(ptr::null())
    });

    // Also assert the rejection is total: not one byte, not a newline.
    let (c, r) = libs();
    assert!(capture(|| c.print_line(ptr::null())).is_empty());
    assert!(capture(|| r.print_line(ptr::null())).is_empty());
}

// ---------------------------------------------------------------------------
// E2 — bad(): helperBad() yields NULL, so E1's guard fires internally
// ---------------------------------------------------------------------------

#[test]
fn err_e2_bad_is_silent() {
    diff_exact("E2 bad()", b"", |lib| lib.call_bad());

    // Pin the ground truth explicitly: the C library must not print the
    // dangling "helperBad string" -- GCC substitutes a null return.
    let (c, r) = libs();
    let out_c = capture(|| c.call_bad());
    let out_r = capture(|| r.call_bad());
    assert_eq!(out_c, Vec::<u8>::new(), "C bad() unexpectedly printed");
    assert_eq!(out_r, Vec::<u8>::new(), "Rust bad() unexpectedly printed");
    assert!(
        !out_c.windows(9).any(|w| w == b"helperBad"),
        "C bad() leaked the stack string"
    );
}

// ---------------------------------------------------------------------------
// E3 — driver(0): falsy, `else` branch into bad()
// ---------------------------------------------------------------------------

#[test]
fn err_e3_driver_zero_is_silent() {
    diff_exact("E3 driver(0)", b"", |lib| lib.call_driver(0));
}

// ---------------------------------------------------------------------------
// E4 — printLine(""): passes the guard, empty payload (distinct from NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_e4_print_line_empty_vs_null() {
    let empty = CBuf::new(b"");
    diff_exact("E4 printLine(\"\")", b"\n", |lib| {
        lib.print_line(empty.as_ptr())
    });

    // The two must be *distinguishable*, in the same direction, in both libs.
    let (c, r) = libs();
    for lib in [c, r] {
        let via_null = capture(|| lib.print_line(ptr::null()));
        let via_empty = capture(|| lib.print_line(empty.as_ptr()));
        assert_eq!(via_null, b"", "[{}] NULL must print nothing", lib.which);
        assert_eq!(via_empty, b"\n", "[{}] \"\" must print \\n", lib.which);
        assert_ne!(
            via_null, via_empty,
            "[{}] NULL and \"\" must not be conflated",
            lib.which
        );
    }
}

// ---------------------------------------------------------------------------
// G1 — repeated NULL calls: the guard must be stateless
// ---------------------------------------------------------------------------

#[test]
fn err_g1_repeated_null() {
    let mut rng = Rng::new(SEED ^ 0xE1);
    for i in 0..64 {
        let n = rng.range(2, 32);
        diff_exact(&format!("G1 iter {i} n {n}"), b"", |lib| {
            for _ in 0..n {
                lib.print_line(ptr::null());
            }
        });
    }

    // Interleaved with valid calls: a NULL must not disturb the next payload.
    let payload = b"after-null".to_vec();
    let buf = CBuf::new(&payload);
    let expected = b"after-null\nafter-null\n".to_vec();
    diff_exact("G1 null between valid calls", &expected, |lib| {
        lib.print_line(ptr::null());
        lib.print_line(buf.as_ptr());
        lib.print_line(ptr::null());
        lib.print_line(ptr::null());
        lib.print_line(buf.as_ptr());
        lib.print_line(ptr::null());
    });
}

// ---------------------------------------------------------------------------
// G2 — oversized payload: 1 MiB, far past libc's 4096-byte stdout buffer
// ---------------------------------------------------------------------------

#[test]
fn err_g2_oversized_string() {
    let mut rng = Rng::new(SEED ^ 0xE2);
    let len = 1024 * 1024;
    let payload = rng.bytes_ascii(len);
    let buf = CBuf::new(&payload);
    let mut expected = payload.clone();
    expected.push(b'\n');
    diff_exact("G2 printLine(1 MiB)", &expected, |lib| {
        lib.print_line(buf.as_ptr())
    });
}

// ---------------------------------------------------------------------------
// G3 — NUL terminator is the last byte of the allocation (no slack)
// ---------------------------------------------------------------------------

#[test]
fn err_g3_no_slack_after_terminator() {
    let mut rng = Rng::new(SEED ^ 0xE3);
    for i in 0..128 {
        let len = rng.range(0, 512);
        let payload = rng.bytes_nonzero(len);
        let buf = CBuf::new(&payload); // boxed slice: exactly len+1 bytes
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("G3 iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// G4 — format specifiers are data, never a format string
// ---------------------------------------------------------------------------

#[test]
fn err_g4_format_specifiers_not_interpreted() {
    // %n is the dangerous one: if either library ever passed `line` as the
    // format string, this would write through a bogus pointer instead of
    // printing the two characters.
    let cases: [&[u8]; 8] = [
        b"%n",
        b"%n%n%n%n%n%n%n%n",
        b"%s",
        b"%99999999d",
        b"%*s",
        b"%hn",
        b"%%n",
        b"AAAA%08x.%08x.%08x.%08x.%n",
    ];
    for case in cases {
        let buf = CBuf::new(case);
        let mut expected = case.to_vec();
        expected.push(b'\n');
        diff_exact(
            &format!("G4 {:?}", String::from_utf8_lossy(case)),
            &expected,
            |lib| lib.print_line(buf.as_ptr()),
        );
    }
}

// ---------------------------------------------------------------------------
// G5 — non-UTF-8 / high bytes: a `const char *` is bytes, not text
// ---------------------------------------------------------------------------

#[test]
fn err_g5_non_utf8_bytes() {
    // Hand-picked invalid UTF-8 sequences plus randomized high-byte noise.
    let fixed: [&[u8]; 7] = [
        &[0x80],
        &[0xff, 0xfe, 0xfd],
        &[0xc3],             // truncated 2-byte sequence
        &[0xe2, 0x82],       // truncated 3-byte sequence
        &[0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        &[0xc0, 0x80],       // overlong encoding of NUL (not a real NUL byte)
        &[0xed, 0xa0, 0x80], // surrogate
    ];
    for case in fixed {
        let buf = CBuf::new(case);
        let mut expected = case.to_vec();
        expected.push(b'\n');
        diff_exact(&format!("G5 {case:02x?}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }

    let mut rng = Rng::new(SEED ^ 0xE5);
    for i in 0..256 {
        let len = rng.range(1, 64);
        let payload: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 >> 1)).collect();
        let payload: Vec<u8> = payload.into_iter().map(|b| if b == 0 { 0x80 } else { b }).collect();
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("G5 random iter {i} len {len}"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// G6 — "out-of-range enum" analogue for driver's bare int parameter
// ---------------------------------------------------------------------------

#[test]
fn err_g6_driver_truthiness_low_byte_zero() {
    // `driver` takes an `int`, so there is no invalid variant: C truthiness is
    // evaluated over all 32 bits. Every value below is non-zero yet has a zero
    // low byte -- the classic mistranslation trap.
    for v in [
        0x0000_0100i32,
        0x0000_1000,
        0x0001_0000,
        0x0100_0000,
        0x1000_0000,
        0x4000_0000,
        i32::MIN, // 0x8000_0000: only the sign bit set
        -256,
        -65_536,
        -16_777_216,
        0x7fff_ff00,
    ] {
        diff_exact(&format!("G6 driver({v:#010x})"), GOOD_LINE, |lib| {
            lib.call_driver(v)
        });
    }

    // ...and the single value that is falsy.
    diff_exact("G6 driver(0) is the only falsy input", b"", |lib| {
        lib.call_driver(0)
    });
}

// ---------------------------------------------------------------------------
// G7 — extremes and randomized ints, including one step past the boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_g7_driver_extremes() {
    for v in [i32::MIN, i32::MIN + 1, -2, -1, 1, 2, i32::MAX - 1, i32::MAX] {
        diff_exact(&format!("G7 driver({v})"), GOOD_LINE, |lib| {
            lib.call_driver(v)
        });
    }

    let mut rng = Rng::new(SEED ^ 0xE7);
    for i in 0..512 {
        let v = rng.next_i32();
        let expected: &[u8] = if v != 0 { GOOD_LINE } else { b"" };
        diff_exact(&format!("G7 random iter {i} driver({v})"), expected, |lib| {
            lib.call_driver(v)
        });
    }
}

// ---------------------------------------------------------------------------
// G8 — terminator-adjacent single bytes
// ---------------------------------------------------------------------------

#[test]
fn err_g8_control_and_high_single_byte() {
    for b in [0x01u8, 0x07, 0x09, 0x0a, 0x0d, 0x1b, 0x7f, 0x80, 0xfe, 0xff] {
        let buf = CBuf::new(&[b]);
        let expected = [b, b'\n'];
        diff_exact(&format!("G8 printLine([{b:#04x}])"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }

    // A payload that is nothing but newlines still gets one more from puts.
    for n in 1..=8usize {
        let payload = vec![b'\n'; n];
        let buf = CBuf::new(&payload);
        let mut expected = payload.clone();
        expected.push(b'\n');
        diff_exact(&format!("G8 {n} newlines"), &expected, |lib| {
            lib.print_line(buf.as_ptr())
        });
    }
}
