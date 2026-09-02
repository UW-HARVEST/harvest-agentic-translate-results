//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each test constructs the exact invalid
//! input/condition, calls BOTH `.so`s, asserts they agree byte-for-byte, and
//! additionally pins the *specific* sentinel the C returns (`-1` vs. a
//! non-negative length) so "both failed somehow" cannot pass for the wrong
//! reason.

mod common;

use common::*;
use std::ffi::c_char;
use std::ffi::c_int;

/// Call the C library only, to learn/pin the exact sentinel.
fn c_ret(call: &Call<'_>) -> c_int {
    raw_ret(libs().c, call)
}

fn rs_ret(call: &Call<'_>) -> c_int {
    raw_ret(libs().rs, call)
}

fn raw_ret(f: Hex2BinFn, call: &Call<'_>) -> c_int {
    let mut buf: Vec<u8> = match call.bin {
        BinArg::Buf(n) => vec![SENTINEL; n],
        BinArg::Null => Vec::new(),
    };
    let bin_ptr = match call.bin {
        BinArg::Buf(_) => buf.as_mut_ptr(),
        BinArg::Null => std::ptr::null_mut(),
    };
    let hex_owned: Vec<u8>;
    let hex_ptr = match call.hex {
        HexArg::Bytes(b) => {
            hex_owned = b.to_vec();
            hex_owned.as_ptr() as *const c_char
        }
        HexArg::Null => {
            hex_owned = Vec::new();
            std::ptr::null()
        }
    };
    let ig_owned: Vec<u8>;
    let ig_ptr = match call.ignore {
        Some(s) => {
            let mut v = s.to_vec();
            v.push(0);
            ig_owned = v;
            ig_owned.as_ptr() as *const c_char
        }
        None => {
            ig_owned = Vec::new();
            std::ptr::null()
        }
    };
    let mut end: *const c_char = std::ptr::null();
    let end_ptr = if call.want_hex_end {
        &mut end as *mut *const c_char
    } else {
        std::ptr::null_mut()
    };
    let r = unsafe {
        f(
            bin_ptr,
            call.bin_maxlen,
            hex_ptr,
            call.hex_len,
            ig_ptr,
            end_ptr,
        )
    };
    let _ = (&hex_owned, &ig_owned, &buf);
    r
}

/// Assert C and Rust agree AND that the shared result is exactly `expect`.
#[track_caller]
fn assert_same_and_ret(label: &str, call: &Call<'_>, expect: c_int) {
    assert_same(label, call);
    let c = c_ret(call);
    let r = rs_ret(call);
    assert_eq!(c, expect, "[{label}] C returned {c}, expected sentinel {expect}");
    assert_eq!(r, expect, "[{label}] Rust returned {r}, expected sentinel {expect}");
}

fn call<'a>(
    bin_maxlen: usize,
    hex: &'a [u8],
    ignore: Option<&'a [u8]>,
    want_hex_end: bool,
) -> Call<'a> {
    Call {
        bin: BinArg::Buf(bin_maxlen + 8),
        bin_maxlen,
        hex: HexArg::Bytes(hex),
        hex_len: hex.len(),
        ignore,
        want_hex_end,
    }
}

/// Offset written into `*hex_end_p`, read from the C library (ground truth) and
/// asserted equal for Rust via `assert_same`.
fn c_hex_end_off(call: &Call<'_>) -> usize {
    assert!(call.want_hex_end);
    let l = libs();
    let mut buf: Vec<u8> = match call.bin {
        BinArg::Buf(n) => vec![SENTINEL; n],
        BinArg::Null => Vec::new(),
    };
    let bin_ptr = match call.bin {
        BinArg::Buf(_) => buf.as_mut_ptr(),
        BinArg::Null => std::ptr::null_mut(),
    };
    let hex_owned: Vec<u8> = match call.hex {
        HexArg::Bytes(b) => b.to_vec(),
        HexArg::Null => Vec::new(),
    };
    let hex_ptr = match call.hex {
        HexArg::Bytes(_) => hex_owned.as_ptr() as *const c_char,
        HexArg::Null => std::ptr::null(),
    };
    let ig_owned: Vec<u8> = match call.ignore {
        Some(s) => {
            let mut v = s.to_vec();
            v.push(0);
            v
        }
        None => Vec::new(),
    };
    let ig_ptr = match call.ignore {
        Some(_) => ig_owned.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };
    let mut end: *const c_char = std::ptr::null();
    unsafe {
        (l.c)(
            bin_ptr,
            call.bin_maxlen,
            hex_ptr,
            call.hex_len,
            ig_ptr,
            &mut end,
        );
    }
    (end as usize).wrapping_sub(hex_ptr as usize)
}

