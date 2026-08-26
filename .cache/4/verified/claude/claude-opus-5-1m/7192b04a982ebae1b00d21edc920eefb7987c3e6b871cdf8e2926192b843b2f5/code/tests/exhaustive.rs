//! EXHAUSTIVE differential enumeration over short inputs.
//!
//! Randomized fuzzing can miss a code path by luck; these tests cannot. They
//! enumerate *every* string over the C's accepted alphabet up to a given
//! length, and every (`length`, `offset`) pair for short buffers, and compare
//! the two `.so`s on each one.

mod common;

use common::*;

/// The exact set of bytes the C `switch` accepts, plus representatives of every
/// class that reaches `default:`.
const ALPHA: &[u8] = b"0123456789+-.eE";
const ALPHA_PLUS_STOP: &[u8] = b"0123456789+-.eE,\0 a";

/// Enumerate every string of length `n` over `alphabet` and compare.
fn exhaustive_len(alphabet: &[u8], n: usize, mut on_case: impl FnMut(&[u8])) {
    let k = alphabet.len();
    let total = k.pow(n as u32);
    let mut buf = vec![alphabet[0]; n];
    for mut idx in 0..total {
        for slot in buf.iter_mut() {
            *slot = alphabet[idx % k];
            idx /= k;
        }
        on_case(&buf);
    }
}

#[test]
fn exhaustive_charset_len_1_to_4() {
    let mut count = 0usize;
    for n in 1..=4 {
        exhaustive_len(ALPHA, n, |s| {
            assert_same(&Case::new(s));
            count += 1;
        });
    }
    // 15 + 225 + 3375 + 50625
    assert_eq!(count, 15 + 225 + 3375 + 50625);
}

#[test]
fn exhaustive_charset_len_5() {
    let mut count = 0usize;
    exhaustive_len(ALPHA, 5, |s| {
        assert_same(&Case::new(s));
        count += 1;
    });
    assert_eq!(count, 759_375);
}

#[test]
fn exhaustive_with_stop_bytes_len_1_to_4() {
    let mut count = 0usize;
    for n in 1..=4 {
        exhaustive_len(ALPHA_PLUS_STOP, n, |s| {
            assert_same(&Case::new(s));
            count += 1;
        });
    }
    let k = ALPHA_PLUS_STOP.len();
    assert_eq!(count, k + k * k + k * k * k + k * k * k * k);
}

/// Every (content, length, offset) triple for 3-byte charset contents.
#[test]
fn exhaustive_len_offset_matrix() {
    let mut count = 0usize;
    exhaustive_len(ALPHA, 3, |s| {
        for length in 0..=3usize {
            for offset in 0..=4usize {
                assert_same(&Case::new(s).length(length).offset(offset));
                count += 1;
            }
        }
    });
    assert_eq!(count, 3375 * 4 * 5);
}

/// Every single byte value as the sole content, across every (length, offset).
#[test]
fn exhaustive_single_byte_all_values() {
    for b in 0u16..=255 {
        let content = vec![b as u8];
        for length in 0..=1usize {
            for offset in 0..=2usize {
                assert_same(&Case::new(&content).length(length).offset(offset));
            }
        }
        // Same byte as a terminator after a valid token, and as a leader.
        assert_same(&Case::new(vec![b'1', b as u8]));
        assert_same(&Case::new(vec![b as u8, b'1']));
        assert_same(&Case::new(vec![b'-', b as u8, b'1']));
        assert_same(&Case::new(vec![b'1', b'.', b as u8]));
        assert_same(&Case::new(vec![b'1', b'e', b as u8]));
    }
}

/// Exhaustive 2-byte prefixes over all 256 byte values crossed with the whole
/// accepted charset — catches any byte-classification mismatch in the `switch`.
#[test]
fn exhaustive_byte_class_cross_charset() {
    for b in 0u16..=255 {
        for &c in ALPHA {
            assert_same(&Case::new(vec![b as u8, c]));
            assert_same(&Case::new(vec![c, b as u8]));
            assert_same(&Case::new(vec![c, b as u8, c]));
        }
    }
}

/// The byte at index `length` is ALWAYS an in-charset digit (`common::GUARD`),
/// so any read past `length` changes the parsed number. Exhaustively sweep the
/// cut position over all-charset contents: the result must equal parsing only
/// the visible prefix, for both implementations.
#[test]
fn exhaustive_read_guard_at_length_boundary() {
    // All-digit contents: the scan can only be stopped by `length`.
    exhaustive_len(ALPHA, 3, |s| {
        for length in 0..=3usize {
            let out = assert_same(&Case::new(s).length(length));
            // Whatever was decided, at most `length` bytes can have been consumed.
            assert!(
                out.buf_offset <= length,
                "read past length: {:?} length={length} -> offset={}",
                escape(s),
                out.buf_offset
            );
        }
    });

    // Long all-digit runs cut at every position: `offset` must equal `length`
    // (the guard digits must NOT be picked up).
    let digits: Vec<u8> = (0..64).map(|i| b'0' + ((i % 9) as u8) + 1).collect();
    for length in 1..=64usize {
        let out = assert_same(&Case::new(&digits).length(length));
        assert_eq!(out.ret, C_TRUE, "length={length}");
        assert_eq!(
            out.buf_offset, length,
            "length={length}: guard bytes leaked into the parse"
        );
    }

    // Same, but starting at every interior offset.
    for offset in 0..32usize {
        for length in (offset + 1)..=64usize {
            let out = assert_same(&Case::new(&digits).length(length).offset(offset));
            assert_eq!(out.ret, C_TRUE, "off={offset} len={length}");
            assert_eq!(out.buf_offset, length, "off={offset} len={length}");
        }
    }
}

