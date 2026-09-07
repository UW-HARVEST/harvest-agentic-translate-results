//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH the C
//! `.so` and the Rust `.so`, and asserts they return the SAME specific
//! sentinel (`-1`) — not merely "both failed somehow" — plus identical
//! `*hex_end_p` and identical output-buffer contents.

mod harness;
use harness::*;

/// The only error sentinel `hex2bin` has.
const ERR: i32 = -1;

// --- ERRORS.md row 1 -----------------------------------------------------
// bin_maxlen == 0 with a valid leading hex digit -> line 31 rejects.
#[test]
fn error_row01_bin_maxlen_zero() {
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..ITERS {
        let n = rng.range(1, 16);
        let hex = rng.hex_digits(n);
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(8)
                .bin_maxlen(0)
                .no_ignore()
                .hex_end(he);
            assert_same_ret(&call, ERR, "ERRORS.md row 1: bin_maxlen == 0");
            if he {
                // *hex_end_p must point at hex[0].
                assert_eq!(
                    call.run_c().hex_end_off,
                    Some(0),
                    "row 1: *hex_end_p should be &hex[0]"
                );
            }
        }
    }
}

// --- ERRORS.md row 2 -----------------------------------------------------
// bin_maxlen < hex_len/2 with all-valid hex: partial write then -1, and
// *hex_end_p == &hex[2*bin_maxlen].
#[test]
fn error_row02_bin_maxlen_too_small() {
    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..ITERS {
        let nbytes = rng.range(2, 20);
        let maxlen = rng.below(nbytes); // strictly less than needed
        let hex = rng.hex_digits(nbytes * 2);
        let call = Call::new(hex.clone())
            .bin_cap(nbytes + 8)
            .bin_maxlen(maxlen)
            .poison(rng.u8())
            .no_ignore()
            .hex_end(true);
        assert_same_ret(&call, ERR, "ERRORS.md row 2: bin_maxlen too small");

        let c = call.run_c();
        assert_eq!(
            c.hex_end_off,
            Some((2 * maxlen) as isize),
            "row 2: *hex_end_p should be &hex[2*bin_maxlen]"
        );
        // The bytes written before the rejection are still there (C never
        // rolls back the buffer; only bin_pos is zeroed).
        assert_eq!(c.bin, call.run_rust().bin, "row 2: partial buffer differs");
    }
}

// Fixed hand-checked instance from ERRORS.md row 2.
#[test]
fn error_row02_literal_example() {
    let call = Call::new(b"aabb".to_vec())
        .bin_cap(8)
        .bin_maxlen(1)
        .poison(0x00)
        .no_ignore()
        .hex_end(true);
    assert_same_ret(&call, ERR, "ERRORS.md row 2: bin_maxlen=1 hex=aabb");
    let c = call.run_c();
    assert_eq!(c.hex_end_off, Some(2));
    assert_eq!(c.bin[0], 0xaa, "the first byte was written before rejection");
}

// --- ERRORS.md row 3 -----------------------------------------------------
// Odd consumed digit count -> state != 0 -> hex_pos-- and ret = -1.
#[test]
fn error_row03_odd_digit_count_rewind() {
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..ITERS {
        let ndigits = rng.range(0, 20) * 2 + 1; // 1, 3, 5, ...
        let hex = rng.hex_digits(ndigits);
        let call = Call::new(hex.clone())
            .bin_cap(ndigits + 8)
            .bin_maxlen(ndigits) // plenty of room
            .no_ignore()
            .hex_end(true);
        assert_same_ret(&call, ERR, "ERRORS.md row 3: odd digit count");
        // hex_pos reached ndigits, then line 44 rewound it by one.
        assert_eq!(
            call.run_c().hex_end_off,
            Some((ndigits - 1) as isize),
            "row 3: *hex_end_p must point back at the unpaired digit"
        );
    }
}