// ===========================================================================
// Row 1 — bin_maxlen == 0 with a leading hex digit
// ===========================================================================

#[test]
fn errors_md_row_01_bin_maxlen_zero() {
    let __before = comparisons();
    for &d in MIXED {
        for want_end in [false, true] {
            let hex = [d, d];
            let c = call(0, &hex, None, want_end);
            assert_same_and_ret(&format!("row01/{}", d as char), &c, -1);
            if want_end {
                assert_eq!(c_hex_end_off(&c), 0, "row01: *hex_end_p must be &hex[0]");
            }
        }
    }
    // Randomised: any all-hex input with bin_maxlen == 0 must be -1.
    let mut rng = Rng::new(SEED ^ 0x101);
    for i in 0..300 {
        let n = rng.range(1, 32);
        let hex = rand_hex(&mut rng, n, MIXED);
        assert_same_and_ret(&format!("row01/rand#{i}"), &call(0, &hex, None, true), -1);
    }
    assert_did_work("errors_md_row_01_bin_maxlen_zero", __before, 344);
}

// ===========================================================================
// Row 2 — bin_maxlen < hex_len/2
// ===========================================================================

#[test]
fn errors_md_row_02_bin_maxlen_too_small() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x102);
    for i in 0..400 {
        let bytes = rng.range(1, 24);
        let hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        let maxlen = rng.range(0, bytes - 1);
        let c = call(maxlen, &hex, None, true);
        assert_same_and_ret(&format!("row02#{i}"), &c, -1);
        assert_eq!(
            c_hex_end_off(&c),
            2 * maxlen,
            "row02: *hex_end_p must point at the first unconsumable digit"
        );
    }
    assert_did_work("errors_md_row_02_bin_maxlen_too_small", __before, 400);
}

// ===========================================================================
// Row 3 — odd digit count (state != 0 at loop exit)
// ===========================================================================

#[test]
fn errors_md_row_03_odd_digit_count() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x103);
    for i in 0..400 {
        let nibbles = 2 * rng.range(0, 24) + 1;
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        // bin_maxlen large enough that only the odd-nibble error can fire.
        let c = call(nibbles, &hex, None, true);
        assert_same_and_ret(&format!("row03#{i}"), &c, -1);
        assert_eq!(
            c_hex_end_off(&c),
            nibbles - 1,
            "row03: hex_pos-- must leave *hex_end_p on the last digit"
        );
    }
    assert_did_work("errors_md_row_03_odd_digit_count", __before, 400);
}

// ===========================================================================
// Row 4 — separator mid-byte makes the digit count odd
// ===========================================================================

