//! Phase B + Phase C for the three stateless leaf entry points, driven directly
//! through their `.so` exports (not via the `findrep` convenience wrapper).
//!
//! Covers CONFIGS.md rows 1–7 (`validate_and_normalize`), 20–25
//! (`process_octal_string`), 26–33 (`find_and_replace_char`) and ERRORS.md rows
//! 5–17, 20, 21.

mod common;
use common::*;

use std::ffi::{c_char, c_int};

// ===========================================================================
// validate_and_normalize
// ===========================================================================

#[track_caller]
fn check_validate(p: &Pair, v: c_int) {
    unsafe {
        let c = (p.c.validate_and_normalize)(v);
        let r = (p.r.validate_and_normalize)(v);
        assert_eq!(c, r, "validate_and_normalize({v}): C={c} Rust={r}");
    }
}

/// CONFIGS #1 — identity path for 0 (ERRORS #5: 0 is NOT raised to 64).
#[test]
fn cfg01_validate_zero() {
    let p = fresh_pair();
    check_validate(&p, 0);
    unsafe { assert_eq!((p.c.validate_and_normalize)(0), 0) };
}

/// CONFIGS #2 / ERRORS #7 — `1..=63` clamps up to `0100` = 64.
#[test]
fn cfg02_validate_clamps_up_below_lower_threshold() {
    let p = fresh_pair();
    for v in 1..=63 {
        check_validate(&p, v);
        unsafe { assert_eq!((p.c.validate_and_normalize)(v), 64, "C clamp for {v}") };
    }
    let mut rng = Rng::new(0x11);
    for _ in 0..2000 {
        check_validate(&p, rng.range_i32(1, 63));
    }
}

/// CONFIGS #3 — `64..=511` is the identity band.
#[test]
fn cfg03_validate_identity_band() {
    let p = fresh_pair();
    for v in 64..=511 {
        check_validate(&p, v);
        unsafe { assert_eq!((p.c.validate_and_normalize)(v), v, "C identity for {v}") };
    }
    let mut rng = Rng::new(0x12);
    for _ in 0..2000 {
        check_validate(&p, rng.range_i32(64, 511));
    }
}

/// CONFIGS #4 / ERRORS #8 — `> 0777` clamps down to 511.
#[test]
fn cfg04_validate_clamps_down_above_upper_threshold() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x13);
    for _ in 0..5000 {
        let v = rng.range_i32(512, i32::MAX);
        check_validate(&p, v);
        unsafe { assert_eq!((p.c.validate_and_normalize)(v), 511, "C clamp for {v}") };
    }
    check_validate(&p, i32::MAX);
}

/// CONFIGS #5 / ERRORS #6 — negatives are never clamped, including INT_MIN.
#[test]
fn cfg05_validate_negatives_are_identity() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x14);
    for _ in 0..5000 {
        let v = rng.range_i32(i32::MIN, -1);
        check_validate(&p, v);
        unsafe { assert_eq!((p.c.validate_and_normalize)(v), v, "C identity for {v}") };
    }
    check_validate(&p, i32::MIN);
}

/// CONFIGS #6 + ERRORS #9/#10/#11 — exact boundaries, `<` and `>` being strict.
#[test]
fn cfg06_validate_boundaries() {
    let p = fresh_pair();
    for &v in BOUNDARIES.iter() {
        check_validate(&p, v);
    }
    unsafe {
        // ERRORS #11: one step below / above the range.
        assert_eq!((p.c.validate_and_normalize)(63), 64);
        assert_eq!((p.c.validate_and_normalize)(512), 511);
        // ERRORS #9/#10: the boundaries themselves are untouched.
        assert_eq!((p.c.validate_and_normalize)(64), 64);
        assert_eq!((p.c.validate_and_normalize)(511), 511);
    }
}

/// CONFIGS #7 — full-range fuzz.
#[test]
fn cfg07_validate_full_range_fuzz() {
    let p = fresh_pair();
    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..20_000 {
        check_validate(&p, rng.next_i32());
        check_validate(&p, rng.interesting_i32());
    }
}

// ===========================================================================
// process_octal_string
// ===========================================================================

/// Runs both libraries into sentinel-filled 100-byte buffers and compares all 100
/// bytes, so a misplaced NUL or a byte written past the terminator is caught.
#[track_caller]
fn check_octal(p: &Pair, v: c_int) -> Vec<u8> {
    let mut cb = sentinel_buf();
    let mut rb = sentinel_buf();
    unsafe {
        (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, v);
        (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, v);
    }
    assert_bytes_eq(&cb, &rb, &format!("process_octal_string(buf, {v})"));
    cb
}

