//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`
//! files, asserts they return the SAME sentinel (`-1` vs `>= 0`) *and* the same
//! side effects, and additionally pins the C-observed value so a "both wrong the
//! same way" regression cannot hide.

mod harness;
use harness::*;

use std::ffi::{c_char, c_int};

/// Call the C `.so` directly to record the ground-truth return value.
fn c_ret(case: &Case) -> c_int {
    let f = libs().c;
    let mut buf = vec![0u8; case.bin_maxlen.min(4096) + case.hex.len() + 8];
    if matches!(case.bin_arg, BinArg::AliasHex) {
        buf[..case.hex.len()].copy_from_slice(&case.hex);
    }
    let bin = match case.bin_arg {
        BinArg::Null => std::ptr::null_mut(),
        BinArg::Buffer | BinArg::AliasHex => buf.as_mut_ptr(),
    };
    let hex: *const c_char = match (case.bin_arg, case.hex_arg) {
        (_, HexArg::Null) => std::ptr::null(),
        (BinArg::AliasHex, HexArg::Buffer) => buf.as_ptr() as *const c_char,
        (_, HexArg::Buffer) => case.hex.as_ptr() as *const c_char,
    };
    let ig = case.ignore.as_ref().map(|s| {
        let mut v = s.clone();
        v.push(0);
        v
    });
    let ig_ptr: *const c_char = match &ig {
        None => std::ptr::null(),
        Some(v) => v.as_ptr() as *const c_char,
    };
    let mut slot: *const c_char = std::ptr::null();
    let end: *mut *const c_char = if case.want_hex_end {
        &mut slot
    } else {
        std::ptr::null_mut()
    };
    unsafe { f(bin, case.bin_maxlen, hex, case.hex_len, ig_ptr, end) }
}

/// Assert the two implementations agree AND that the C sentinel is the expected
/// one, so the row is genuinely testing the documented rejection.
#[track_caller]
fn expect(case: &Case, expected: c_int) {
    assert_same(case);
    let got = c_ret(case);
    assert_eq!(
        got, expected,
        "ERRORS.md row mis-specified: C returned {got}, table says {expected}, \
         case hex={:?} bin_maxlen={} ignore={:?} hex_end_p={}",
        String::from_utf8_lossy(&case.hex),
        case.bin_maxlen,
        case.ignore,
        case.want_hex_end
    );
}

// ---------------------------------------------------------------------------
// E1 — output overflow with bin_maxlen == 0 and at least one hex digit.
// ---------------------------------------------------------------------------
#[test]
fn e1_overflow_bin_maxlen_zero() {
    for want_end in [false, true] {
        expect(&Case::new(b"00", 0).hex_end(want_end), -1);
        expect(&Case::new(b"ff", 0).hex_end(want_end), -1);
        expect(&Case::new(b"0", 0).hex_end(want_end), -1);
        expect(&Case::new(b"deadbeef", 0).hex_end(want_end), -1);
        for ig in [None, Some(&b""[..]), Some(&b" "[..])] {
            expect(&Case::new(b"00", 0).ignore(ig).hex_end(want_end), -1);
        }
    }
    // Randomized: any non-empty run of hex digits with bin_maxlen == 0 fails.
    let mut rng = Rng::new(0xE1);
    for _ in 0..500 {
        let len = 1 + rng.below(32);
        let hex = random_from(&mut rng, MIXED, len);
        for want_end in [false, true] {
            expect(&Case::new(&hex, 0).hex_end(want_end), -1);
        }
    }
}

// ---------------------------------------------------------------------------
// E2 — mid-buffer overflow: bin_maxlen smaller than the pairs available.
//      The already-written bytes stay written; only the count is zeroed.
// ---------------------------------------------------------------------------
#[test]
fn e2_overflow_mid_buffer() {
    for want_end in [false, true] {
        expect(&Case::new(b"0011", 1).hex_end(want_end), -1);
        expect(&Case::new(b"00112233", 2).hex_end(want_end), -1);
        expect(&Case::new(b"aabbccdd", 3).hex_end(want_end), -1);
    }
    let mut rng = Rng::new(0xE2);
    for _ in 0..500 {
        let pairs = 2 + rng.below(16);
        let hex = random_from(&mut rng, MIXED, pairs * 2);
        let bml = 1 + rng.below(pairs - 1); // 1..pairs-1, strictly short
        for want_end in [false, true] {
            expect(&Case::new(&hex, bml).hex_end(want_end), -1);
        }
    }
}