#[test]
fn errors_md_row_04_separator_mid_byte() {
    let __before = comparisons();
    // "ab:c:d" -> "ab" ok, then 'c' (state 0->0xFF), then ':' with state != 0
    let cases: &[(&[u8], usize)] = &[
        (b"ab:c:d", 3),
        (b"a:b", 0),
        (b"0123:4:", 5),
        (b"f-", 0),
        (b"aabb:c ", 5),
    ];
    for (hex, expect_end) in cases {
        let c = call(16, hex, Some(b": -"), true);
        assert_same_and_ret(
            &format!("row04/{}", String::from_utf8_lossy(hex)),
            &c,
            -1,
        );
        assert_eq!(
            c_hex_end_off(&c),
            *expect_end,
            "row04/{}: hex_end offset",
            String::from_utf8_lossy(hex)
        );
    }

    // Randomised: build "<even digits><digit><sep>..." which is always odd-broken.
    let mut rng = Rng::new(SEED ^ 0x104);
    for i in 0..300 {
        let nb = 2 * rng.range(0, 8);
        let mut hex = rand_hex(&mut rng, nb, MIXED);
        hex.push(*rng.pick(MIXED));
        hex.push(*rng.pick(b": -"));
        let extra = rng.range(0, 4);
        for _ in 0..extra {
            hex.push(*rng.pick(MIXED));
        }
        assert_same_and_ret(&format!("row04/rand#{i}"), &call(64, &hex, Some(b": -"), true), -1);
    }
    assert_did_work("errors_md_row_04_separator_mid_byte", __before, 305);
}

// ===========================================================================
// Row 5 — hex_end_p == NULL and a non-hex byte stops parsing early
// ===========================================================================

#[test]
fn errors_md_row_05_null_hex_end_early_stop() {
    let __before = comparisons();
    let cases: &[&[u8]] = &[b"aabb!", b"!aabb", b"00zz", b"12 34", b"aabb\x00"];
    for hex in cases {
        assert_same_and_ret(
            &format!("row05/{}", String::from_utf8_lossy(hex)),
            &call(16, hex, None, false),
            -1,
        );
    }
    let mut rng = Rng::new(SEED ^ 0x105);
    for i in 0..400 {
        let nb = 2 * rng.range(0, 12);
        let mut hex = rand_hex(&mut rng, nb, MIXED);
        hex.push(*rng.pick(ADJACENT));
        let tail = rng.range(0, 6);
        for _ in 0..tail {
            hex.push(*rng.pick(MIXED));
        }
        assert_same_and_ret(&format!("row05/rand#{i}"), &call(64, &hex, None, false), -1);
    }
    assert_did_work("errors_md_row_05_null_hex_end_early_stop", __before, 405);
}

// ===========================================================================
// Row 6 — hex_end_p == NULL and buffer-full
// ===========================================================================

#[test]
fn errors_md_row_06_null_hex_end_buffer_full() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x106);
    for i in 0..400 {
        let bytes = rng.range(1, 24);
        let hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        let maxlen = rng.range(0, bytes - 1);
        assert_same_and_ret(&format!("row06#{i}"), &call(maxlen, &hex, None, false), -1);
    }
    assert_did_work("errors_md_row_06_null_hex_end_buffer_full", __before, 400);
}

// ===========================================================================
// Row 7 — hex_end_p == NULL and odd digit count
// ===========================================================================

#[test]
fn errors_md_row_07_null_hex_end_odd_count() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x107);
    for i in 0..400 {
        let nibbles = 2 * rng.range(0, 24) + 1;
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        assert_same_and_ret(&format!("row07#{i}"), &call(nibbles, &hex, None, false), -1);
    }
    assert_did_work("errors_md_row_07_null_hex_end_odd_count", __before, 400);
}

// ===========================================================================
// Row 8 — non-hex byte with ignore == NULL
// ===========================================================================

#[test]
fn errors_md_row_08_non_hex_with_null_ignore() {
    let __before = comparisons();
    // With hex_end_p set and an even number of digits consumed, the C reports
    // SUCCESS with the partial length — pin that exact value.
    let mut rng = Rng::new(SEED ^ 0x108);
    for i in 0..400 {
        let bytes = rng.range(0, 12);
        let mut hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        hex.push(*rng.pick(ADJACENT));
        let c = call(64, &hex, None, true);
        assert_same_and_ret(&format!("row08#{i}"), &c, bytes as c_int);
        assert_eq!(
            c_hex_end_off(&c),
            2 * bytes,
            "row08: *hex_end_p must sit on the offending byte"
        );
        // Same input, hex_end_p == NULL => -1 (unless nothing was left over,
        // which cannot happen because we appended a stopping byte).
        assert_same_and_ret(&format!("row08/nullend#{i}"), &call(64, &hex, None, false), -1);
    }
    assert_did_work("errors_md_row_08_non_hex_with_null_ignore", __before, 800);
}