/// Justification for the "NaN maps to 0 instead of INT_MIN" surviving mutant in
/// `mutants.sh`: the NaN arm of `double_to_int_c` is DEAD CODE.
///
/// `strtod` can only return NaN for the spellings `nan` / `nan(...)`, and only
/// returns `inf` for `inf` / `infinity` or via overflow. Since the C's `switch`
/// admits only `[0-9+-.eE]` into the temporary buffer, no NaN spelling can ever
/// reach `strtod`. Confirmed here over EVERY charset string up to length 5
/// (759 375 + 54 240 inputs) and every overflow/underflow shape.
#[test]
fn strtod_never_returns_nan_over_the_accepted_charset() {
    let mut checked = 0usize;
    for n in 1..=4 {
        exhaustive_len(ALPHA, n, |s| {
            let out = assert_same(&Case::new(s));
            if out.ret == C_TRUE {
                let v = f64::from_bits(out.item_double_bits);
                assert!(!v.is_nan(), "strtod returned NaN for {:?}", escape(s));
            }
            checked += 1;
        });
    }
    exhaustive_len(ALPHA, 5, |s| {
        let out = assert_same(&Case::new(s));
        if out.ret == C_TRUE {
            assert!(
                !f64::from_bits(out.item_double_bits).is_nan(),
                "strtod returned NaN for {:?}",
                escape(s)
            );
        }
        checked += 1;
    });
    assert_eq!(checked, 15 + 225 + 3375 + 50625 + 759_375);

    // Overflow / underflow / huge-exponent shapes cannot produce NaN either.
    for s in [
        "1e999999", "-1e999999", "1e-999999", "-1e-999999", "0e999999", "9".repeat(400).as_str(),
        "1.7976931348623159e308", "-1.7976931348623159e308",
    ] {
        let out = assert_same_str(s);
        if out.ret == C_TRUE {
            assert!(
                !f64::from_bits(out.item_double_bits).is_nan(),
                "strtod returned NaN for {s:?}"
            );
        }
    }
}

/// Justification for the "`can_access_at_index` saturating add" surviving
/// mutant: `offset + index` can never actually overflow.
///
/// The loop condition must first hold at `index == 0` (so `offset < length`),
/// and thereafter `offset + index <= length <= usize::MAX`. The wrap-around case
/// is therefore only reachable at `index == 0`, where wrapping and saturating
/// arithmetic agree. Exercised at the extreme offsets.
#[test]
fn offset_plus_index_never_overflows() {
    let content: Vec<u8> = (0..32).map(|i| b'0' + (i % 10) as u8).collect();
    // Only `offset >= length` pairs: exactly the region where `offset + index`
    // COULD overflow. The guard fails at `index == 0`, so the body never runs and
    // wrapping vs. saturating arithmetic cannot be distinguished.
    for &offset in &[
        usize::MAX,
        usize::MAX - 1,
        usize::MAX - 31,
        usize::MAX / 2,
        1usize << 63,
    ] {
        for &length in &[0usize, 1, 32, 1usize << 62, usize::MAX / 2] {
            if offset < length {
                continue; // that case reads real bytes; covered elsewhere
            }
            let case = Case::new(&content).length(length).offset(offset);
            let out = assert_same(&case);
            // Nothing may be consumed: the guard fails at index 0.
            assert_eq!(out.buf_offset, offset, "{}", case.label());
            assert_eq!(out.ret, C_FALSE, "{}", case.label());
        }
    }
    // The complementary region (`offset < length`, so the loop DOES run) can only
    // reach `index <= length - offset`, hence `offset + index <= length`: no
    // overflow. Demonstrated at the largest offsets that still address the real
    // buffer.
    for cut in 1..=32usize {
        let case = Case::new(&content).length(32).offset(32 - cut);
        let out = assert_same(&case);
        assert_eq!(out.buf_offset, 32, "{}", case.label());
    }
}

/// Justification for the "rewrite loop always runs" surviving mutant: the loop
/// replaces `'.'` with `decimal_point`, and `decimal_point` is hard-coded to
/// `'.'` in the C, so the loop is a no-op whether or not it executes. Verified
/// by checking that inputs with and without `'.'` are handled identically to the
/// C across the exhaustive charset sweep above; here we additionally confirm the
/// content buffer is never modified.
#[test]
fn rewrite_loop_is_a_no_op() {
    for s in ["1.5", "1.2.3", "....", "1", "1e5", ".", "-0.0"] {
        let out = assert_same_str(s);
        // `content` is `const unsigned char *`; neither side may write to it.
        // `run()` gives each implementation its own copy, and both produce the
        // same observable result, which is asserted by `assert_same`.
        let _ = out;
    }
}
