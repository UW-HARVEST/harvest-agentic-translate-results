//! Phase B extension — EXHAUSTIVE small-input differential tests
//! (`CONFIGS.md` rows 31 and 32).
//!
//! Randomized testing can miss a single byte value; for `hex_len <= 2` the
//! whole input space is small enough to enumerate completely, which pins down
//! the character classifier and the nibble-pairing logic for every possible
//! byte. Longer inputs are enumerated exhaustively over a representative
//! alphabet of boundary bytes.

mod common;

use common::*;

/// Row 31 — every possible input of length 0, 1 and 2 (1 + 256 + 65536 inputs)
/// against the full option matrix.
#[test]
fn cfg_31_exhaustive_len_0_1_2() {
    let ign_variants: [Option<&[u8]>; 4] =
        [None, Some(b""), Some(b":"), Some(b" \t\r\n:-")];

    // length 0 and 1: full option matrix
    for b in 0u16..=255 {
        for ign in ign_variants {
            for &want_end in &[false, true] {
                for &bm in &[0usize, 1, 2, usize::MAX] {
                    let mut c0 = Case::new(Vec::<u8>::new())
                        .bin_maxlen(bm)
                        .want_end(want_end);
                    c0.ignore = ign.map(|v| v.to_vec());
                    check(&c0);
                    let mut c1 = Case::new(vec![b as u8]).bin_maxlen(bm).want_end(want_end);
                    c1.ignore = ign.map(|v| v.to_vec());
                    check(&c1);
                }
            }
        }
    }

    // length 2: all 65536 byte pairs, two full option settings ...
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            let hex = vec![hi as u8, lo as u8];
            check(&Case::new(hex.clone()).no_ignore().bin_maxlen(1).want_end(true));
            let mut c = Case::new(hex).bin_maxlen(1).want_end(false);
            c.ignore = Some(b":".to_vec());
            check(&c);
        }
    }

    // ... and a strided sample of the pairs against the remaining options.
    let mut n = 0usize;
    for hi in 0u16..=255 {
        for lo in 0u16..=255 {
            n += 1;
            if n % 5 != 0 {
                continue;
            }
            let hex = vec![hi as u8, lo as u8];
            for ign in ign_variants {
                for &want_end in &[false, true] {
                    for &bm in &[0usize, 1, 2, usize::MAX] {
                        let mut c = Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                        c.ignore = ign.map(|v| v.to_vec());
                        check(&c);
                    }
                }
            }
        }
    }
}

/// A representative alphabet: both ends of every accepted range, the bytes just
/// outside them, NUL, high-bit bytes, and typical separators.
const REPR: &[u8] = &[
    0x00, 0x20, 0x2f, 0x30, 0x39, 0x3a, 0x40, 0x41, 0x46, 0x47, 0x60, 0x61, 0x66, 0x67, 0x7f, 0x80,
    0xff, b':',
];

/// Row 32 — exhaustive over `REPR` for lengths 3 and 4 (18^3 + 18^4 inputs).
#[test]
fn cfg_32_exhaustive_repr_len_3_4() {
    // length 3
    for &a in REPR {
        for &b in REPR {
            for &c in REPR {
                let hex = vec![a, b, c];
                for ign in [None, Some(b":".to_vec()), Some(Vec::new())] {
                    for &want_end in &[false, true] {
                        for &bm in &[0usize, 1, 2] {
                            let mut case =
                                Case::new(hex.clone()).bin_maxlen(bm).want_end(want_end);
                            case.ignore = ign.clone();
                            check(&case);
                        }
                    }
                }
            }
        }
    }
    // length 4
    for &a in REPR {
        for &b in REPR {
            for &c in REPR {
                for &d in REPR {
                    let hex = vec![a, b, c, d];
                    check(&Case::new(hex.clone()).no_ignore().bin_maxlen(2).want_end(true));
                    let mut case = Case::new(hex.clone()).bin_maxlen(2).want_end(false);
                    case.ignore = Some(b":".to_vec());
                    check(&case);
                    let mut case = Case::new(hex).bin_maxlen(1).want_end(true);
                    case.ignore = Some(b" \t\r\n:-".to_vec());
                    check(&case);
                }
            }
        }
    }
}

/// Row 33 — every possible 3-byte input (all 16 777 216 triples), under two
/// option settings. This exhausts the interaction of the classifier, the
/// odd-digit error path and the ignore-skip logic for short inputs.
#[test]
fn cfg_33_exhaustive_all_triples() {
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            for c in 0u16..=255 {
                let hex = vec![a as u8, b as u8, c as u8];
                check(&Case::new(hex.clone()).no_ignore().bin_maxlen(2).want_end(true));
                let mut case = Case::new(hex).bin_maxlen(1).want_end(false);
                case.ignore = Some(b":".to_vec());
                check(&case);
            }
        }
    }
}