// ---------------------------------------------------------------------------
// E3 — odd digit count: state != 0 at loop exit -> hex_pos-- and ret = -1.
// ---------------------------------------------------------------------------
#[test]
fn e3_odd_digit_count() {
    for want_end in [false, true] {
        expect(&Case::new(b"000", 2).hex_end(want_end), -1);
        expect(&Case::new(b"abcde", 4).hex_end(want_end), -1);
        expect(&Case::new(b"f", 1).hex_end(want_end), -1);
    }
    let mut rng = Rng::new(0xE3);
    for _ in 0..500 {
        let len = 2 * rng.below(16) + 1; // odd
        let hex = random_from(&mut rng, MIXED, len);
        let bml = len / 2 + 1 + rng.below(3); // generous, so only parity fails
        for want_end in [false, true] {
            expect(&Case::new(&hex, bml).hex_end(want_end), -1);
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — non-hex byte at an ODD nibble boundary: `ignore` is bypassed because
//      state != 0, the loop breaks, and the parity check then fails.
// ---------------------------------------------------------------------------
#[test]
fn e4_ignored_char_at_odd_boundary() {
    for want_end in [false, true] {
        expect(&Case::new(b"0 0", 4).ignore(Some(b" ")).hex_end(want_end), -1);
        // NB: the separator must sit at an ODD index. In "ab:cd" the ':' is at
        // index 2 (an even boundary) so it is ignored and the call SUCCEEDS —
        // "abc:de" puts it at index 3, which is the odd-boundary case.
        expect(&Case::new(b"abc:de", 4).ignore(Some(b":")).hex_end(want_end), -1);
        expect(&Case::new(b"ab:cd", 4).ignore(Some(b":")).hex_end(want_end), 2);
        expect(
            &Case::new(b"aabbc dd", 8).ignore(Some(b" ")).hex_end(want_end),
            -1,
        );
    }
    let mut rng = Rng::new(0xE4);
    for _ in 0..500 {
        let pairs = 1 + rng.below(8);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        let odd = 1 + 2 * rng.below(pairs);
        hex.insert(odd, *rng.pick(b": -"));
        for want_end in [false, true] {
            expect(
                &Case::new(&hex, pairs + 4).ignore(Some(b": -")).hex_end(want_end),
                -1,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — hex_end_p == NULL and hex_pos != hex_len because a non-hex char stopped
//      the scan. The very same input with hex_end_p != NULL SUCCEEDS.
// ---------------------------------------------------------------------------
#[test]
fn e5_unconsumed_input_without_hex_end_p() {
    expect(&Case::new(b"00zz", 4).hex_end(false), -1);
    // Contrast: with hex_end_p the same input is a success returning 1.
    expect(&Case::new(b"00zz", 4).hex_end(true), 1);

    expect(&Case::new(b"aabb!!", 8).hex_end(false), -1);
    expect(&Case::new(b"aabb!!", 8).hex_end(true), 2);

    let mut rng = Rng::new(0xE5);
    for _ in 0..500 {
        let pairs = rng.below(9);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        hex.extend_from_slice(b"zz");
        expect(&Case::new(&hex, pairs + 2).hex_end(false), -1);
        expect(&Case::new(&hex, pairs + 2).hex_end(true), pairs as c_int);
    }
}

// ---------------------------------------------------------------------------
// E6 — with ignore == NULL every separator is fatal when hex_end_p == NULL.
// ---------------------------------------------------------------------------
#[test]
fn e6_separator_fatal_when_ignore_null() {
    expect(&Case::new(b"00:11", 4).hex_end(false), -1);
    expect(&Case::new(b"00 11", 4).hex_end(false), -1);
    expect(&Case::new(b"aa-bb", 4).hex_end(false), -1);
    // With an ignore set covering the separator it succeeds instead.
    expect(&Case::new(b"00:11", 4).ignore(Some(b":")).hex_end(false), 2);

    let mut rng = Rng::new(0xE6);
    for _ in 0..400 {
        let pairs = 1 + rng.below(8);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        let even = 2 * rng.below(pairs + 1); // any even index in 0..=hex.len()
        let sep = *rng.pick(b": -");
        hex.insert(even, sep);
        expect(&Case::new(&hex, pairs + 2).hex_end(false), -1);
    }
}

// ---------------------------------------------------------------------------
// E7 — non-hex char absent from a non-NULL ignore set, at an even boundary.
// ---------------------------------------------------------------------------
#[test]
fn e7_char_not_in_ignore_set() {
    expect(&Case::new(b"00!11", 4).ignore(Some(b" ")).hex_end(false), -1);
    expect(&Case::new(b"aabb?", 4).ignore(Some(b": -")).hex_end(false), -1);
    expect(&Case::new(b"00!11", 4).ignore(Some(b"")).hex_end(false), -1);

    let mut rng = Rng::new(0xE7);
    for _ in 0..400 {
        let pairs = 1 + rng.below(8);
        let mut hex = random_from(&mut rng, MIXED, pairs * 2);
        hex.insert(2 * rng.below(pairs + 1), b'!'); // '!' never in the set
        expect(
            &Case::new(&hex, pairs + 2).ignore(Some(b" :-")).hex_end(false),
            -1,
        );
    }
}

// ---------------------------------------------------------------------------
// E8 — overflow AND hex_end_p == NULL (site 1 + site 3): ret already -1.
// ---------------------------------------------------------------------------
#[test]
fn e8_overflow_and_no_hex_end_p() {
    expect(&Case::new(b"00112233", 1).hex_end(false), -1);
    expect(&Case::new(b"00", 0).hex_end(false), -1);
    let mut rng = Rng::new(0xE8);
    for _ in 0..400 {
        let pairs = 2 + rng.below(12);
        let hex = random_from(&mut rng, MIXED, pairs * 2);
        expect(&Case::new(&hex, rng.below(pairs)).hex_end(false), -1);
    }
}

// ---------------------------------------------------------------------------
// E9 — odd digit count AND hex_end_p == NULL (site 2 + site 3).
// ---------------------------------------------------------------------------
#[test]
fn e9_odd_count_and_no_hex_end_p() {
    expect(&Case::new(b"000", 2).hex_end(false), -1);
    expect(&Case::new(b"0", 8).hex_end(false), -1);
    let mut rng = Rng::new(0xE9);
    for _ in 0..400 {
        let len = 2 * rng.below(16) + 1;
        let hex = random_from(&mut rng, MIXED, len);
        expect(&Case::new(&hex, len).hex_end(false), -1);
    }
}

// ---------------------------------------------------------------------------
// E10 — leading non-hex byte, nothing consumed, hex_end_p == NULL.
// ---------------------------------------------------------------------------
#[test]
fn e10_leading_non_hex_nothing_consumed() {
    expect(&Case::new(b"zz", 4).hex_end(false), -1);
    expect(&Case::new(b"!!!!", 4).hex_end(false), -1);
    // With hex_end_p it is a success returning 0 bytes.
    expect(&Case::new(b"zz", 4).hex_end(true), 0);
    expect(&Case::new(b"!!!!", 4).hex_end(true), 0);
    for b in BOUNDARY {
        expect(&Case::new(&[*b, *b], 4).hex_end(false), -1);
        expect(&Case::new(&[*b, *b], 4).hex_end(true), 0);
    }
}

// ---------------------------------------------------------------------------
// E11 — the minimal odd case: a single hex digit.
// ---------------------------------------------------------------------------
#[test]
fn e11_single_hex_digit() {
    for d in MIXED {
        for want_end in [false, true] {
            for bml in [0usize, 1, 8] {
                expect(&Case::new(&[*d], bml).hex_end(want_end), -1);
            }
        }
    }
    // hex_len == 1 while the buffer holds more (length wins over content).
    expect(&Case::new(b"0011", 4).hex_len(1).hex_end(true), -1);
}

// ---------------------------------------------------------------------------
// B1..B11 — generic FFI boundary rows.
// ---------------------------------------------------------------------------

#[test]
fn b1_null_bin_with_zero_maxlen() {
    for want_end in [false, true] {
        expect(&Case::new(b"00", 0).bin_null().hex_end(want_end), -1);
        expect(&Case::new(b"deadbeef", 0).bin_null().hex_end(want_end), -1);
        for ig in [None, Some(&b""[..]), Some(&b" "[..])] {
            expect(&Case::new(b"aa", 0).bin_null().ignore(ig).hex_end(want_end), -1);
        }
    }
    // A NULL bin with zero maxlen and zero-length hex is still a success.
    expect(&Case::new(b"", 0).bin_null().hex_end(true), 0);
}

#[test]
fn b2_null_hex_zero_len() {
    expect(&Case::new(b"", 0).hex_null().hex_end(true), 0);
    expect(&Case::new(b"", 16).hex_null().hex_end(true), 0);
    for ig in [None, Some(&b""[..]), Some(&b" "[..])] {
        expect(&Case::new(b"", 8).hex_null().ignore(ig).hex_end(true), 0);
    }
}

#[test]
fn b3_everything_null_and_zero() {
    expect(
        &Case::new(b"", 0).hex_null().bin_null().hex_end(false),
        0,
    );
    expect(&Case::new(b"", 0).hex_null().bin_null().hex_end(true), 0);
}

#[test]
fn b4_bin_maxlen_size_max() {
    for want_end in [false, true] {
        expect(&Case::new(b"00", usize::MAX).hex_end(want_end), 1);
        expect(&Case::new(b"deadbeef", usize::MAX).hex_end(want_end), 4);
        // Odd count still fails even with an unbounded output buffer.
        expect(&Case::new(b"abc", usize::MAX).hex_end(want_end), -1);
    }
    // One step below SIZE_MAX too.
    expect(&Case::new(b"00", usize::MAX - 1).hex_end(true), 1);
}

#[test]
fn b5_zero_hex_len_with_non_empty_buffer() {
    for want_end in [false, true] {
        for bml in [0usize, 1, 8, usize::MAX] {
            expect(&Case::new(b"deadbeef", bml).hex_len(0).hex_end(want_end), 0);
        }
    }
}

#[test]
fn b6_empty_ignore_set() {
    // strchr("", c) matches only c == 0.
    expect(&Case::new(b"00 11", 4).ignore(Some(b"")).hex_end(true), 1);
    expect(&Case::new(b"00 11", 4).ignore(Some(b"")).hex_end(false), -1);
    expect(&Case::new(b"00\011", 4).ignore(Some(b"")).hex_end(true), 2);
    expect(&Case::new(b"00\011", 4).ignore(Some(b"")).hex_end(false), 2);
}

#[test]
fn b7_embedded_nul_ignored_when_ignore_non_null() {
    // The NUL terminator of the ignore set makes an embedded NUL "ignorable".
    expect(&Case::new(b"aa\0bb", 4).ignore(Some(b" ")).hex_end(true), 2);
    expect(&Case::new(b"aa\0bb", 4).ignore(Some(b" ")).hex_end(false), 2);
    expect(&Case::new(b"\0aabb", 4).ignore(Some(b"")).hex_end(false), 2);
    // ...but only at an even boundary.
    expect(&Case::new(b"a\0abb", 4).ignore(Some(b" ")).hex_end(false), -1);
}

#[test]
fn b8_embedded_nul_with_null_ignore() {
    expect(&Case::new(b"aa\0bb", 4).hex_end(true), 1);
    expect(&Case::new(b"aa\0bb", 4).hex_end(false), -1);
    expect(&Case::new(b"\0aabb", 4).hex_end(true), 0);
    expect(&Case::new(b"\0aabb", 4).hex_end(false), -1);
}

#[test]
fn b9_ignore_set_with_hex_digits_is_dead() {
    // Digits present in the ignore set are still decoded, never skipped.
    expect(&Case::new(b"0123", 2).ignore(Some(b"0123")).hex_end(true), 2);
    expect(&Case::new(b"0123", 2).ignore(Some(b"0123")).hex_end(false), 2);
    expect(
        &Case::new(b"aabb", 2)
            .ignore(Some(b"abcdefABCDEF0123456789"))
            .hex_end(false),
        2,
    );
    // A single digit in an ignore-everything set is still an odd-count error.
    expect(
        &Case::new(b"a", 2)
            .ignore(Some(b"abcdefABCDEF0123456789"))
            .hex_end(false),
        -1,
    );
}

#[test]
fn b10_one_step_past_each_class_boundary() {
    // '/' 0x2f, ':' 0x3a, '@' 0x40, 'G' 0x47, '`' 0x60, 'g' 0x67
    for b in BOUNDARY {
        for want_end in [false, true] {
            // Alone -> nothing consumed.
            expect(
                &Case::new(&[*b, *b], 4).hex_end(want_end),
                if want_end { 0 } else { -1 },
            );
            // After a full byte -> stops there.
            expect(
                &Case::new(&[b'a', b'a', *b], 4).hex_end(want_end),
                if want_end { 1 } else { -1 },
            );
            // At an odd boundary -> parity error regardless.
            expect(&Case::new(&[b'a', *b], 4).hex_end(want_end), -1);
        }
    }
    // And the accepted extremes really are accepted.
    for pair in [&b"00"[..], &b"99"[..], &b"AA"[..], &b"FF"[..], &b"aa"[..], &b"ff"[..]] {
        expect(&Case::new(pair, 1).hex_end(false), 1);
    }
}

/// B11 — full `0x00..=0xFF` sweep, the analogue of an out-of-range enum value.
/// Every byte pattern is a legal `char` at the FFI boundary and the C accepts
/// exactly `[0-9A-Fa-f]`; the Rust must agree on all 256.
#[test]
fn b11_full_byte_domain_sweep() {
    fn is_hex_digit(b: u8) -> bool {
        b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
    }
    for b in 0u8..=255 {
        // Sole character: a hex digit is an odd-count error (-1); any other byte
        // stops the scan at index 0, which yields 0 when hex_end_p reports the
        // stop position and -1 when it cannot (hex_pos 0 != hex_len 1).
        expect(
            &Case::new(&[b], 4).hex_end(true),
            if is_hex_digit(b) { -1 } else { 0 },
        );
        expect(&Case::new(&[b], 4).hex_end(false), -1);

        // Second character after a valid first nibble.
        let two = [b'a', b];
        expect(
            &Case::new(&two, 4).hex_end(true),
            if is_hex_digit(b) { 1 } else { -1 },
        );
        // Without hex_end_p: a hex digit completes the pair and fully consumes
        // the input (-> 1); anything else stops early (-> -1).
        expect(
            &Case::new(&two, 4).hex_end(false),
            if is_hex_digit(b) { 1 } else { -1 },
        );

        // Third character after a complete byte: even boundary, so `ignore` may
        // rescue it.
        let three = [b'a', b'a', b];
        expect(
            &Case::new(&three, 4).hex_end(true),
            if is_hex_digit(b) { -1 } else { 1 },
        );
        // With the byte itself in the ignore set (non-hex ones get skipped).
        let ig = [b, b'~'];
        let ig_slice: &[u8] = if b == 0 { &ig[1..] } else { &ig[..] };
        expect(
            &Case::new(&three, 4).ignore(Some(ig_slice)).hex_end(true),
            if is_hex_digit(b) { -1 } else { 1 },
        );
        // Full agreement is what matters for every remaining combination.
        for ig2 in [None, Some(&b""[..]), Some(&b" "[..])] {
            for want_end in [false, true] {
                for bml in [0usize, 1, 2, 4] {
                    assert_same(&Case::new(&three, bml).ignore(ig2).hex_end(want_end));
                    assert_same(&Case::new(&two, bml).ignore(ig2).hex_end(want_end));
                    assert_same(&Case::new(&[b], bml).ignore(ig2).hex_end(want_end));
                }
            }
        }
    }
}