// ===========================================================================
// Row 9 — non-hex byte NOT in the ignore set
// ===========================================================================

#[test]
fn errors_md_row_09_non_hex_not_in_ignore_set() {
    let __before = comparisons();
    let ignore: &[u8] = b": -";
    let mut rng = Rng::new(SEED ^ 0x109);
    // Junk bytes guaranteed absent from `ignore`.
    let junk: Vec<u8> = ADJACENT
        .iter()
        .copied()
        .filter(|b| !ignore.contains(b))
        .collect();
    for i in 0..400 {
        let bytes = rng.range(0, 12);
        let mut hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        hex.push(*rng.pick(&junk));
        let c = call(64, &hex, Some(ignore), true);
        assert_same_and_ret(&format!("row09#{i}"), &c, bytes as c_int);
        assert_eq!(c_hex_end_off(&c), 2 * bytes);
        assert_same_and_ret(
            &format!("row09/nullend#{i}"),
            &call(64, &hex, Some(ignore), false),
            -1,
        );
    }
    assert_did_work("errors_md_row_09_non_hex_not_in_ignore_set", __before, 800);
}

// ===========================================================================
// Row 10 — first byte is non-hex, hex_end_p set => returns 0 (not an error)
// ===========================================================================

#[test]
fn errors_md_row_10_first_byte_non_hex_returns_zero() {
    let __before = comparisons();
    for b in 0u8..=255 {
        if is_hex_digit(b) {
            continue;
        }
        let hex = [b, b'a', b'a'];
        // ignore = NULL => every non-hex byte (including 0x00) stops parsing.
        let c = call(8, &hex, None, true);
        assert_same_and_ret(&format!("row10/0x{b:02x}"), &c, 0);
        assert_eq!(c_hex_end_off(&c), 0, "row10/0x{b:02x}: *hex_end_p == &hex[0]");
    }
    assert_did_work("errors_md_row_10_first_byte_non_hex_returns_zero", __before, 234);
}

// ===========================================================================
// Row 11 — NUL byte matches the terminator of `ignore` (the quirk)
// ===========================================================================

#[test]
fn errors_md_row_11_nul_matches_ignore_terminator() {
    let __before = comparisons();
    // ignore = "" and a NUL between bytes: the NUL is SKIPPED, so "aa\0bb"
    // decodes to two bytes and consumes the whole input.
    let hex: &[u8] = b"aa\x00bb";
    let c = call(8, hex, Some(b""), true);
    assert_same_and_ret("row11/empty-ignore", &c, 2);
    assert_eq!(
        c_hex_end_off(&c),
        hex.len(),
        "row11: the NUL must be consumed, not stop parsing"
    );

    // Non-empty ignore that does NOT list 0x00 explicitly — still skipped.
    let c2 = call(8, hex, Some(b": -"), true);
    assert_same_and_ret("row11/sep-ignore", &c2, 2);
    assert_eq!(c_hex_end_off(&c2), hex.len());

    // Randomised sprinkling of NULs at even (state == 0) positions.
    let mut rng = Rng::new(SEED ^ 0x10b);
    for i in 0..400 {
        let bytes = rng.range(1, 12);
        let mut hex = Vec::new();
        for b in 0..bytes {
            if b > 0 {
                // Intentional: a run of NUL bytes at an even (state == 0)
                // position, which the C skips via the strchr-terminator quirk.
                let nuls = rng.range(0, 3);
                hex.extend(std::iter::repeat_n(0u8, nuls));
            }
            hex.push(*rng.pick(MIXED));
            hex.push(*rng.pick(MIXED));
        }
        let c = call(bytes, &hex, Some(b""), true);
        assert_same_and_ret(&format!("row11/rand#{i}"), &c, bytes as c_int);
        assert_eq!(c_hex_end_off(&c), hex.len());
    }
    assert_did_work("errors_md_row_11_nul_matches_ignore_terminator", __before, 402);
}