#[test]
fn error_row03_literal_example() {
    let call = Call::new(b"abc".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .no_ignore()
        .hex_end(true);
    assert_same_ret(&call, ERR, "ERRORS.md row 3: hex=abc");
    assert_eq!(call.run_c().hex_end_off, Some(2));
}

// --- ERRORS.md row 4 -----------------------------------------------------
// hex_end_p == NULL and the scan broke early on a non-hex, non-ignored char.
#[test]
fn error_row04_null_hex_end_with_trailing_garbage() {
    let mut rng = Rng::new(SEED ^ 104);
    const BAD: &[u8] = b"!!$%^&*()_+[]{}|;'\",<>?~ gGzZ/:@`";
    for _ in 0..ITERS {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        hex.push(*rng.pick(BAD));
        hex.push(*rng.pick(BAD));
        let call = Call::new(hex.clone())
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes + 4)
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&call, ERR, "ERRORS.md row 4: NULL hex_end_p + garbage");
    }
}

#[test]
fn error_row04_literal_example() {
    let call = Call::new(b"ab!!".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .no_ignore()
        .hex_end(false);
    assert_same_ret(&call, ERR, "ERRORS.md row 4: hex=ab!! hex_end_p=NULL");
    // With a non-NULL hex_end_p the very same input is NOT an error.
    let ok = call.hex_end(true);
    assert_same_ret(&ok, 1, "ERRORS.md row 4 contrast: hex_end_p != NULL -> ok");
    assert_eq!(ok.run_c().hex_end_off, Some(2));
}

// --- ERRORS.md row 5 -----------------------------------------------------
// hex_end_p == NULL and the scan broke on bin_pos >= bin_maxlen.
#[test]
fn error_row05_null_hex_end_and_out_of_room() {
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..ITERS {
        let nbytes = rng.range(2, 20);
        let maxlen = rng.below(nbytes);
        let hex = rng.hex_digits(nbytes * 2);
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            .bin_maxlen(maxlen)
            .poison(rng.u8())
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&call, ERR, "ERRORS.md row 5: NULL hex_end_p + no room");
    }
}

#[test]
fn error_row05_literal_example() {
    let call = Call::new(b"aabb".to_vec())
        .bin_cap(8)
        .bin_maxlen(1)
        .no_ignore()
        .hex_end(false);
    assert_same_ret(&call, ERR, "ERRORS.md row 5: bin_maxlen=1 hex=aabb no hex_end");
}

// --- ERRORS.md row 6 -----------------------------------------------------
// Odd digit count AND hex_end_p == NULL (rows 3 + 4 combined).
#[test]
fn error_row06_odd_count_and_null_hex_end() {
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..ITERS {
        let ndigits = rng.range(0, 20) * 2 + 1;
        let hex = rng.hex_digits(ndigits);
        let call = Call::new(hex)
            .bin_cap(ndigits + 8)
            .bin_maxlen(ndigits)
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&call, ERR, "ERRORS.md row 6: odd count + NULL hex_end_p");
    }
    let call = Call::new(b"abc".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .no_ignore()
        .hex_end(false);
    assert_same_ret(&call, ERR, "ERRORS.md row 6: hex=abc hex_end_p=NULL");
}

// --- ERRORS.md row 7 -----------------------------------------------------
// Trailing digit after a skipped separator => odd count => -1.
#[test]
fn error_row07_trailing_ignore_then_odd_digit() {
    let call = Call::new(b"ab c".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .ignore(b" ")
        .hex_end(true);
    assert_same_ret(&call, ERR, "ERRORS.md row 7: hex='ab c' ignore=' '");
    assert_eq!(
        call.run_c().hex_end_off,
        Some(3),
        "row 7: *hex_end_p should be &hex[3]"
    );

    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..ITERS {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        hex.push(b' ');
        hex.push(rng.hex_digit()); // unpaired
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(n)
                .ignore(b" ")
                .hex_end(he);
            assert_same_ret(&call, ERR, "ERRORS.md row 7 randomized");
        }
    }
}