/// CONFIGS #20 — the literal `0` prefix plus `%o` of zero yields a double zero.
#[test]
fn cfg20_octal_zero() {
    let p = fresh_pair();
    let out = check_octal(&p, 0);
    let s = &out[..out.iter().position(|&b| b == 0).unwrap()];
    assert_eq!(s, b"Octal: 00, Decimal: 0", "got {:?}", show(s));
}

/// CONFIGS #21 — the exact value `findrep` passes internally (`0123` = 83).
#[test]
fn cfg21_octal_the_value_findrep_uses() {
    let p = fresh_pair();
    let out = check_octal(&p, 0o123);
    let s = &out[..out.iter().position(|&b| b == 0).unwrap()];
    assert_eq!(s, b"Octal: 0123, Decimal: 83", "got {:?}", show(s));
}

/// CONFIGS #22 — randomized positive values.
#[test]
fn cfg22_octal_random_positive() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x21);
    for _ in 0..2000 {
        check_octal(&p, rng.range_i32(0, i32::MAX));
    }
}

/// CONFIGS #23 / ERRORS #20 — negatives print `%o` as unsigned, `%d` as signed.
#[test]
fn cfg23_octal_negative_two_complement() {
    let p = fresh_pair();
    let out = check_octal(&p, -1);
    let s = &out[..out.iter().position(|&b| b == 0).unwrap()];
    assert_eq!(
        s,
        b"Octal: 037777777777, Decimal: -1",
        "got {:?}",
        show(s)
    );
    let mut rng = Rng::new(0x22);
    for _ in 0..2000 {
        check_octal(&p, rng.range_i32(i32::MIN, -1));
    }
}

/// CONFIGS #24 / ERRORS #21 — extremes and every constant the C uses.
#[test]
fn cfg24_octal_boundaries() {
    let p = fresh_pair();
    for &v in BOUNDARIES.iter() {
        check_octal(&p, v);
    }
    for v in [i32::MIN, i32::MAX, -1, 0, 1, 7, 8, 9, 0o100, 0o123, 0o150, 0o777] {
        check_octal(&p, v);
    }
    // Widest possible output still fits the C's `char buffer[50]`.
    let out = check_octal(&p, i32::MIN);
    let s = &out[..out.iter().position(|&b| b == 0).unwrap()];
    assert_eq!(
        s,
        b"Octal: 020000000000, Decimal: -2147483648",
        "got {:?}",
        show(s)
    );
    assert_eq!(s.len(), 41, "41 bytes + NUL must fit in char buffer[50]");
}

/// CONFIGS #25 — nothing beyond the terminator may be touched, and calling twice
/// into a dirty buffer must leave identical trailing garbage on both sides.
#[test]
fn cfg25_octal_exact_terminator_and_no_overwrite() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x23);
    for _ in 0..2000 {
        // First write a long value, then a short one into the same buffer: the C's
        // strcpy only writes strlen+1 bytes, so the tail of the long string survives.
        let long = rng.range_i32(i32::MIN, -1);
        let short = rng.range_i32(0, 9);
        let mut cb = sentinel_buf();
        let mut rb = sentinel_buf();
        unsafe {
            (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, long);
            (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, long);
            assert_bytes_eq(&cb, &rb, "octal long write");
            (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, short);
            (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, short);
        }
        assert_bytes_eq(
            &cb,
            &rb,
            &format!("octal short write {short} over long write {long}"),
        );
    }
}

// ===========================================================================
// find_and_replace_char
// ===========================================================================

#[track_caller]
fn check_replace(p: &Pair, s: &[u8], ch: c_int) -> Vec<u8> {
    let mut cb = cstr_buf(s);
    let mut rb = cstr_buf(s);
    unsafe {
        (p.c.find_and_replace_char)(cb.as_mut_ptr() as *mut c_char, ch);
        (p.r.find_and_replace_char)(rb.as_mut_ptr() as *mut c_char, ch);
    }
    assert_bytes_eq(
        &cb,
        &rb,
        &format!("find_and_replace_char({:?}, {ch})", show(s)),
    );
    cb
}

/// CONFIGS #26 — hit at index 0.
#[test]
fn cfg26_replace_hit_at_index_zero() {
    let p = fresh_pair();
    let out = check_replace(&p, b"Octal: 0123", b'O' as c_int);
    assert_eq!(&out[..11], b"Xctal: 0123");
}