// ===========================================================================
// Row 12 — NUL byte with ignore == NULL
// ===========================================================================

#[test]
fn errors_md_row_12_nul_with_null_ignore() {
    let __before = comparisons();
    let hex: &[u8] = b"aa\x00bb";
    let c = call(8, hex, None, true);
    // "aa" -> ONE decoded byte, then the NUL stops parsing (ignore == NULL).
    assert_same_and_ret("row12/end-set", &c, 1);
    assert_eq!(c_hex_end_off(&c), 2, "row12: parsing must stop at the NUL");
    assert_same_and_ret("row12/end-null", &call(8, hex, None, false), -1);
    assert_did_work("errors_md_row_12_nul_with_null_ignore", __before, 2);
}

// ===========================================================================
// Row 13 — NUL byte with ignore != NULL but state != 0
// ===========================================================================

#[test]
fn errors_md_row_13_nul_mid_byte_with_ignore() {
    let __before = comparisons();
    // "aaa\0" -> 'a','a' -> byte; 'a' -> state != 0; NUL not skipped (state!=0)
    let hex: &[u8] = b"aaa\x00bb";
    let c = call(8, hex, Some(b""), true);
    assert_same_and_ret("row13", &c, -1);
    assert_eq!(c_hex_end_off(&c), 2, "row13: hex_pos-- lands on the odd digit");
    assert_same_and_ret("row13/end-null", &call(8, hex, Some(b""), false), -1);
    assert_did_work("errors_md_row_13_nul_mid_byte_with_ignore", __before, 2);
}

// ===========================================================================
// Row 14 — high bytes 0x80..=0xFF
// ===========================================================================

#[test]
fn errors_md_row_14_high_bytes() {
    let __before = comparisons();
    for b in 0x80u8..=0xff {
        let hex = [b'a', b'a', b, b'b', b'b'];
        // Not ignorable => stops after the first byte, returns 1.
        let c = call(8, &hex, None, true);
        assert_same_and_ret(&format!("row14/0x{b:02x}/none"), &c, 1);
        assert_eq!(c_hex_end_off(&c), 2);
        // hex_end_p == NULL => -1
        assert_same_and_ret(&format!("row14/0x{b:02x}/nullend"), &call(8, &hex, None, false), -1);
        // Listed in `ignore` => skipped, decodes both bytes.
        let ig = [b];
        let c2 = call(8, &hex, Some(&ig), true);
        assert_same_and_ret(&format!("row14/0x{b:02x}/ignored"), &c2, 2);
        assert_eq!(c_hex_end_off(&c2), hex.len());
    }
    assert_did_work("errors_md_row_14_high_bytes", __before, 384);
}

// ===========================================================================
// Row 15 — bytes adjacent to the hex ranges
// ===========================================================================

#[test]
fn errors_md_row_15_range_adjacent_bytes() {
    let __before = comparisons();
    for &b in b"/:@G`g" {
        let hex = [b'a', b'a', b, b'b', b'b'];
        let c = call(8, &hex, None, true);
        assert_same_and_ret(&format!("row15/{}", b as char), &c, 1);
        assert_eq!(c_hex_end_off(&c), 2, "row15/{}: must reject", b as char);
        assert_same_and_ret(
            &format!("row15/{}/nullend", b as char),
            &call(8, &hex, None, false),
            -1,
        );
    }
    // ...and the digits immediately *inside* the ranges must be accepted.
    for &b in b"09afAF" {
        let hex = [b'a', b'a', b, b'b'];
        let c = call(8, &hex, None, true);
        assert_same_and_ret(&format!("row15/inside/{}", b as char), &c, 2);
        assert_eq!(c_hex_end_off(&c), hex.len());
    }
    assert_did_work("errors_md_row_15_range_adjacent_bytes", __before, 18);
}

// ===========================================================================
// Row 16 — hex_len == 0
// ===========================================================================

