//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each test constructs the exact rejecting
//! condition, calls BOTH shared objects, and asserts they return the same
//! sentinel/error value (and the same side effects), not merely "both failed".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — bin_maxlen == 0 with at least one valid digit
// ---------------------------------------------------------------------------
#[test]
fn err_01_bin_maxlen_zero() {
    let mut rng = Rng::new(0xe001);
    for _ in 0..2000 {
        let n = rng.range(1, 24);
        let mut hex: Vec<u8> = (0..n).map(|_| *rng.pick(MIXED)).collect();
        hex[0] = *rng.pick(MIXED); // guarantee a leading valid digit
        for &want_end in &[false, true] {
            for ign in [None, Some(Vec::new()), Some(SEPS.to_vec())] {
                let mut c = Case::new(hex.clone()).bin_maxlen(0).want_end(want_end);
                c.ignore = ign.clone();
                let out = check_and_get(&c);
                assert_eq!(out.ret, -1, "bin_maxlen==0 must fail: {:?}", c);
                if want_end {
                    assert_eq!(out.hex_end, Some(0), "must stop at the first digit");
                }
                // bin was not written (nothing fits).
                assert_eq!(out.bin, (0..c.bin_cap).map(fill).collect::<Vec<u8>>());
            }
        }
    }
}

fn fill(i: usize) -> u8 {
    (i as u8).wrapping_mul(31).wrapping_add(0xA5)
}

// ---------------------------------------------------------------------------
// Row 2 — buffer fills mid-stream
// ---------------------------------------------------------------------------
#[test]
fn err_02_bin_maxlen_truncates() {
    let mut rng = Rng::new(0xe002);
    for _ in 0..2000 {
        let bytes = rng.range(2, 32);
        let hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let cap = rng.range(1, bytes - 1);
        for &want_end in &[false, true] {
            let c = Case::new(hex.clone()).bin_maxlen(cap).want_end(want_end);
            let out = check_and_get(&c);
            assert_eq!(out.ret, -1, "truncating buffer must fail: {:?}", c);
            if want_end {
                assert_eq!(
                    out.hex_end,
                    Some((cap * 2) as isize),
                    "must stop at the digit that no longer fits"
                );
            }
            // The C keeps the bytes it already stored; the Rust must match
            // (compared byte-for-byte by `check`). Sanity-check the reference.
            for i in 0..cap {
                let expect = {
                    let hi = hexval(hex[2 * i]);
                    let lo = hexval(hex[2 * i + 1]);
                    (hi << 4) | lo
                };
                assert_eq!(out.bin[i], expect, "partial output byte {i}");
            }
        }
    }
}

fn hexval(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("not a hex digit"),
    }
}