// --- ERRORS.md row 8 -----------------------------------------------------
// Ignore char BETWEEN the two nibbles of a byte: `state != 0` suppresses the
// ignore branch, so the C breaks and then rewinds.
#[test]
fn error_row08_ignore_char_at_nibble_boundary() {
    let call = Call::new(b"a b".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .ignore(b" ")
        .hex_end(true);
    assert_same_ret(&call, ERR, "ERRORS.md row 8: hex='a b' ignore=' '");
    assert_eq!(
        call.run_c().hex_end_off,
        Some(0),
        "row 8: broke at index 1 with state!=0, then hex_pos-- -> 0"
    );

    let mut rng = Rng::new(SEED ^ 108);
    const SETS: &[&[u8]] = &[b" ", b":", b" :\n\t-"];
    for _ in 0..ITERS {
        let nbytes = rng.range(1, 12);
        let set = *rng.pick(SETS);
        let mut hex = rng.hex_digits(nbytes * 2);
        let idx = rng.below(nbytes) * 2 + 1; // odd index => state != 0
        let sep = *rng.pick(set);
        hex.insert(idx, sep);
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(n + 8)
                .bin_maxlen(n)
                .ignore(set)
                .hex_end(he);
            assert_same_ret(&call, ERR, "ERRORS.md row 8 randomized");
        }
    }
}

// --- ERRORS.md row 9 -----------------------------------------------------
// High bytes 0x80..0xFF as the first char, hex_end_p == NULL.
#[test]
fn error_row09_high_byte_first_char() {
    for c in 0x80u16..=0xFF {
        let call = Call::new(vec![c as u8, b'a', b'b'])
            .bin_cap(8)
            .bin_maxlen(8)
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&call, ERR, &format!("ERRORS.md row 9: high byte {c:#04x}"));

        // Contrast: with hex_end_p non-NULL it decodes zero bytes, no error.
        let ok = call.hex_end(true);
        assert_same_ret(&ok, 0, &format!("ERRORS.md row 9 contrast {c:#04x}"));
        assert_eq!(ok.run_c().hex_end_off, Some(0));
    }
}

// --- ERRORS.md row 10 ----------------------------------------------------
// Boundary invalid chars one step past each valid range.
#[test]
fn error_row10_boundary_invalid_chars() {
    // '/' 0x2F, ':' 0x3A, '@' 0x40, 'G' 0x47, '`' 0x60, 'g' 0x67
    const BOUNDARY: &[u8] = &[0x2F, 0x3A, 0x40, 0x47, 0x60, 0x67];
    for &c in BOUNDARY {
        // As the sole char, hex_end_p == NULL -> error.
        let call = Call::new(vec![c])
            .bin_cap(8)
            .bin_maxlen(8)
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&call, ERR, &format!("ERRORS.md row 10: {c:#04x} alone"));

        // After an even number of digits, hex_end_p != NULL -> NOT an error.
        let ok = Call::new(vec![b'a', b'b', c, b'c', b'd'])
            .bin_cap(8)
            .bin_maxlen(8)
            .no_ignore()
            .hex_end(true);
        assert_same_ret(&ok, 1, &format!("ERRORS.md row 10 contrast: {c:#04x}"));
        assert_eq!(ok.run_c().hex_end_off, Some(2));

        // Same shape but hex_end_p == NULL -> error.
        let bad = ok.hex_end(false);
        assert_same_ret(&bad, ERR, &format!("ERRORS.md row 10: {c:#04x} no hex_end"));
    }

    // Also verify the two valid boundary chars either side are accepted.
    for &c in &[b'0', b'9', b'A', b'F', b'a', b'f'] {
        let ok = Call::new(vec![c, c])
            .bin_cap(8)
            .bin_maxlen(1)
            .no_ignore()
            .hex_end(false);
        assert_same_ret(&ok, 1, &format!("ERRORS.md row 10 valid: {c:#04x}"));
    }
}