#[test]
fn errors_md_row_16_hex_len_zero() {
    let __before = comparisons();
    for ignore in [None, Some(&b""[..]), Some(&b": "[..])] {
        for maxlen in [0usize, 1, 16] {
            let c = Call {
                bin: BinArg::Buf(maxlen + 8),
                bin_maxlen: maxlen,
                hex: HexArg::Bytes(b"aabbcc"),
                hex_len: 0,
                ignore,
                want_hex_end: true,
            };
            assert_same_and_ret("row16/end-set", &c, 0);
            assert_eq!(c_hex_end_off(&c), 0);
            let mut c2 = c.clone();
            c2.want_hex_end = false;
            assert_same_and_ret("row16/end-null", &c2, 0);
        }
    }
    assert_did_work("errors_md_row_16_hex_len_zero", __before, 18);
}

// ===========================================================================
// Row 17 — hex == NULL, hex_len == 0, hex_end_p set
// ===========================================================================

#[test]
fn errors_md_row_17_null_hex_with_hex_end() {
    let __before = comparisons();
    for ignore in [None, Some(&b""[..]), Some(&b": "[..])] {
        let c = Call {
            bin: BinArg::Buf(8),
            bin_maxlen: 8,
            hex: HexArg::Null,
            hex_len: 0,
            ignore,
            want_hex_end: true,
        };
        assert_same_and_ret("row17", &c, 0);
        // *hex_end_p must be NULL itself (offset 0 from a NULL base).
        assert_eq!(c_hex_end_off(&c), 0);
    }
    assert_did_work("errors_md_row_17_null_hex_with_hex_end", __before, 3);
}

// ===========================================================================
// Row 18 — bin == NULL, bin_maxlen == 0, hex has digits
// ===========================================================================

#[test]
fn errors_md_row_18_null_bin_with_digits() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x112);
    for i in 0..200 {
        let nb = rng.range(1, 16);
        let hex = rand_hex(&mut rng, nb, MIXED);
        for want_end in [false, true] {
            let c = Call {
                bin: BinArg::Null,
                bin_maxlen: 0,
                hex: HexArg::Bytes(&hex),
                hex_len: hex.len(),
                ignore: None,
                want_hex_end: want_end,
            };
            assert_same_and_ret(&format!("row18#{i}"), &c, -1);
        }
    }
    assert_did_work("errors_md_row_18_null_bin_with_digits", __before, 400);
}

// ===========================================================================
// Row 19 — bin == NULL, bin_maxlen == 0, hex_len == 0
// ===========================================================================

#[test]
fn errors_md_row_19_null_bin_zero_len() {
    let __before = comparisons();
    for ignore in [None, Some(&b""[..]), Some(&b": "[..])] {
        for want_end in [false, true] {
            let c = Call {
                bin: BinArg::Null,
                bin_maxlen: 0,
                hex: HexArg::Null,
                hex_len: 0,
                ignore,
                want_hex_end: want_end,
            };
            assert_same_and_ret("row19", &c, 0);
        }
    }
    assert_did_work("errors_md_row_19_null_bin_zero_len", __before, 6);
}

// ===========================================================================
// Row 20 — bin_maxlen == SIZE_MAX with an even input
// ===========================================================================

#[test]
fn errors_md_row_20_size_max_even_input() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x114);
    for i in 0..200 {
        let bytes = rng.range(1, 24);
        let hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        let c = Call {
            bin: BinArg::Buf(bytes + 8),
            bin_maxlen: usize::MAX,
            hex: HexArg::Bytes(&hex),
            hex_len: hex.len(),
            ignore: None,
            want_hex_end: true,
        };
        assert_same_and_ret(&format!("row20#{i}"), &c, bytes as c_int);
    }
    assert_did_work("errors_md_row_20_size_max_even_input", __before, 200);
}

// ===========================================================================
// Row 21 — bin_maxlen == SIZE_MAX with an odd input
// ===========================================================================