/// CONFIGS #27 — hit in the middle and at the final character.
#[test]
fn cfg27_replace_hit_middle_and_last() {
    let p = fresh_pair();
    let out = check_replace(&p, b"abcdef", b'c' as c_int);
    assert_eq!(&out[..6], b"abXdef");
    let out = check_replace(&p, b"abcdef", b'f' as c_int);
    assert_eq!(&out[..6], b"abcdeX");
}

/// CONFIGS #28 / ERRORS #12 — miss leaves the buffer untouched.
#[test]
fn cfg28_replace_miss_is_noop() {
    let p = fresh_pair();
    let before = cstr_buf(b"abcdef");
    let after = check_replace(&p, b"abcdef", b'z' as c_int);
    assert_eq!(before, after, "miss must not modify the buffer");
}

/// CONFIGS #29 / ERRORS #13 — empty string: `strlen == 0`, so nothing is searched.
#[test]
fn cfg29_replace_empty_string() {
    let p = fresh_pair();
    let before = cstr_buf(b"");
    let after = check_replace(&p, b"", b'X' as c_int);
    assert_eq!(before, after, "empty string must stay untouched");
    // Even searching for the terminator itself must not write.
    let after = check_replace(&p, b"", 0);
    assert_eq!(before, after);
}

/// CONFIGS #30 / ERRORS #17 — only the FIRST occurrence is replaced.
#[test]
fn cfg30_replace_only_first_occurrence() {
    let p = fresh_pair();
    let out = check_replace(&p, b"aaaa", b'a' as c_int);
    assert_eq!(&out[..4], b"Xaaa");
    let out = check_replace(&p, b"banana", b'a' as c_int);
    assert_eq!(&out[..6], b"bXnana");
}

/// ERRORS #14 — searching for NUL: `memchr` is given only `strlen` bytes, so the
/// terminator is outside the searched range and is never hit.
#[test]
fn cfg31a_replace_search_for_nul_never_matches() {
    let p = fresh_pair();
    let before = cstr_buf(b"abcdef");
    let after = check_replace(&p, b"abcdef", 0);
    assert_eq!(before, after, "search_char == 0 must be a no-op");
    // Any multiple of 256 truncates to 0 as well.
    for ch in [0, 256, 512, -256, 65536, i32::MIN] {
        let after = check_replace(&p, b"abcdef", ch);
        assert_eq!(before, after, "search_char {ch} truncates to 0");
    }
}

/// CONFIGS #31 / ERRORS #15/#16 — out-of-`unsigned char`-range `search_char`.
/// `memchr` converts to `unsigned char`, so these ALIAS a real byte and DO match.
/// This is the "integer with no valid variant crosses the FFI boundary" class.
#[test]
fn cfg31b_replace_search_char_out_of_byte_range_aliases() {
    let p = fresh_pair();
    // 'A' == 0x41. Every value congruent to 0x41 mod 256 must behave like 'A'.
    for ch in [
        0x41,
        0x141,
        0x241,
        0xFF41u32 as i32,
        -191, // 0xFFFFFF41
        -65_215,
        0x7FFF_FF41,
    ] {
        let out = check_replace(&p, b"zzAzz", ch);
        assert_eq!(
            &out[..5],
            b"zzXzz",
            "search_char {ch} (low byte 0x41) must match 'A'"
        );
    }
    // And a value whose low byte is absent must miss on both sides.
    let before = cstr_buf(b"zzAzz");
    let after = check_replace(&p, b"zzAzz", 0x142);
    assert_eq!(before, after);
}

/// CONFIGS #33 — a string that already contains the replacement character.
#[test]
fn cfg33_replace_string_already_contains_x() {
    let p = fresh_pair();
    let out = check_replace(&p, b"XXabcXX", b'a' as c_int);
    assert_eq!(&out[..7], b"XXXbcXX");
    let out = check_replace(&p, b"XXabcXX", b'X' as c_int);
    assert_eq!(&out[..7], b"XXabcXX", "replacing 'X' with 'X' is idempotent");
}

