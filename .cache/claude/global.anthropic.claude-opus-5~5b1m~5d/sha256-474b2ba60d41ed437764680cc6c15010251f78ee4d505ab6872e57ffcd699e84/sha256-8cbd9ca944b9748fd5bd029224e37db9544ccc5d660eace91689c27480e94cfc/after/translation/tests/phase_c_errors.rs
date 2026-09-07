//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Because every public function in this
//! library is `void`, the "error result" is the byte stream on stdout: a
//! rejected input produces a specific, asserted byte sequence (often exactly
//! zero bytes). Each test asserts C and Rust agree on that exact sequence —
//! not merely that "both did something".

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

const GOOD_OUT: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";
const BAD_OUT: &[u8] = b"fffffffe\n";
const TOO_LARGE: &[u8] = b"data value is too large to perform arithmetic safely.\n";

// ---------------------------------------------------------------------------
// ERRORS row 1 — printLine(NULL): the `if(line != NULL)` guard rejects
// ---------------------------------------------------------------------------

#[test]
fn err01_print_line_null_pointer_writes_nothing() {
    // Exact sentinel: ZERO bytes of output, and no crash.
    diff_eq("printLine(NULL)", b"", |lib| unsafe {
        sym_line(lib)(std::ptr::null())
    });
}

#[test]
fn err01b_print_line_null_repeated_and_mixed() {
    // NULL must be inert even when interleaved with valid calls, so a missing
    // guard would show up as extra/garbage bytes in a known position.
    let ok = cbuf(b"ok");
    let expect = b"ok\nok\nok\n";
    diff_eq("printLine NULL interleaved", expect, |lib| unsafe {
        let f = sym_line(lib);
        f(std::ptr::null());
        f(ok.as_ptr() as *const c_char);
        f(std::ptr::null());
        f(std::ptr::null());
        f(ok.as_ptr() as *const c_char);
        f(std::ptr::null());
        f(ok.as_ptr() as *const c_char);
        f(std::ptr::null());
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 2 — printLine(""): zero-length boundary of row 1
// ---------------------------------------------------------------------------

#[test]
fn err02_print_line_empty_is_not_null() {
    let buf = cbuf(b"");
    diff_eq("printLine(\"\") != NULL", b"\n", |lib| unsafe {
        sym_line(lib)(buf.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 3 — printf conversion specifiers must be data, not format
// ---------------------------------------------------------------------------

#[test]
fn err03_print_line_format_string_injection() {
    // `%n` is the dangerous one: if the payload were ever used AS a format
    // string this would write through a bogus pointer / abort. Both libraries
    // must print it literally.
    let payloads: &[&[u8]] = &[
        b"%n", b"%n%n%n%n", b"%s", b"%99999999d", b"%p%p%p%p", b"%%n", b"%.2147483647f",
        b"%1$s", b"%hhn", b"%*d",
    ];
    for p in payloads {
        let buf = cbuf(p);
        let mut expect = p.to_vec();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine fmt-injection {:?}", String::from_utf8_lossy(p)),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 4 — oversized payload
// ---------------------------------------------------------------------------

#[test]
fn err04_print_line_oversized_payload() {
    for &len in &[1usize << 16, (1 << 20) + 1] {
        let s = vec![b'Z'; len];
        let buf = cbuf(&s);
        let mut expect = s.clone();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine oversized {len}"),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 5 — non-UTF-8 bytes must NOT be rejected or replaced
// ---------------------------------------------------------------------------

#[test]
fn err05_print_line_invalid_utf8_bytes() {
    let payloads: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xFF],
        vec![0xC0, 0x80],       // overlong encoding
        vec![0xED, 0xA0, 0x80], // UTF-16 surrogate
        vec![0xF5, 0x80, 0x80, 0x80],
        vec![0xFE, 0xFF],
        (0x80u8..=0xFF).collect(),
        (0x01u8..=0xFF).collect(),
    ];
    for p in &payloads {
        let buf = cbuf(p);
        let mut expect = p.clone();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine non-utf8 ({} bytes)", p.len()),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 6 — interior NUL silently truncates
// ---------------------------------------------------------------------------

#[test]
fn err06_print_line_interior_nul() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"a\0b", b"a\n"),
        (b"\0", b"\n"),
        (b"hello\0world\0again", b"hello\n"),
        (b"\0\0\0", b"\n"),
        (b"x\0\0y", b"x\n"),
    ];
    for (raw, expect) in cases {
        let buf = cbuf(raw);
        diff_eq(
            &format!("printLine interior NUL {:?}", raw),
            expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 7 — negative char sign-extends to eight hex digits
// ---------------------------------------------------------------------------

#[test]
fn err07_hex_negative_sign_extends() {
    for v in -128i16..=-1 {
        let v8 = v as i8;
        let expect = format!("{:08x}\n", (v8 as i32) as u32);
        diff_eq(
            &format!("printHexCharLine({v8}) must be 8 digits"),
            expect.as_bytes(),
            |lib| unsafe { sym_hex(lib)(v8 as c_char) },
        );
    }
    // Spot-check the values named in ERRORS.md.
    diff_eq("printHexCharLine(-2)", b"fffffffe\n", |lib| unsafe {
        sym_hex(lib)(-2i8 as c_char)
    });
    diff_eq("printHexCharLine(-128)", b"ffffff80\n", |lib| unsafe {
        sym_hex(lib)(-128i8 as c_char)
    });
    diff_eq("printHexCharLine(-1)", b"ffffffff\n", |lib| unsafe {
        sym_hex(lib)(-1i8 as c_char)
    });
}

// ---------------------------------------------------------------------------
// ERRORS rows 8, 9, 10 — zero / zero-pad / CHAR_MAX boundaries
// ---------------------------------------------------------------------------

#[test]
fn err08_hex_zero_boundary() {
    diff_eq("printHexCharLine(0)", b"00\n", |lib| unsafe {
        sym_hex(lib)(0)
    });
}

#[test]
fn err09_hex_zero_pad_boundary() {
    for v in 1i8..=15 {
        let expect = format!("0{v:x}\n");
        diff_eq(
            &format!("printHexCharLine({v}) zero-pad"),
            expect.as_bytes(),
            |lib| unsafe { sym_hex(lib)(v as c_char) },
        );
    }
    // one step past the zero-pad band
    diff_eq("printHexCharLine(16)", b"10\n", |lib| unsafe {
        sym_hex(lib)(16)
    });
}

#[test]
fn err10_hex_char_max_boundary() {
    // CHAR_MAX and one step either side of the signed-char boundary.
    diff_eq("printHexCharLine(127) == CHAR_MAX", b"7f\n", |lib| unsafe {
        sym_hex(lib)(127)
    });
    diff_eq("printHexCharLine(126)", b"7e\n", |lib| unsafe {
        sym_hex(lib)(126)
    });
    // 128 has no signed-char representation; the bit pattern 0x80 is -128 and
    // must wrap to the sign-extended form, NOT print "80".
    diff_eq("printHexCharLine(0x80 pattern)", b"ffffff80\n", |lib| unsafe {
        sym_hex(lib)(0x80u8 as i8 as c_char)
    });
    // ...and 0xFF likewise.
    diff_eq("printHexCharLine(0xFF pattern)", b"ffffffff\n", |lib| unsafe {
        sym_hex(lib)(0xFFu8 as i8 as c_char)
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 11 — bad(): CWE-197 truncation overflow
// ---------------------------------------------------------------------------

#[test]
fn err11_bad_truncation_overflow() {
    // 127 * 2 == 254 truncated into a signed char == -2 -> "fffffffe", NOT
    // "fe" and NOT "000000fe".
    let out = diff_eq("bad() truncation", BAD_OUT, |lib| unsafe { sym_bad(lib)() });
    let _ = out;
}

// ---------------------------------------------------------------------------
// ERRORS row 12 — goodB2G's explicit range check REJECTS
// ---------------------------------------------------------------------------

#[test]
fn err12_good_b2g_range_check_rejects() {
    // `good()` output = goodG2B ("04\n") followed by goodB2G's rejection line.
    let out = diff_eq("good() range rejection", GOOD_OUT, |lib| unsafe {
        sym_good(lib)()
    });
    let _ = out;
    // The rejection message must be present and must be the tail of the output.
    assert!(
        GOOD_OUT.ends_with(TOO_LARGE),
        "goodB2G rejection message missing from ground truth"
    );
}

// ---------------------------------------------------------------------------
// ERRORS row 13 — goodG2B's positivity guard ACCEPTS (guard-not-inverted)
// ---------------------------------------------------------------------------

#[test]
fn err13_good_g2b_guard_accepts() {
    let out = diff("good() prefix", |lib| unsafe { sym_good(lib)() });
    assert!(
        out.starts_with(b"04\n"),
        "goodG2B must emit \"04\\n\" first; got {:?}",
        String::from_utf8_lossy(&out)
    );
}

// ---------------------------------------------------------------------------
// ERRORS row 14 — driver(0) takes the bad() branch
// ---------------------------------------------------------------------------

#[test]
fn err14_driver_false_branch() {
    diff_eq("driver(0) -> bad", BAD_OUT, |lib| unsafe {
        sym_driver(lib)(0)
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 15 — driver() with out-of-range / non-boolean ints
// ---------------------------------------------------------------------------

#[test]
fn err15_driver_out_of_range_int_values() {
    // A C `int` parameter accepts ANY int. Every non-zero value must select
    // good(); this is also the "out-of-range enum/bool value crossing the FFI
    // boundary" case: there is no valid variant beyond 0/1, yet the C handles
    // all of them and the Rust must match.
    let vals: &[i32] = &[
        1,
        -1,
        2,
        -2,
        7,
        -7,
        127,
        128,
        255,
        256,
        -256,
        32767,
        32768,
        65535,
        65536,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        0x0000_0100,
        0x0001_0000,
        0x0100_0000,
        -0x0100_0000,
        0x7FFF_FF00,
        0x1234_5600,
        1 << 31u32 as i32,
    ];
    for &v in vals {
        let expect: &[u8] = if v == 0 { BAD_OUT } else { GOOD_OUT };
        diff_eq(&format!("driver({v}) out-of-range int"), expect, |lib| unsafe {
            sym_driver(lib)(v as c_int)
        });
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 16 — non-zero ints whose LOW BYTE is zero (narrowing-bug probe)
// ---------------------------------------------------------------------------

#[test]
fn err16_driver_low_byte_zero_must_be_truthy() {
    // If the Rust wrapper narrowed `useGood` to i8/u8/u16 before testing
    // truthiness, these would wrongly hit bad(). They must all hit good().
    let mut vals: Vec<i32> = vec![
        0x0000_0100,
        0x0000_FF00,
        0x0001_0000,
        0x00FF_0000,
        0x0100_0000,
        0x7F00_0000,
        i32::MIN, // 0x8000_0000 -> low 16 bits are zero too
        -0x0001_0000,
        -0x0000_0100,
    ];
    // plus randomized low-byte-zero and low-halfword-zero values
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x1616);
    for _ in 0..512 {
        let mut v = rng.next_i32() & !0xFF;
        if v == 0 {
            v = 0x100;
        }
        vals.push(v);
        let mut w = rng.next_i32() & !0xFFFF;
        if w == 0 {
            w = 0x1_0000;
        }
        vals.push(w);
    }
    for &v in &vals {
        diff_eq(
            &format!("driver({v:#010x}) low-byte-zero"),
            GOOD_OUT,
            |lib| unsafe { sym_driver(lib)(v as c_int) },
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries (required even though not table rows)
// ---------------------------------------------------------------------------

#[test]
fn generic_null_and_misaligned_pointers() {
    // NULL already covered; also check a pointer that is non-null but points at
    // an immediate terminator at an odd (misaligned) offset - C `char*` has no
    // alignment requirement, so this must behave like the empty string.
    let backing = vec![0u8; 8];
    let odd = unsafe { backing.as_ptr().add(3) };
    diff_eq("printLine(misaligned empty)", b"\n", |lib| unsafe {
        sym_line(lib)(odd as *const c_char)
    });
}

#[test]
fn generic_repeated_calls_do_not_accumulate_state() {
    // The library has no global state; calling everything many times must be
    // exactly the concatenation of the individual outputs in both libs.
    let mut expect = Vec::new();
    for _ in 0..25 {
        expect.extend_from_slice(BAD_OUT);
        expect.extend_from_slice(GOOD_OUT);
        expect.extend_from_slice(BAD_OUT); // driver(0)
        expect.extend_from_slice(GOOD_OUT); // driver(1)
    }
    diff_eq("no accumulated state", &expect, |lib| unsafe {
        let fb = sym_bad(lib);
        let fg = sym_good(lib);
        let fd = sym_driver(lib);
        for _ in 0..25 {
            fb();
            fg();
            fd(0);
            fd(1);
        }
    });
}

#[test]
fn generic_all_char_and_all_driver_values_cross_product() {
    // Every char value followed by driver(0)/driver(1) in one capture: catches
    // any state leakage between the hex formatter and the branch dispatch.
    for v in i16::from(i8::MIN)..=i16::from(i8::MAX) {
        let v8 = v as i8;
        let mut expect = format!("{:02x}\n", (v8 as i32) as u32).into_bytes();
        if v8 < 0 {
            expect = format!("{:08x}\n", (v8 as i32) as u32).into_bytes();
        }
        expect.extend_from_slice(BAD_OUT);
        expect.extend_from_slice(GOOD_OUT);
        diff_eq(&format!("cross product char {v8}"), &expect, |lib| unsafe {
            sym_hex(lib)(v8 as c_char);
            sym_driver(lib)(0);
            sym_driver(lib)(1);
        });
    }
}