#[test]
fn errors_md_row_21_size_max_odd_input() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x115);
    for i in 0..200 {
        let nibbles = 2 * rng.range(0, 24) + 1;
        let hex = rand_hex(&mut rng, nibbles, MIXED);
        let c = Call {
            bin: BinArg::Buf(nibbles + 8),
            bin_maxlen: usize::MAX,
            hex: HexArg::Bytes(&hex),
            hex_len: hex.len(),
            ignore: None,
            want_hex_end: true,
        };
        assert_same_and_ret(&format!("row21#{i}"), &c, -1);
        assert_eq!(c_hex_end_off(&c), nibbles - 1);
    }
    assert_did_work("errors_md_row_21_size_max_odd_input", __before, 200);
}

// ===========================================================================
// Row 22 — ignore == "" rejects every non-hex, non-NUL byte
// ===========================================================================

#[test]
fn errors_md_row_22_empty_ignore_rejects_non_nul() {
    let __before = comparisons();
    for b in 1u8..=255 {
        if is_hex_digit(b) {
            continue;
        }
        let hex = [b'a', b'a', b, b'b', b'b'];
        let c = call(8, &hex, Some(b""), true);
        assert_same_and_ret(&format!("row22/0x{b:02x}"), &c, 1);
        assert_eq!(c_hex_end_off(&c), 2);
        assert_same_and_ret(
            &format!("row22/0x{b:02x}/nullend"),
            &call(8, &hex, Some(b""), false),
            -1,
        );
    }
    assert_did_work("errors_md_row_22_empty_ignore_rejects_non_nul", __before, 466);
}

// ===========================================================================
// Row 23 — ignore entries that are hex digits are inert
// ===========================================================================

#[test]
fn errors_md_row_23_hex_digits_in_ignore_are_inert() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x117);
    for i in 0..300 {
        let bytes = rng.range(1, 16);
        let hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        // ignore consisting only of hex digits must behave exactly like "".
        let with_digits = call(bytes, &hex, Some(b"0123456789abcdefABCDEF"), true);
        let with_empty = call(bytes, &hex, Some(b""), true);
        assert_same_and_ret(&format!("row23/digits#{i}"), &with_digits, bytes as c_int);
        assert_same_and_ret(&format!("row23/empty#{i}"), &with_empty, bytes as c_int);
        assert_eq!(c_hex_end_off(&with_digits), c_hex_end_off(&with_empty));
    }
    assert_did_work("errors_md_row_23_hex_digits_in_ignore_are_inert", __before, 600);
}

// ===========================================================================
// Row 24 — input consists solely of ignorable bytes, hex_end_p set
// ===========================================================================

#[test]
fn errors_md_row_24_all_ignorable_with_hex_end() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x118);
    for i in 0..300 {
        let n = rng.range(1, 24);
        let hex: Vec<u8> = (0..n).map(|_| *rng.pick(b": -\t")).collect();
        let c = call(rng.range(0, 8), &hex, Some(b": -\t"), true);
        assert_same_and_ret(&format!("row24#{i}"), &c, 0);
        assert_eq!(
            c_hex_end_off(&c),
            hex.len(),
            "row24: all bytes consumed => *hex_end_p == &hex[hex_len]"
        );
    }
    assert_did_work("errors_md_row_24_all_ignorable_with_hex_end", __before, 300);
}

// ===========================================================================
// Row 25 — input consists solely of ignorable bytes, hex_end_p == NULL
// ===========================================================================

#[test]
fn errors_md_row_25_all_ignorable_null_hex_end() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x119);
    for i in 0..300 {
        let n = rng.range(1, 24);
        let hex: Vec<u8> = (0..n).map(|_| *rng.pick(b": -\t")).collect();
        // hex_pos reaches hex_len, so line 52 does NOT fire: result is 0, not -1.
        assert_same_and_ret(
            &format!("row25#{i}"),
            &call(rng.range(0, 8), &hex, Some(b": -\t"), false),
            0,
        );
    }
    assert_did_work("errors_md_row_25_all_ignorable_null_hex_end", __before, 300);
}