// --- ERRORS.md row 11 ----------------------------------------------------
// Embedded NUL: skipped when ignore != NULL (strchr matches the terminator),
// rejected when ignore == NULL.
#[test]
fn error_row11_embedded_nul_strchr_terminator_quirk() {
    // ignore != NULL: NUL is skipped, so "ab\0cd" decodes to 2 bytes.
    for set in [&b""[..], &b" "[..], &b":"[..], &b" :\n\t-"[..]] {
        let call = Call::new(b"ab\0cd".to_vec())
            .bin_cap(8)
            .bin_maxlen(8)
            .ignore(set)
            .hex_end(true);
        assert_same_ret(&call, 2, "ERRORS.md row 11: NUL skipped when ignore != NULL");
        assert_eq!(call.run_c().hex_end_off, Some(5));

        // Even with hex_end_p == NULL it is still not an error, because the
        // whole input was consumed.
        let call = call.hex_end(false);
        assert_same_ret(&call, 2, "ERRORS.md row 11: NUL skipped, NULL hex_end_p");
    }

    // ignore == NULL: the NUL breaks the loop.
    let call = Call::new(b"ab\0cd".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .no_ignore()
        .hex_end(false);
    assert_same_ret(&call, ERR, "ERRORS.md row 11: NUL rejected when ignore == NULL");
    let ok = call.hex_end(true);
    assert_same_ret(&ok, 1, "ERRORS.md row 11: NUL stops the scan");
    assert_eq!(ok.run_c().hex_end_off, Some(2));

    // A NUL at an odd (nibble) index is NOT skipped even when ignore != NULL.
    let call = Call::new(b"a\0b".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .ignore(b" ")
        .hex_end(true);
    assert_same_ret(&call, ERR, "ERRORS.md row 11: NUL at nibble boundary");
}

// --- ERRORS.md row 12 ----------------------------------------------------
// ignore == "" matches only c == 0.
#[test]
fn error_row12_empty_ignore_set_matches_only_nul() {
    let mut rng = Rng::new(SEED ^ 112);
    const BAD: &[u8] = b" :-!gGzZ/@`";
    for _ in 0..ITERS {
        let nbytes = rng.range(1, 12);
        let mut hex = rng.hex_digits(nbytes * 2);
        hex.push(*rng.pick(BAD));
        let n = hex.len();
        let call = Call::new(hex)
            .bin_cap(n + 8)
            .bin_maxlen(n)
            .ignore(b"")
            .hex_end(false);
        assert_same_ret(&call, ERR, "ERRORS.md row 12: empty ignore set");
    }
    // ... but a NUL IS matched by the empty set.
    let call = Call::new(b"ab\0cd".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .ignore(b"")
        .hex_end(false);
    assert_same_ret(&call, 2, "ERRORS.md row 12: empty set still matches NUL");
}

// --- ERRORS.md row 13 ----------------------------------------------------
// bin == NULL with bin_maxlen == 0: rejected before any dereference.
#[test]
fn error_row13_bin_null_never_dereferenced() {
    for hex in [&b"a"[..], b"ab", b"abcd", b"00", b"FF"] {
        for he in [true, false] {
            let call = Call::new(hex.to_vec())
                .bin_cap(0)
                .bin_maxlen(0)
                .bin_null()
                .no_ignore()
                .hex_end(he);
            assert_same_ret(&call, ERR, "ERRORS.md row 13: bin=NULL, bin_maxlen=0");
        }
    }
    // bin == NULL but no digits at all -> not an error, still no dereference.
    for he in [true, false] {
        let call = Call::new(vec![])
            .bin_cap(0)
            .bin_maxlen(0)
            .bin_null()
            .no_ignore()
            .hex_end(he);
        assert_same_ret(&call, 0, "ERRORS.md row 13: bin=NULL, empty input");
    }
}

// --- ERRORS.md row 14 ----------------------------------------------------
// hex == NULL with hex_len == 0 -> returns 0, *hex_end_p == NULL.
#[test]
fn error_row14_hex_null_zero_len_is_not_an_error() {
    for maxlen in [0usize, 1, 8, usize::MAX] {
        for set in [None, Some(&b""[..]), Some(&b" :\n\t-"[..])] {
            for he in [true, false] {
                let mut call = Call::new(vec![])
                    .bin_cap(8)
                    .bin_maxlen(maxlen)
                    .hex_len(0)
                    .hex_null()
                    .hex_end(he);
                call = match set {
                    None => call.no_ignore(),
                    Some(s) => call.ignore(s),
                };
                assert_same_ret(&call, 0, "ERRORS.md row 14: hex=NULL, hex_len=0");
                if he {
                    assert_eq!(
                        call.run_c().hex_end_off,
                        Some(0),
                        "row 14: *hex_end_p == NULL + 0 == NULL"
                    );
                }
            }
        }
    }
}

// --- ERRORS.md row 15 ----------------------------------------------------
// Oversized hex_len with bin_maxlen == SIZE_MAX: no overflow check exists, so
// the length guard simply never fires. Combined with a terminating invalid
// byte so the scan stops inside the allocated buffer.
#[test]
fn error_row15_size_max_maxlen_never_rejects() {
    let mut rng = Rng::new(SEED ^ 115);
    for _ in 0..ITERS {
        let nbytes = rng.range(1, 24);
        let mut hex = rng.hex_digits(nbytes * 2);
        hex.push(b'!'); // stops the scan before hex_len is reached
        let real = hex.len();
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            // hex_len far beyond the digits, but the '!' breaks first.
            .hex_len(real + 8)
            .bin_maxlen(usize::MAX)
            .no_ignore()
            .hex_end(true);
        assert_same_ret(&call, nbytes as i32, "ERRORS.md row 15: SIZE_MAX maxlen");
        assert_eq!(call.run_c().hex_end_off, Some((real - 1) as isize));

        // With hex_end_p == NULL the un-consumed tail turns it into -1.
        let bad = call.hex_end(false);
        assert_same_ret(&bad, ERR, "ERRORS.md row 15: unconsumed tail, NULL hex_end_p");
    }
}

// --- ERRORS.md row 16 ----------------------------------------------------
// The API takes no enum; the int-typed value crossing the boundary is the
// return. Verify the full returned range agrees, including large bin_pos
// values, and that no out-of-range int is produced for reachable sizes.
#[test]
fn error_row16_return_value_range_agrees() {
    let mut rng = Rng::new(SEED ^ 116);
    // Sweep bin_pos across many magnitudes up to 32 KiB of decoded output.
    for nbytes in [0usize, 1, 2, 127, 128, 255, 256, 4095, 4096, 32767] {
        let hex = rng.hex_digits(nbytes * 2);
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            .bin_maxlen(nbytes)
            .no_ignore()
            .hex_end(true);
        assert_same_ret(
            &call,
            nbytes as i32,
            &format!("ERRORS.md row 16: return {nbytes}"),
        );
    }
    // And that -1 is returned as exactly -1 (0xFFFFFFFF), not 255 or similar.
    let call = Call::new(b"a".to_vec())
        .bin_cap(8)
        .bin_maxlen(8)
        .no_ignore()
        .hex_end(true);
    let c = call.run_c();
    let r = call.run_rust();
    assert_eq!(c.ret, -1i32);
    assert_eq!(r.ret, -1i32);
    assert_eq!(c.ret as u32, 0xFFFF_FFFF);
    assert_eq!(r.ret as u32, 0xFFFF_FFFF);
}

// --- Generic boundary coverage required by Phase C -----------------------

/// Values one step past every documented valid range, and out-of-range
/// "enum-like" integers pushed across the FFI boundary. `hex2bin` has no enum
/// parameter, so the closest analogue is the full 0..=255 byte domain of
/// `hex[i]` and the extreme `size_t` values for the two length parameters —
/// every one of them is a real input the C accepts and must be matched.
#[test]
fn generic_boundaries_all_byte_values_and_extreme_lengths() {
    let mut rng = Rng::new(SEED ^ 200);
    for c in 0u16..=255 {
        for &(hex_len, bin_maxlen) in &[
            (0usize, 0usize),
            (0, usize::MAX),
            (1, 0),
            (1, 1),
            (1, usize::MAX),
            (2, 0),
            (2, 1),
            (2, usize::MAX),
        ] {
            for set in [None, Some(&b""[..]), Some(&b" "[..])] {
                for he in [true, false] {
                    let mut call = Call::new(vec![c as u8, c as u8])
                        .bin_cap(8)
                        .bin_maxlen(bin_maxlen)
                        .hex_len(hex_len)
                        .hex_end(he);
                    call = match set {
                        None => call.no_ignore(),
                        Some(s) => call.ignore(s),
                    };
                    assert_same(
                        &call,
                        &format!("generic boundary c={c:#04x} len={hex_len} max={bin_maxlen}"),
                    );
                }
            }
        }
    }

    // usize::MAX - 1 / isize::MAX style extremes for bin_maxlen only (a huge
    // hex_len would read out of bounds in BOTH implementations, which is the
    // caller's contract violation, not a behaviour to compare).
    for &maxlen in &[usize::MAX, usize::MAX - 1, isize::MAX as usize, 1 << 40] {
        let hex = rng.hex_digits(16);
        let call = Call::new(hex)
            .bin_cap(16)
            .bin_maxlen(maxlen)
            .no_ignore()
            .hex_end(true);
        assert_same_ret(&call, 8, "generic boundary: extreme bin_maxlen");
    }
}

/// Null-pointer boundaries in every combination that the C is actually
/// allowed to be called with (i.e. where it provably does not dereference).
#[test]
fn generic_boundaries_null_pointers() {
    // bin=NULL & hex=NULL & ignore=NULL & hex_end_p=NULL, all lengths zero.
    let call = Call::new(vec![])
        .bin_cap(0)
        .bin_maxlen(0)
        .hex_len(0)
        .bin_null()
        .hex_null()
        .no_ignore()
        .hex_end(false);
    assert_same_ret(&call, 0, "generic: every pointer NULL");

    // Same but with hex_end_p non-NULL.
    let call = call.hex_end(true);
    assert_same_ret(&call, 0, "generic: all NULL except hex_end_p");
    assert_eq!(call.run_c().hex_end_off, Some(0));

    // bin=NULL, hex valid, bin_maxlen=0, digits present -> -1 before deref.
    let call = Call::new(b"deadbeef".to_vec())
        .bin_cap(0)
        .bin_maxlen(0)
        .bin_null()
        .ignore(b" ")
        .hex_end(true);
    assert_same_ret(&call, ERR, "generic: bin=NULL with digits");
}

/// Zero and oversized lengths.
#[test]
fn generic_boundaries_zero_and_oversized_lengths() {
    let mut rng = Rng::new(SEED ^ 201);
    for _ in 0..ITERS {
        let nd = rng.range(1, 24) * 2;
        let hex = rng.hex_digits(nd);
        let n = hex.len();
        for &hl in &[0usize, 1, n - 1, n] {
            for &ml in &[0usize, 1, n / 2 - 1, n / 2, n, usize::MAX] {
                for he in [true, false] {
                    let call = Call::new(hex.clone())
                        .bin_cap(n + 8)
                        .hex_len(hl)
                        .bin_maxlen(ml)
                        .no_ignore()
                        .hex_end(he);
                    assert_same(&call, "generic: zero/oversized lengths");
                }
            }
        }
    }
}
