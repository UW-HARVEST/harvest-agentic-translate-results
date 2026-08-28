//! Level 1: `w_utf8_drop` — the lowest-level exported function.
//!
//! It is a pure function of the input bytes, so it can be checked exhaustively
//! over short inputs.

mod common;

use common::{Impls, Rng, INTERESTING};

fn check(impls: &Impls, input: &[u8]) {
    let (c, r) = impls.drop_offsets(input);
    assert_eq!(
        c, r,
        "w_utf8_drop offset mismatch for {:02X?}: c={c} rust={r}",
        input
    );
}

#[test]
fn drop_empty_and_ascii() {
    let impls = Impls::load();
    check(&impls, b"");
    check(&impls, b"hello world");
    check(&impls, b"\x01\x7f");
    check(&impls, "héllo wörld — ünïcode ✓ 𝄞".as_bytes());
}

/// Every possible one-byte string.
#[test]
fn drop_exhaustive_len1() {
    let impls = Impls::load();
    for b in 1u8..=255 {
        check(&impls, &[b]);
    }
}

/// Every possible two-byte string (255 * 255).
#[test]
fn drop_exhaustive_len2() {
    let impls = Impls::load();
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            check(&impls, &[a, b]);
        }
    }
}

/// Every possible three-byte string (255^3 ≈ 16.6M).
#[test]
fn drop_exhaustive_len3() {
    let impls = Impls::load();
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            for c in 1u8..=255 {
                check(&impls, &[a, b, c]);
            }
        }
    }
}

/// Four- and five-byte strings drawn from the boundary alphabet.
#[test]
fn drop_boundary_alphabet_len4_len5() {
    let impls = Impls::load();
    for &a in INTERESTING {
        for &b in INTERESTING {
            for &c in INTERESTING {
                for &d in INTERESTING {
                    check(&impls, &[a, b, c, d]);
                    for &e in INTERESTING {
                        check(&impls, &[a, b, c, d, e]);
                    }
                }
            }
        }
    }
}

/// Random longer inputs, both fully random and biased towards lead bytes.
#[test]
fn drop_random_long() {
    let impls = Impls::load();
    let mut rng = Rng::new(0xD1CE_5EED);
    let mut buf = Vec::new();

    for _ in 0..20_000 {
        let len = 1 + rng.below(64);
        buf.clear();
        for _ in 0..len {
            buf.push(rng.byte());
        }
        check(&impls, &buf);
    }

    for _ in 0..20_000 {
        let len = 1 + rng.below(64);
        buf.clear();
        for _ in 0..len {
            buf.push(INTERESTING[rng.below(INTERESTING.len())]);
        }
        check(&impls, &buf);
    }
}

/// A valid multi-byte sequence truncated at the very end of the buffer: the
/// macros must stop at the NUL rather than read past it.
#[test]
fn drop_truncated_sequences() {
    let impls = Impls::load();
    for lead in [0xC2u8, 0xDF, 0xE0, 0xE1, 0xED, 0xEF, 0xF0, 0xF1, 0xF4] {
        check(&impls, &[lead]);
        check(&impls, &[lead, 0x80]);
        check(&impls, &[lead, 0xBF]);
        check(&impls, &[lead, 0x80, 0x80]);
        check(&impls, &[lead, 0xBF, 0xBF]);
        check(&impls, &[b'a', lead]);
        check(&impls, &[b'a', lead, 0x80]);
        check(&impls, &[b'a', lead, 0x80, 0x80]);
    }
}