// ===========================================================================
// Generic FFI-boundary boundaries (task-mandated, beyond the table)
// ===========================================================================

/// Every byte value 0x00..=0xFF in every position of a 1..3-byte input, under
/// all four (ignore, hex_end_p) modes and bin_maxlen 0..=2. This is the
/// "out-of-range value crossing the FFI boundary" sweep: the C API has no enum
/// parameter, so the only value domain a caller can put out of range is the
/// input byte, and it is covered exhaustively here.
#[test]
fn boundary_exhaustive_byte_domain_all_modes() {
    let __before = comparisons();
    let ignores: [Option<&[u8]>; 4] = [None, Some(b""), Some(b": -"), Some(&[0u8])];
    for b in 0u8..=255 {
        for pos in 0..3usize {
            let mut hex = vec![b'a', b'a', b'a'];
            hex[pos] = b;
            for ig in ignores {
                for want_end in [false, true] {
                    for maxlen in 0..=2usize {
                        let c = Call {
                            bin: BinArg::Buf(maxlen + 8),
                            bin_maxlen: maxlen,
                            hex: HexArg::Bytes(&hex),
                            hex_len: 3,
                            ignore: ig,
                            want_hex_end: want_end,
                        };
                        assert_same(&format!("boundary/0x{b:02x}@{pos}"), &c);
                    }
                }
            }
        }
    }
    assert_did_work("boundary_exhaustive_byte_domain_all_modes", __before, 18432);
}

/// `bin_maxlen` and `hex_len` one step past every interesting boundary.
#[test]
fn boundary_off_by_one_lengths() {
    let __before = comparisons();
    let mut rng = Rng::new(SEED ^ 0x201);
    for i in 0..500 {
        let bytes = rng.range(1, 16);
        let hex = rand_hex(&mut rng, 2 * bytes, MIXED);
        for maxlen in [
            0,
            bytes.saturating_sub(1),
            bytes,
            bytes + 1,
            usize::MAX - 1,
            usize::MAX,
        ] {
            // Keep the real allocation adequate; the callee never writes more
            // than `bytes` bytes.
            let c = Call {
                bin: BinArg::Buf(bytes + 8),
                bin_maxlen: maxlen,
                hex: HexArg::Bytes(&hex),
                hex_len: hex.len(),
                ignore: None,
                want_hex_end: true,
            };
            assert_same(&format!("boundary/off-by-one#{i}/max={maxlen}"), &c);
        }
        for hex_len in 0..=hex.len() {
            let c = Call {
                bin: BinArg::Buf(bytes + 8),
                bin_maxlen: bytes,
                hex: HexArg::Bytes(&hex),
                hex_len,
                ignore: None,
                want_hex_end: true,
            };
            assert_same(&format!("boundary/hexlen#{i}/{hex_len}"), &c);
        }
    }
    assert_did_work("boundary_off_by_one_lengths", __before, 3000);
}

/// All NULL-pointer combinations the C provably never dereferences.
#[test]
fn boundary_null_pointer_matrix() {
    let __before = comparisons();
    for bin_null in [false, true] {
        for hex_null in [false, true] {
            for ig_null in [false, true] {
                for end_null in [false, true] {
                    let hex_len = if hex_null { 0 } else { 4 };
                    let bin_maxlen = if bin_null { 0 } else { 4 };
                    let c = Call {
                        bin: if bin_null {
                            BinArg::Null
                        } else {
                            BinArg::Buf(12)
                        },
                        bin_maxlen,
                        hex: if hex_null {
                            HexArg::Null
                        } else {
                            HexArg::Bytes(b"a1B2")
                        },
                        hex_len,
                        ignore: if ig_null { None } else { Some(b": ") },
                        want_hex_end: !end_null,
                    };
                    assert_same(
                        &format!("null-matrix/bin={bin_null},hex={hex_null},ig={ig_null},end={end_null}"),
                        &c,
                    );
                }
            }
        }
    }
    assert_did_work("boundary_null_pointer_matrix", __before, 16);
}