/// CONFIGS #32 — randomized fuzz over byte strings (including high-bit bytes, which
/// exercise the `char` vs `unsigned char` signedness question) × random `search_char`.
#[test]
fn cfg32_replace_random_fuzz() {
    let p = fresh_pair();
    let mut rng = Rng::new(0x5EED_5EED);
    for _ in 0..5000 {
        let len = rng.below(40) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            // 1..=255: no embedded NUL, but high-bit bytes included on purpose.
            s.push(rng.range_i32(1, 255) as u8);
        }
        let ch = match rng.below(4) {
            // Bias toward characters actually present so hits are common.
            0 if !s.is_empty() => *rng.pick(&s) as c_int,
            1 => rng.range_i32(0, 255),
            2 => rng.next_i32(),
            _ => rng.range_i32(-1000, 1000),
        };
        check_replace(&p, &s, ch);
    }
}

/// Fuzz specifically over high-bit bytes: if either side treated `char` as signed
/// when comparing, `0x80..=0xFF` would diverge.
#[test]
fn cfg32b_replace_high_bit_bytes() {
    let p = fresh_pair();
    for b in 0x80u8..=0xFF {
        let s = [b'a', b, b'z'];
        let out = check_replace(&p, &s, b as c_int);
        assert_eq!(&out[..3], &[b'a', b'X', b'z'], "byte 0x{b:02x} must match");
        // Also as a sign-extended negative int with the same low byte.
        let neg = (b as i32) - 256;
        let out = check_replace(&p, &s, neg);
        assert_eq!(&out[..3], &[b'a', b'X', b'z'], "int {neg} must match 0x{b:02x}");
    }
}

// ===========================================================================
// Generic API boundaries: buffer capacity extremes
// ===========================================================================

/// Longest string that fits `char message[100]`: 99 bytes + NUL. Exercises `strlen`
/// and the `memchr` length at full capacity, with the match at the very last byte.
#[test]
fn boundary_replace_full_capacity_buffer() {
    let p = fresh_pair();
    let mut s = vec![b'a'; 99];
    let out = check_replace(&p, &s, b'a' as c_int);
    assert_eq!(out[0], b'X', "first byte replaced");
    assert_eq!(out[99], 0, "terminator must stay at index 99");

    // Match at the final character only.
    s[98] = b'q';
    let out = check_replace(&p, &s, b'q' as c_int);
    assert_eq!(out[98], b'X', "last character must be reachable");
    assert_eq!(out[99], 0);

    // Miss over a full-capacity buffer.
    let before = cstr_buf(&s);
    let after = check_replace(&p, &s, b'Z' as c_int);
    assert_eq!(before, after);

    // Every length from 0 to 99, with the match at each possible position.
    for len in 0..=99usize {
        let mut v = vec![b'.'; len];
        if len > 0 {
            for pos in [0usize, len / 2, len - 1] {
                v.iter_mut().for_each(|c| *c = b'.');
                v[pos] = b'!';
                let out = check_replace(&p, &v, b'!' as c_int);
                assert_eq!(out[pos], b'X', "len {len} pos {pos}");
            }
        } else {
            check_replace(&p, &v, b'!' as c_int);
        }
    }
}

/// `process_octal_string` into a buffer exactly as large as the widest output
/// (41 bytes + NUL = 42). Anything wider would overrun the C's own `char buffer[50]`.
#[test]
fn boundary_octal_exact_capacity_destination() {
    let p = fresh_pair();
    for v in [i32::MIN, i32::MAX, -1, 0] {
        let mut cb = vec![SENTINEL; 42];
        let mut rb = vec![SENTINEL; 42];
        unsafe {
            (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, v);
            (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, v);
        }
        assert_bytes_eq(&cb, &rb, &format!("42-byte dest, octal_val {v}"));
        assert!(cb.contains(&0), "output must be NUL-terminated within 42 bytes");
    }
    // Every octal-digit-count boundary: 1, 8, 64, ... up to 2^30.
    let mut v: i64 = 1;
    while v <= i32::MAX as i64 {
        check_octal(&p, v as c_int);
        check_octal(&p, (v - 1) as c_int);
        check_octal(&p, -(v as i32));
        v *= 8;
    }
}

/// Exhaustive sweep of `validate_and_normalize` over a contiguous window that spans
/// both thresholds, leaving no off-by-one unchecked.
#[test]
fn boundary_validate_exhaustive_window() {
    let p = fresh_pair();
    for v in -1024..=1600 {
        check_validate(&p, v);
    }
    // And exhaustively near the type extremes.
    for v in (i32::MIN..i32::MIN + 512).chain(i32::MAX - 512..=i32::MAX) {
        check_validate(&p, v);
    }
}
