//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row uses many randomized inputs
//! (fixed seed) and compares the C `.so` against the Rust `.so` through the
//! FFI boundary only.

mod common;

use common::*;

fn count_digits(hex: &[u8], len: usize) -> usize {
    hex[..len].iter().filter(|&&c| is_hex_digit(c)).count()
}

// ---------------------------------------------------------------------------
// Row 1 — strict mode (hex_end_p == NULL), no ignore set, exact bin_maxlen
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_strict_exact() {
    let mut rng = Rng::new(0x0101_0101);
    for _ in 0..2000 {
        let bytes = rng.range(0, 64);
        let hex = random_stream(&mut rng, bytes * 2, MIXED);
        let c = Case::new(hex).bin_maxlen(bytes).want_end(false);
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — lenient mode (hex_end_p != NULL), exact bin_maxlen
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_lenient_exact() {
    let mut rng = Rng::new(0x0202_0202);
    for _ in 0..2000 {
        let bytes = rng.range(0, 64);
        let hex = random_stream(&mut rng, bytes * 2, MIXED);
        let c = Case::new(hex).bin_maxlen(bytes).want_end(true);
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — bin_maxlen with slack
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_slack_bin() {
    let mut rng = Rng::new(0x0303_0303);
    for _ in 0..2000 {
        let bytes = rng.range(0, 48);
        let slack = rng.range(1, 16);
        let hex = random_stream(&mut rng, bytes * 2, MIXED);
        let c = Case::new(hex)
            .bin_maxlen(bytes + slack)
            .want_end(rng.bool());
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — bin_maxlen too small (partial write, then -1)
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_short_bin() {
    let mut rng = Rng::new(0x0404_0404);
    for _ in 0..2000 {
        let bytes = rng.range(1, 48);
        let short = rng.below(bytes); // 0 ..= bytes-1
        let hex = random_stream(&mut rng, bytes * 2, MIXED);
        let c = Case::new(hex).bin_maxlen(short).want_end(rng.bool());
        let out = check_and_get(&c);
        assert_eq!(out.ret, -1, "short buffer must fail: {:?}", c);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — empty input, all option combinations
// ---------------------------------------------------------------------------
#[test]
fn cfg_05_empty_input() {
    for &bin_maxlen in &[0usize, 1, 16, usize::MAX] {
        for &want_end in &[false, true] {
            for ign in [None, Some(&b""[..]), Some(&b" :"[..])] {
                for &null_bin in &[false, true] {
                    for &null_hex in &[false, true] {
                        let mut c = Case::new(Vec::<u8>::new())
                            .hex_len(0)
                            .bin_maxlen(bin_maxlen)
                            .want_end(want_end)
                            .null_bin(null_bin)
                            .null_hex(null_hex);
                        c.ignore = ign.map(|v| v.to_vec());
                        let out = check_and_get(&c);
                        assert_eq!(out.ret, 0, "empty input must return 0: {:?}", c);
                        if want_end {
                            assert_eq!(out.hex_end, Some(0));
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — hex_len == 1 and hex_len == 2, every digit value
// ---------------------------------------------------------------------------
#[test]
fn cfg_06_one_and_two_digits() {
    for &a in MIXED {
        for &want_end in &[false, true] {
            for &bin_maxlen in &[0usize, 1, 2] {
                check(&Case::new(vec![a]).bin_maxlen(bin_maxlen).want_end(want_end));
                for &b in MIXED {
                    check(
                        &Case::new(vec![a, b])
                            .bin_maxlen(bin_maxlen)
                            .want_end(want_end),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — odd digit counts
// ---------------------------------------------------------------------------
#[test]
fn cfg_07_odd_lengths() {
    let mut rng = Rng::new(0x0707_0707);
    for n in (1..=33).step_by(2) {
        for _ in 0..60 {
            let hex = random_stream(&mut rng, n, MIXED);
            for &want_end in &[false, true] {
                for &bm in &[n / 2, n / 2 + 1, n, 0] {
                    let c = Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                    check(&c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8/9/10/11 — alphabet / case coverage
// ---------------------------------------------------------------------------
fn alphabet_row(seed: u64, alphabet: &[u8]) {
    let mut rng = Rng::new(seed);
    for _ in 0..1500 {
        let bytes = rng.range(0, 40);
        let hex = random_stream(&mut rng, bytes * 2, alphabet);
        let bm = match rng.below(3) {
            0 => bytes,
            1 => bytes + rng.range(1, 4),
            _ => bytes.saturating_sub(rng.range(0, 2)),
        };
        check(&Case::new(hex).bin_maxlen(bm).want_end(rng.bool()));
    }
}

#[test]
fn cfg_08_lowercase_only() {
    alphabet_row(0x0808_0808, LOWER);
}

#[test]
fn cfg_09_uppercase_only() {
    alphabet_row(0x0909_0909, UPPER);
}

#[test]
fn cfg_10_mixed_case() {
    // Both nibbles of each byte drawn from different case classes.
    let mut rng = Rng::new(0x0a0a_0a0a);
    for _ in 0..1500 {
        let bytes = rng.range(1, 32);
        let mut hex = Vec::with_capacity(bytes * 2);
        for _ in 0..bytes {
            hex.push(*rng.pick(LETTERS_LOWER));
            hex.push(*rng.pick(LETTERS_UPPER));
        }
        check(&Case::new(hex.clone()).bin_maxlen(bytes));
        hex.reverse();
        check(&Case::new(hex).bin_maxlen(bytes).want_end(false));
    }
    // Exhaustive single byte over all 22 accepted digit characters.
    for &a in MIXED {
        for &b in MIXED {
            check(&Case::new(vec![a, b]).bin_maxlen(1));
        }
    }
}

#[test]
fn cfg_11_single_class_streams() {
    alphabet_row(0x0b0b_0b0b, DIGITS_ONLY);
    alphabet_row(0x0b0b_0b0c, LETTERS_LOWER);
    alphabet_row(0x0b0b_0b0d, LETTERS_UPPER);
}

// ---------------------------------------------------------------------------
// Row 12 — ignore == "" with non-hex bytes present
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_empty_ignore() {
    let mut rng = Rng::new(0x0c0c_0c0c);
    for _ in 0..3000 {
        let n = rng.range(1, 24);
        let hex: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(4) == 0 {
                    rng.byte()
                } else {
                    *rng.pick(MIXED)
                }
            })
            .collect();
        let digits = count_digits(&hex, n);
        let c = Case::new(hex)
            .ignore(&b""[..])
            .bin_maxlen(digits / 2 + rng.below(2))
            .want_end(rng.bool());
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — single ':' separator between every byte
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_colon_separated() {
    let mut rng = Rng::new(0x0d0d_0d0d);
    for _ in 0..1500 {
        let bytes = rng.range(1, 32);
        let mut hex = Vec::new();
        for i in 0..bytes {
            if i > 0 {
                hex.push(b':');
            }
            hex.push(*rng.pick(MIXED));
            hex.push(*rng.pick(MIXED));
        }
        for &want_end in &[false, true] {
            for &bm in &[bytes, bytes + 1, bytes - 1] {
                check(
                    &Case::new(hex.clone())
                        .ignore(&b":"[..])
                        .bin_maxlen(bm)
                        .want_end(want_end),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — multi-char ignore set, random runs of separators at aligned spots
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_multi_ignore_runs() {
    let mut rng = Rng::new(0x0e0e_0e0e);
    for _ in 0..2000 {
        let bytes = rng.range(1, 24);
        let mut hex = Vec::new();
        for i in 0..bytes {
            if i > 0 {
                for _ in 0..rng.range(1, 4) {
                    hex.push(*rng.pick(SEPS));
                }
            }
            hex.push(*rng.pick(MIXED));
            hex.push(*rng.pick(MIXED));
        }
        let c = Case::new(hex)
            .ignore(SEPS)
            .bin_maxlen(bytes + rng.below(2))
            .want_end(rng.bool());
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — leading separators
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_leading_separators() {
    let mut rng = Rng::new(0x0f0f_0f0f);
    for _ in 0..2000 {
        let bytes = rng.range(0, 16);
        let mut hex = Vec::new();
        for _ in 0..rng.range(1, 6) {
            hex.push(*rng.pick(SEPS));
        }
        for _ in 0..bytes * 2 {
            hex.push(*rng.pick(MIXED));
        }
        check(
            &Case::new(hex)
                .ignore(SEPS)
                .bin_maxlen(bytes)
                .want_end(rng.bool()),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 16 — trailing separators (fully consumed => success even in strict mode)
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_trailing_separators() {
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..2000 {
        let bytes = rng.range(0, 16);
        let mut hex = Vec::new();
        for _ in 0..bytes * 2 {
            hex.push(*rng.pick(MIXED));
        }
        for _ in 0..rng.range(1, 6) {
            hex.push(*rng.pick(SEPS));
        }
        let strict = Case::new(hex.clone())
            .ignore(SEPS)
            .bin_maxlen(bytes)
            .want_end(false);
        let out = check_and_get(&strict);
        assert_eq!(
            out.ret, bytes as i32,
            "trailing separators are consumed, so strict mode must succeed: {:?}",
            strict
        );
        check(&Case::new(hex).ignore(SEPS).bin_maxlen(bytes).want_end(true));
    }
}

// ---------------------------------------------------------------------------
// Row 17 — separator at a mid-byte position (state != 0 blocks the skip)
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_midbyte_separator() {
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..2000 {
        let bytes = rng.range(1, 16);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        // Insert a separator at an odd index => right after an unpaired nibble.
        let odd_positions: Vec<usize> = (0..hex.len()).filter(|i| i % 2 == 1).collect();
        let pos = *rng.pick(&odd_positions);
        hex.insert(pos, *rng.pick(SEPS));
        for &want_end in &[false, true] {
            let c = Case::new(hex.clone())
                .ignore(SEPS)
                .bin_maxlen(bytes + 1)
                .want_end(want_end);
            let out = check_and_get(&c);
            assert_eq!(out.ret, -1, "mid-byte separator must fail: {:?}", c);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — ignore set that also contains hex digits
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_ignore_contains_hex_digits() {
    let mut rng = Rng::new(0x1212_1212);
    let sets: [&[u8]; 4] = [b"0aA:", b"0123456789abcdefABCDEF", b"f", b"09:"];
    for _ in 0..2000 {
        let n = rng.range(0, 32);
        let hex: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(5) == 0 {
                    *rng.pick(SEPS)
                } else {
                    *rng.pick(MIXED)
                }
            })
            .collect();
        let set = *rng.pick(&sets);
        let digits = count_digits(&hex, n);
        check(
            &Case::new(hex)
                .ignore(set)
                .bin_maxlen(digits / 2 + rng.below(2))
                .want_end(rng.bool()),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 19 — ignore set of high-bit bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_high_bit_ignore() {
    let mut rng = Rng::new(0x1313_1313);
    for _ in 0..2000 {
        let n = rng.range(0, 32);
        let hex: Vec<u8> = (0..n)
            .map(|_| match rng.below(4) {
                0 => 0x80,
                1 => 0xff,
                2 => 0x80 | rng.byte(),
                _ => *rng.pick(MIXED),
            })
            .collect();
        let set: &[u8] = match rng.below(3) {
            0 => b"\x80\xff",
            1 => b"\x80",
            _ => b"\xff\xfe\x81\x80",
        };
        let digits = count_digits(&hex, n);
        check(
            &Case::new(hex)
                .ignore(set)
                .bin_maxlen(digits / 2 + rng.below(2))
                .want_end(rng.bool()),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 20 — every one of the 256 byte values as the embedded char
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_all_bytes_matrix() {
    let mut rng = Rng::new(0x1414_1414);
    for b in 0u16..=255 {
        let b = b as u8;
        for pos in [0usize, 1, 2, 3, 4] {
            let mut hex: Vec<u8> = (0..6).map(|_| *rng.pick(MIXED)).collect();
            hex.insert(pos, b);
            let ign_variants: Vec<Option<Vec<u8>>> = vec![
                None,
                Some(Vec::new()),
                Some(if b == 0 { vec![b':'] } else { vec![b] }),
                Some(vec![if b == b'z' { b'y' } else { b'z' }]),
                Some({
                    let mut v = SEPS.to_vec();
                    if b != 0 {
                        v.push(b);
                    }
                    v
                }),
            ];
            for ign in ign_variants {
                for &want_end in &[false, true] {
                    for &bm in &[0usize, 1, 3, 4] {
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
// Row 21 — boundary bytes at every position
// ---------------------------------------------------------------------------
const BOUNDARY: &[u8] = &[
    0x00, 0x2f, 0x3a, 0x40, 0x47, 0x60, 0x67, 0x7f, 0x80, 0xff, 0x0f, 0x10, 0x1f, 0x20, 0x29, 0x39,
    0x41, 0x46, 0x61, 0x66, 0xc1, 0xe1,
];

#[test]
fn cfg_21_boundary_bytes() {
    let base: &[u8] = b"0123456789abcdefABCDEF";
    for &b in BOUNDARY {
        for pos in 0..=8usize {
            let mut hex: Vec<u8> = base[..8].to_vec();
            hex.insert(pos, b);
            for ign in [
                None,
                Some(Vec::new()),
                Some(SEPS.to_vec()),
                Some(if b == 0 { vec![b'!'] } else { vec![b] }),
            ] {
                for &want_end in &[false, true] {
                    for &bm in &[0usize, 2, 4, 5] {
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
// Row 22 — embedded NUL + non-NULL ignore (strchr terminator quirk)
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_embedded_nul_quirk() {
    let mut rng = Rng::new(0x1616_1616);
    for _ in 0..2000 {
        let bytes = rng.range(1, 12);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        let nuls = rng.range(1, 3);
        for _ in 0..nuls {
            let pos = rng.range(0, hex.len());
            hex.insert(pos, 0);
        }
        for ign in [
            None,
            Some(Vec::new()),
            Some(SEPS.to_vec()),
            Some(b"abc".to_vec()),
        ] {
            for &want_end in &[false, true] {
                let mut c = Case::new(hex.clone())
                    .bin_maxlen(bytes + 1)
                    .want_end(want_end);
                c.ignore = ign.clone();
                check(&c);
            }
        }
    }
    // Deterministic aligned case: the NUL sits at an even index, so with a
    // non-NULL ignore set the whole string still decodes.
    let out = check_and_get(&Case::new(b"ab\0cd".to_vec()).ignore(&b""[..]).bin_maxlen(2));
    assert_eq!(out.ret, 2);
    assert_eq!(&out.bin[..2], &[0xab, 0xcd]);
    // ... and with ignore == NULL it stops at the NUL.
    let out = check_and_get(&Case::new(b"ab\0cd".to_vec()).no_ignore().bin_maxlen(2));
    assert_eq!(out.ret, 1);
    assert_eq!(out.hex_end, Some(2));
}

// ---------------------------------------------------------------------------
// Row 23 — hex_len shorter than the backing buffer
// ---------------------------------------------------------------------------
#[test]
fn cfg_23_partial_view() {
    let mut rng = Rng::new(0x1717_1717);
    for _ in 0..3000 {
        let total = rng.range(1, 40);
        let hex: Vec<u8> = (0..total)
            .map(|_| {
                if rng.below(6) == 0 {
                    rng.byte()
                } else {
                    *rng.pick(MIXED)
                }
            })
            .collect();
        let view = rng.range(0, total);
        let digits = count_digits(&hex, view);
        check(
            &Case::new(hex)
                .hex_len(view)
                .bin_maxlen(digits / 2 + rng.below(2))
                .want_end(rng.bool()),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 24 — in-place decoding (bin aliases hex)
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_in_place() {
    let mut rng = Rng::new(0x1818_1818);
    for _ in 0..2000 {
        let bytes = rng.range(1, 40);
        let mut hex: Vec<u8> = (0..bytes * 2).map(|_| *rng.pick(MIXED)).collect();
        if rng.bool() {
            // sprinkle byte-aligned separators
            let mut with_seps = Vec::new();
            for (i, ch) in hex.iter().enumerate() {
                if i % 2 == 0 && i > 0 && rng.bool() {
                    with_seps.push(*rng.pick(SEPS));
                }
                with_seps.push(*ch);
            }
            hex = with_seps;
        }
        let digits = count_digits(&hex, hex.len());
        for ign in [None, Some(SEPS.to_vec())] {
            for &want_end in &[false, true] {
                for &bm in &[digits / 2, digits / 2 + 1, digits / 4] {
                    let mut c = Case::new(hex.clone())
                        .bin_maxlen(bm)
                        .want_end(want_end)
                        .in_place(true);
                    c.ignore = ign.clone();
                    check(&c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — large inputs
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_large_input() {
    let mut rng = Rng::new(0x1919_1919);
    for _ in 0..60 {
        let bytes = rng.range(2048, 4096);
        let mut hex = Vec::with_capacity(bytes * 2 + 64);
        for i in 0..bytes {
            if i > 0 && rng.below(8) == 0 {
                hex.push(*rng.pick(SEPS));
            }
            hex.push(*rng.pick(MIXED));
            hex.push(*rng.pick(MIXED));
        }
        let digits = count_digits(&hex, hex.len());
        for ign in [None, Some(SEPS.to_vec())] {
            for &want_end in &[false, true] {
                for &bm in &[digits / 2, digits / 2 + 3, digits / 3, 0] {
                    let mut c = Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                    c.ignore = ign.clone();
                    check(&c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — huge bin_maxlen
// ---------------------------------------------------------------------------
#[test]
fn cfg_26_huge_bin_maxlen() {
    let mut rng = Rng::new(0x1a1a_1a1a);
    for &bm in &[usize::MAX, usize::MAX / 2, usize::MAX - 1, 1 << 40] {
        for _ in 0..200 {
            let n = rng.range(0, 32);
            let hex: Vec<u8> = (0..n)
                .map(|_| {
                    if rng.below(6) == 0 {
                        rng.byte()
                    } else {
                        *rng.pick(MIXED)
                    }
                })
                .collect();
            for ign in [None, Some(SEPS.to_vec())] {
                let mut c = Case::new(hex.clone()).bin_maxlen(bm).want_end(rng.bool());
                c.ignore = ign.clone();
                check(&c);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27 — control-character ignore set
// ---------------------------------------------------------------------------
#[test]
fn cfg_27_control_char_ignore() {
    let mut rng = Rng::new(0x1b1b_1b1b);
    let sets: [&[u8]; 3] = [b"\x01", b"\x01\x02\x1f", b"\x7f\x0b"];
    for _ in 0..2000 {
        let n = rng.range(1, 24);
        let hex: Vec<u8> = (0..n)
            .map(|_| match rng.below(4) {
                0 => 0x01,
                1 => *rng.pick(&[0x02u8, 0x1f, 0x7f, 0x0b]),
                _ => *rng.pick(MIXED),
            })
            .collect();
        let digits = count_digits(&hex, n);
        check(
            &Case::new(hex)
                .ignore(*rng.pick(&sets))
                .bin_maxlen(digits / 2 + rng.below(2))
                .want_end(rng.bool()),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 28 — round-trip property over random binary payloads
// ---------------------------------------------------------------------------
#[test]
fn cfg_28_round_trip() {
    let mut rng = Rng::new(0x1c1c_1c1c);
    for _ in 0..2000 {
        let n = rng.range(0, 64);
        let payload: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let upper = rng.bool();
        let table: &[u8] = if upper { UPPER } else { LOWER };
        let mut hex = Vec::with_capacity(n * 2);
        for &b in &payload {
            hex.push(table[(b >> 4) as usize]);
            hex.push(table[(b & 0xf) as usize]);
        }
        let c = Case::new(hex).bin_maxlen(n).want_end(rng.bool());
        let out = check_and_get(&c);
        assert_eq!(out.ret, n as i32);
        assert_eq!(&out.bin[..n], &payload[..], "round-trip mismatch");
    }
}

// ---------------------------------------------------------------------------
// Row 29 — full fuzz across all axes
// ---------------------------------------------------------------------------
#[test]
fn cfg_29_fuzz_all_axes() {
    let mut rng = Rng::new(0xdead_beef_0000_0029);
    for _ in 0..20_000 {
        let n = rng.range(0, 48);
        let hex: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let hex_len = if rng.below(8) == 0 { rng.range(0, n) } else { n };
        let bin_maxlen = match rng.below(8) {
            0 => 0,
            1 => usize::MAX,
            2 => rng.range(0, 2),
            _ => rng.range(0, n / 2 + 2),
        };
        let ignore: Option<Vec<u8>> = match rng.below(4) {
            0 => None,
            1 => Some(Vec::new()),
            2 => Some(SEPS.to_vec()),
            _ => {
                let k = rng.range(1, 6);
                Some(
                    (0..k)
                        .map(|_| {
                            let mut b = rng.byte();
                            if b == 0 {
                                b = 1;
                            }
                            b
                        })
                        .collect(),
                )
            }
        };
        let mut c = Case::new(hex)
            .hex_len(hex_len)
            .bin_maxlen(bin_maxlen)
            .want_end(rng.bool())
            .in_place(rng.below(6) == 0);
        c.ignore = ignore;
        if c.in_place {
            // bin aliases the hex buffer, which must be large enough.
            c.bin_cap = std::cmp::max(c.bin_cap, 8);
        }
        check(&c);
    }
}

// ---------------------------------------------------------------------------
// Row 30 — fuzz biased towards valid streams (deep decode paths)
// ---------------------------------------------------------------------------
#[test]
fn cfg_30_fuzz_mostly_valid() {
    let mut rng = Rng::new(0xdead_beef_0000_0030);
    for _ in 0..20_000 {
        let n = rng.range(0, 64);
        let hex: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(10) == 0 {
                    *rng.pick(SEPS)
                } else {
                    *rng.pick(MIXED)
                }
            })
            .collect();
        let digits = count_digits(&hex, n);
        let bin_maxlen = match rng.below(6) {
            0 => digits / 2,
            1 => digits / 2 + 1,
            2 => digits / 2 + rng.range(1, 8),
            3 => digits / 4,
            4 => 0,
            _ => rng.range(0, digits + 1),
        };
        let ignore: Option<Vec<u8>> = match rng.below(4) {
            0 => None,
            1 => Some(SEPS.to_vec()),
            2 => Some(b" :".to_vec()),
            _ => Some(Vec::new()),
        };
        let mut c = Case::new(hex)
            .bin_maxlen(bin_maxlen)
            .want_end(rng.bool());
        c.ignore = ignore;
        check(&c);
    }
}