// ---------------------------------------------------------------------------
// Row 3 — odd digit count (state != 0 at the end)
// ---------------------------------------------------------------------------
#[test]
fn err_03_odd_digit_count() {
    let mut rng = Rng::new(0xe003);
    for _ in 0..2000 {
        let n = 2 * rng.range(0, 16) + 1; // odd
        let hex: Vec<u8> = (0..n).map(|_| *rng.pick(MIXED)).collect();
        for &want_end in &[false, true] {
            for &bm in &[n / 2 + 1, n, n / 2] {
                let c = Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                let out = check_and_get(&c);
                assert_eq!(out.ret, -1, "odd digit count must fail: {:?}", c);
                if want_end && bm > n / 2 {
                    assert_eq!(
                        out.hex_end,
                        Some((n - 1) as isize),
                        "hex_pos is decremented to the unpaired digit"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — hex_end_p == NULL and input not fully consumed
// ---------------------------------------------------------------------------
#[test]
fn err_04_null_hex_end_unconsumed() {
    let mut rng = Rng::new(0xe004);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        // Non-hex byte at an even (byte-aligned) index, so the parse simply stops.
        let pos = 2 * rng.below(bytes);
        hex[pos] = b'z';
        let c = Case::new(hex).bin_maxlen(bytes).want_end(false);
        let out = check_and_get(&c);
        assert_eq!(out.ret, -1, "strict mode must reject unconsumed input");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — stop char is reported, not an error, when hex_end_p != NULL
// ---------------------------------------------------------------------------
#[test]
fn err_05_stop_char_reported_not_error() {
    let mut rng = Rng::new(0xe005);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = 2 * rng.range(0, bytes - 1); // even index
        hex[pos] = b'z';
        let c = Case::new(hex).bin_maxlen(bytes).want_end(true);
        let out = check_and_get(&c);
        assert_eq!(out.ret, (pos / 2) as i32, "partial count is returned");
        assert_eq!(out.hex_end, Some(pos as isize));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — char not in the ignore set
// ---------------------------------------------------------------------------
#[test]
fn err_06_char_not_in_ignore_set() {
    let mut rng = Rng::new(0xe006);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = 2 * rng.range(0, bytes - 1);
        hex[pos] = b'%'; // definitely not in SEPS
        let strict = Case::new(hex.clone())
            .ignore(SEPS)
            .bin_maxlen(bytes)
            .want_end(false);
        assert_eq!(check_and_get(&strict).ret, -1);
        let lenient = Case::new(hex).ignore(SEPS).bin_maxlen(bytes).want_end(true);
        let out = check_and_get(&lenient);
        assert_eq!(out.ret, (pos / 2) as i32);
        assert_eq!(out.hex_end, Some(pos as isize));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — separator inside a byte (state != 0 blocks the skip)
// ---------------------------------------------------------------------------
#[test]
fn err_07_separator_mid_byte() {
    let mut rng = Rng::new(0xe007);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = 2 * rng.below(bytes) + 1; // odd index
        hex.insert(pos, *rng.pick(SEPS));
        for &want_end in &[false, true] {
            let c = Case::new(hex.clone())
                .ignore(SEPS)
                .bin_maxlen(bytes + 1)
                .want_end(want_end);
            let out = check_and_get(&c);
            assert_eq!(out.ret, -1, "mid-byte separator must fail: {:?}", c);
            if want_end {
                assert_eq!(
                    out.hex_end,
                    Some((pos - 1) as isize),
                    "hex_pos-- points back at the unpaired digit"
                );
            }
        }
    }
    // Explicit minimal case from ERRORS.md.
    let out = check_and_get(&Case::new(b"a:b".to_vec()).ignore(&b":"[..]).bin_maxlen(4));
    assert_eq!(out.ret, -1);
    assert_eq!(out.hex_end, Some(0));
}

// ---------------------------------------------------------------------------
// Row 8 — ignore == "" behaves like NULL for non-NUL bytes
// ---------------------------------------------------------------------------
#[test]
fn err_08_empty_ignore_set() {
    let mut rng = Rng::new(0xe008);
    for _ in 0..3000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = rng.below(hex.len());
        let stop = loop {
            let b = rng.byte();
            if !is_hex_digit(b) && b != 0 {
                break b;
            }
        };
        hex[pos] = stop;
        for &want_end in &[false, true] {
            let with_empty = {
                let mut c = Case::new(hex.clone()).bin_maxlen(bytes).want_end(want_end);
                c.ignore = Some(Vec::new());
                c
            };
            let with_null = Case::new(hex.clone())
                .no_ignore()
                .bin_maxlen(bytes)
                .want_end(want_end);
            let a = check_and_get(&with_empty);
            let b = check_and_get(&with_null);
            assert_eq!(
                a, b,
                "ignore=\"\" must behave exactly like ignore=NULL for non-NUL bytes"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — odd digit count AND full buffer at the same time
// ---------------------------------------------------------------------------
#[test]
fn err_09_both_conditions() {
    let mut rng = Rng::new(0xe009);
    for _ in 0..1000 {
        let digits = 2 * rng.range(2, 16) + 1; // odd, >= 5
        let hex: Vec<u8> = (0..digits).map(|_| *rng.pick(MIXED)).collect();
        for cap in 1..=(digits / 2) {
            for &want_end in &[false, true] {
                let c = Case::new(hex.clone()).bin_maxlen(cap).want_end(want_end);
                let out = check_and_get(&c);
                assert_eq!(out.ret, -1);
                if want_end {
                    // The buffer-full break happens first, with state == 0.
                    assert_eq!(out.hex_end, Some((cap * 2) as isize));
                }
            }
        }
    }
    // ERRORS.md example: bin_maxlen=1, hex="abcde"
    let out = check_and_get(&Case::new(b"abcde".to_vec()).bin_maxlen(1));
    assert_eq!(out.ret, -1);
    assert_eq!(out.hex_end, Some(2));
}

// ---------------------------------------------------------------------------
// Row 10 — every failure returns exactly -1, never a partial count
// ---------------------------------------------------------------------------
#[test]
fn err_10_error_return_is_minus_one() {
    let mut rng = Rng::new(0xe010);
    let mut seen_failures = 0usize;
    for _ in 0..5000 {
        let n = rng.range(0, 32);
        let hex: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let bm = rng.range(0, n / 2 + 1);
        let want_end = rng.bool();
        let mut c = Case::new(hex).bin_maxlen(bm).want_end(want_end);
        c.ignore = if rng.bool() { Some(SEPS.to_vec()) } else { None };
        let out = check_and_get(&c);
        assert!(
            out.ret == -1 || out.ret >= 0,
            "only -1 or a byte count may be returned"
        );
        if out.ret < 0 {
            assert_eq!(out.ret, -1, "the only error value is -1");
            seen_failures += 1;
        }
    }
    assert!(seen_failures > 100, "expected the fuzz to hit error paths");
}

// ---------------------------------------------------------------------------
// Row 11 — leading invalid char, lenient mode => 0, not an error
// ---------------------------------------------------------------------------
#[test]
fn err_11_leading_invalid_char() {
    for b in 0u16..=255 {
        let b = b as u8;
        if is_hex_digit(b) {
            continue;
        }
        let mut hex = vec![b];
        hex.extend_from_slice(b"aabbcc");
        let c = Case::new(hex).no_ignore().bin_maxlen(4).want_end(true);
        let out = check_and_get(&c);
        assert_eq!(out.ret, 0, "byte {b:#04x} must stop the parse at offset 0");
        assert_eq!(out.hex_end, Some(0));
    }
}

// ---------------------------------------------------------------------------
// Row 12 — leading invalid char, strict mode => -1
// ---------------------------------------------------------------------------
#[test]
fn err_12_leading_invalid_char_null_end() {
    for b in 0u16..=255 {
        let b = b as u8;
        if is_hex_digit(b) {
            continue;
        }
        let mut hex = vec![b];
        hex.extend_from_slice(b"aabbcc");
        let c = Case::new(hex).no_ignore().bin_maxlen(4).want_end(false);
        assert_eq!(
            check_and_get(&c).ret,
            -1,
            "byte {b:#04x} unconsumed in strict mode must be -1"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — embedded NUL, ignore == NULL
// ---------------------------------------------------------------------------
#[test]
fn err_13_embedded_nul_no_ignore() {
    let mut rng = Rng::new(0xe013);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = rng.below(hex.len());
        hex[pos] = 0;
        for &want_end in &[false, true] {
            let c = Case::new(hex.clone())
                .no_ignore()
                .bin_maxlen(bytes)
                .want_end(want_end);
            let out = check_and_get(&c);
            if want_end {
                if pos % 2 == 0 {
                    assert_eq!(out.ret, (pos / 2) as i32);
                    assert_eq!(out.hex_end, Some(pos as isize));
                } else {
                    // unpaired nibble before the NUL
                    assert_eq!(out.ret, -1);
                    assert_eq!(out.hex_end, Some((pos - 1) as isize));
                }
            } else {
                assert_eq!(out.ret, -1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — embedded NUL with a non-NULL ignore set: strchr matches the
// terminator, so the NUL is skipped
// ---------------------------------------------------------------------------
#[test]
fn err_14_embedded_nul_with_ignore() {
    let mut rng = Rng::new(0xe014);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let pos = 2 * rng.below(bytes); // aligned => state == 0 => skipped
        hex.insert(pos, 0);
        for ign in [
            Some(Vec::new()),
            Some(SEPS.to_vec()),
            Some(b"xyz".to_vec()),
        ] {
            for &want_end in &[false, true] {
                let mut c = Case::new(hex.clone()).bin_maxlen(bytes).want_end(want_end);
                c.ignore = ign.clone();
                let out = check_and_get(&c);
                assert_eq!(
                    out.ret, bytes as i32,
                    "an aligned NUL is skipped when ignore != NULL: {:?}",
                    c
                );
            }
        }
        // Mid-byte NUL is NOT skipped (state != 0) -> error.
        let mut hex2: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        hex2.insert(2 * rng.below(bytes) + 1, 0);
        let mut c = Case::new(hex2).bin_maxlen(bytes + 1).want_end(true);
        c.ignore = Some(Vec::new());
        assert_eq!(check_and_get(&c).ret, -1);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — all 256 byte values as the stop char, full option matrix
// ---------------------------------------------------------------------------
#[test]
fn err_15_all_256_stop_bytes() {
    for b in 0u16..=255 {
        let b = b as u8;
        for pos in [0usize, 1, 2, 3] {
            let mut hex: Vec<u8> = b"0123456789abcdef".to_vec();
            hex[pos] = b;
            let variants: Vec<Option<Vec<u8>>> = vec![
                None,
                Some(Vec::new()),
                Some(if b == 0 { vec![b'#'] } else { vec![b] }),
                Some(vec![if b == b'q' { b'r' } else { b'q' }]),
            ];
            for ign in variants {
                for &want_end in &[false, true] {
                    for &bm in &[0usize, 1, 4, 8, 16] {
                        let mut c = Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                        c.ignore = ign.clone();
                        check(&c);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — one step past each accepted range + high-bit bytes
// ---------------------------------------------------------------------------
#[test]
fn err_16_range_boundary_chars() {
    let past: &[u8] = &[
        0x2f, 0x3a, 0x40, 0x47, 0x60, 0x67, 0x00, 0x7f, 0x80, 0xff, 0xc1, 0xe6, 0xa0,
    ];
    for &b in past {
        assert!(!is_hex_digit(b));
        for pos in 0..6usize {
            let mut hex: Vec<u8> = b"aabbcc".to_vec();
            hex[pos] = b;
            for ign in [None, Some(Vec::new()), Some(SEPS.to_vec())] {
                for &want_end in &[false, true] {
                    let mut c = Case::new(hex.clone()).bin_maxlen(3).want_end(want_end);
                    c.ignore = ign.clone();
                    let out = check_and_get(&c);
                    // With ignore==NULL (or a set without `b`), the byte always stops
                    // the parse; NUL with a non-NULL set is the documented quirk.
                    if c.ignore.is_none() {
                        if want_end {
                            let expect_end = if pos % 2 == 0 { pos } else { pos - 1 };
                            assert_eq!(out.hex_end, Some(expect_end as isize));
                        } else {
                            assert_eq!(out.ret, -1);
                        }
                    }
                }
            }
        }
    }
    // Every accepted byte really is accepted, and every other byte really is
    // rejected (cross-check of the classifier against both implementations).
    for b in 0u16..=255 {
        let b = b as u8;
        let c = Case::new(vec![b, b'0']).no_ignore().bin_maxlen(1);
        let out = check_and_get(&c);
        if is_hex_digit(b) {
            assert_eq!(out.ret, 1, "byte {b:#04x} should be a digit");
        } else {
            assert_eq!(out.ret, 0, "byte {b:#04x} should not be a digit");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — zero lengths and NULL pointers
// ---------------------------------------------------------------------------
#[test]
fn err_17_zero_length_and_null_ptrs() {
    for &bm in &[0usize, 1, 64, usize::MAX] {
        for &want_end in &[false, true] {
            for &null_bin in &[false, true] {
                for &null_hex in &[false, true] {
                    for ign in [None, Some(Vec::new()), Some(SEPS.to_vec())] {
                        let mut c = Case::new(Vec::<u8>::new())
                            .hex_len(0)
                            .bin_maxlen(bm)
                            .want_end(want_end)
                            .null_bin(null_bin)
                            .null_hex(null_hex);
                        c.ignore = ign.clone();
                        let out = check_and_get(&c);
                        assert_eq!(out.ret, 0, "zero length is success: {:?}", c);
                        if want_end {
                            assert_eq!(out.hex_end, Some(0));
                        }
                    }
                }
            }
        }
    }
    // NULL bin with a non-zero bin_maxlen is fine as long as nothing is stored:
    // a leading non-hex byte, or an unpaired single digit.
    for hex in [vec![b'z'], vec![b'a'], vec![b'!', b'a', b'b']] {
        for &want_end in &[false, true] {
            let c = Case::new(hex.clone())
                .bin_maxlen(16)
                .bin_cap(0)
                .null_bin(true)
                .want_end(want_end);
            check(&c);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — oversized bin_maxlen
// ---------------------------------------------------------------------------
#[test]
fn err_18_oversized_bin_maxlen() {
    let mut rng = Rng::new(0xe018);
    for &bm in &[usize::MAX, usize::MAX - 1, usize::MAX / 2, isize::MAX as usize] {
        for _ in 0..300 {
            let n = rng.range(0, 24);
            let hex: Vec<u8> = (0..n)
                .map(|_| {
                    if rng.below(5) == 0 {
                        rng.byte()
                    } else {
                        *rng.pick(MIXED)
                    }
                })
                .collect();
            let mut c = Case::new(hex).bin_maxlen(bm).want_end(rng.bool());
            c.ignore = if rng.bool() { Some(SEPS.to_vec()) } else { None };
            check(&c);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — oversized hex_len (no check exists in C); only probed where the
// parse provably stops on the first byte, so no out-of-bounds read occurs.
// ---------------------------------------------------------------------------
#[test]
fn err_19_oversized_hex_len() {
    let huge: &[usize] = &[usize::MAX, usize::MAX / 2, isize::MAX as usize, 1 << 40];
    for &hl in huge {
        // (a) first byte is not a hex digit and not in the ignore set -> break at 0
        for &want_end in &[false, true] {
            for stop in [b'z', b'!', 0x00u8, 0xff] {
                let c = Case::new(vec![stop, b'a', b'b'])
                    .hex_len(hl)
                    .bin_maxlen(8)
                    .bin_cap(8)
                    .no_ignore()
                    .want_end(want_end);
                let out = check_and_get(&c);
                if want_end {
                    assert_eq!(out.ret, 0);
                    assert_eq!(out.hex_end, Some(0));
                } else {
                    assert_eq!(out.ret, -1);
                }
            }
        }
        // (b) bin_maxlen == 0 and the first byte IS a digit -> break at 0 with -1
        for &want_end in &[false, true] {
            let c = Case::new(vec![b'a', b'b'])
                .hex_len(hl)
                .bin_maxlen(0)
                .bin_cap(8)
                .no_ignore()
                .want_end(want_end);
            let out = check_and_get(&c);
            assert_eq!(out.ret, -1);
            if want_end {
                assert_eq!(out.hex_end, Some(0));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — exotic but valid ignore sets
// ---------------------------------------------------------------------------
#[test]
fn err_20_exotic_ignore_sets() {
    let sets: &[&[u8]] = &[
        b"0123456789abcdefABCDEF", // every hex digit: never consulted for them
        b"a",
        b"\x80\x81\xfe\xff",
        b"\x01",
        b" ",
        b"::::",
        b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ];
    let mut rng = Rng::new(0xe020);
    for set in sets {
        for _ in 0..500 {
            let n = rng.range(0, 24);
            let hex: Vec<u8> = (0..n)
                .map(|_| match rng.below(4) {
                    0 => *rng.pick(set),
                    1 => rng.byte(),
                    _ => *rng.pick(MIXED),
                })
                .collect();
            let mut c = Case::new(hex)
                .bin_maxlen(rng.range(0, n / 2 + 2))
                .want_end(rng.bool());
            c.ignore = Some(set.to_vec());
            check(&c);
        }
        // Hex digits present in the ignore set must still be decoded.
        let c = Case::new(b"aabb".to_vec()).ignore(*set).bin_maxlen(2);
        let out = check_and_get(&c);
        assert_eq!(out.ret, 2, "ignore set must not swallow hex digits: {set:?}");
        assert_eq!(&out.bin[..2], &[0xaa, 0xbb]);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary probes that every C API shares.
//
// `hex2bin` takes no enum arguments, so there is no "out-of-range enum value"
// to smuggle across the boundary; the analogous smuggled values here are
// out-of-range *byte* values for `hex` (covered exhaustively by rows 15/16),
// out-of-range lengths (rows 18/19), and NULL pointers (row 17).
// ---------------------------------------------------------------------------
#[test]
fn err_generic_all_argument_extremes() {
    let lens: &[usize] = &[0, 1, 2, 3];
    let bins: &[usize] = &[0, 1, 2, usize::MAX];
    let mut rng = Rng::new(0xe0ff);
    for &hl in lens {
        for &bm in &bins[..] {
            for &want_end in &[false, true] {
                for ign in [None, Some(Vec::new()), Some(b":".to_vec())] {
                    for _ in 0..64 {
                        let hex: Vec<u8> = (0..hl).map(|_| rng.byte()).collect();
                        let mut c = Case::new(hex).bin_maxlen(bm).want_end(want_end);
                        c.ignore = ign.clone();
                        check(&c);
                    }
                }
            }
        }
    }
}
