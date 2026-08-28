//! Level 2: `w_utf8_filter`, which is built on top of `w_utf8_drop`.
//!
//! Both the `replacement = false` (drop invalid bytes) and `replacement = true`
//! (emit U+FFFD) paths are exercised, including the `realloc` growth path.

mod common;

use common::{hex, Impls, Rng, INTERESTING};

fn check(impls: &Impls, input: &[u8]) {
    for replacement in [false, true] {
        let (c, r) = impls.filter_outputs(input, replacement);
        assert_eq!(
            c.as_deref().map(hex),
            r.as_deref().map(hex),
            "w_utf8_filter(replacement={replacement}) mismatch for {:02X?}",
            input
        );
    }
}

#[test]
fn filter_valid_inputs_take_the_strdup_path() {
    let impls = Impls::load();
    check(&impls, b"");
    check(&impls, b"plain ascii");
    check(&impls, "héllo wörld — ünïcode ✓ 𝄞 \u{10FFFF}".as_bytes());
    // Highest/lowest legal sequence of each length.
    check(&impls, &[0x01]);
    check(&impls, &[0x7F]);
    check(&impls, &[0xC2, 0x80]);
    check(&impls, &[0xDF, 0xBF]);
    check(&impls, &[0xE0, 0xA0, 0x80]);
    check(&impls, &[0xEF, 0xBF, 0xBF]);
    check(&impls, &[0xF0, 0x90, 0x80, 0x80]);
    check(&impls, &[0xF4, 0x8F, 0xBF, 0xBF]);
}

#[test]
fn filter_exhaustive_len1() {
    let impls = Impls::load();
    for b in 1u8..=255 {
        check(&impls, &[b]);
    }
}

#[test]
fn filter_exhaustive_len2() {
    let impls = Impls::load();
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            check(&impls, &[a, b]);
        }
    }
}

/// Exhaustive over the boundary alphabet at lengths 1..=4.
#[test]
fn filter_boundary_alphabet_upto_len4() {
    let impls = Impls::load();
    for &a in INTERESTING {
        check(&impls, &[a]);
        for &b in INTERESTING {
            check(&impls, &[a, b]);
            for &c in INTERESTING {
                check(&impls, &[a, b, c]);
                for &d in INTERESTING {
                    check(&impls, &[a, b, c, d]);
                }
            }
        }
    }
}

/// Exhaustive over the boundary alphabet at length 5.
#[test]
fn filter_boundary_alphabet_len5() {
    let impls = Impls::load();
    for &a in INTERESTING {
        for &b in INTERESTING {
            for &c in INTERESTING {
                for &d in INTERESTING {
                    for &e in INTERESTING {
                        check(&impls, &[a, b, c, d, e]);
                    }
                }
            }
        }
    }
}

/// A valid ASCII prefix followed by garbage exercises the `memcpy(copy, string, i)`
/// prefix copy with every prefix length.
#[test]
fn filter_valid_prefix_then_invalid() {
    let impls = Impls::load();
    for prefix_len in 0..40usize {
        let mut v = vec![b'a'; prefix_len];
        v.push(0xFF);
        check(&impls, &v);

        let mut v = vec![b'a'; prefix_len];
        v.extend_from_slice(&[0xC0, 0x80, b'z']);
        check(&impls, &v);

        // Multi-byte valid prefix.
        let mut v = Vec::new();
        for _ in 0..prefix_len {
            v.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
        }
        v.push(0xF5);
        v.extend_from_slice(&[0xE2, 0x9C, 0x93]);
        check(&impls, &v);
    }
}

#[test]
fn filter_random_long() {
    let impls = Impls::load();
    let mut rng = Rng::new(0x5EED_C0DE);
    let mut buf = Vec::new();

    for _ in 0..8_000 {
        let len = 1 + rng.below(96);
        buf.clear();
        for _ in 0..len {
            buf.push(rng.byte());
        }
        check(&impls, &buf);
    }

    for _ in 0..8_000 {
        let len = 1 + rng.below(96);
        buf.clear();
        for _ in 0..len {
            buf.push(INTERESTING[rng.below(INTERESTING.len())]);
        }
        check(&impls, &buf);
    }

    // Mostly-valid UTF-8 with occasional corruption.
    let seqs: [&[u8]; 5] = [
        b"a",
        &[0xC3, 0xA9],
        &[0xE2, 0x9C, 0x93],
        &[0xF0, 0x9F, 0x98, 0x80],
        &[0xED, 0xA0, 0x80], // surrogate: invalid
    ];
    for _ in 0..8_000 {
        buf.clear();
        let n = 1 + rng.below(30);
        for _ in 0..n {
            if rng.below(8) == 0 {
                buf.push(INTERESTING[rng.below(INTERESTING.len())]);
            } else {
                buf.extend_from_slice(seqs[rng.below(seqs.len())]);
            }
        }
        check(&impls, &buf);
    }
}

/// Many invalid bytes in a row drives the `repl < 3` / `realloc` bookkeeping
/// across several growth steps (one `realloc` per ~1365 replacements).
#[test]
fn filter_realloc_growth_path() {
    let impls = Impls::load();

    for n in [
        1usize, 2, 3, 1364, 1365, 1366, 1367, 2730, 2731, 2732, 4095, 4096, 4097, 8192, 20_000,
    ] {
        check(&impls, &vec![0xFFu8; n]);
        check(&impls, &vec![0x80u8; n]);

        // Alternating valid/invalid so `i` grows faster than the input index.
        let mut v = Vec::with_capacity(n * 2);
        for _ in 0..n {
            v.push(b'a');
            v.push(0xFE);
        }
        check(&impls, &v);
    }

    // Valid 4-byte sequences interleaved with invalid lead bytes: maximises the
    // ratio of output bytes to input bytes while still hitting the else branch.
    let mut v = Vec::new();
    for _ in 0..5000 {
        v.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80, 0xF5]);
    }
    check(&impls, &v);
}

/// Every possible three-byte string (255^3 ≈ 16.6M), both replacement modes.
#[test]
fn filter_exhaustive_len3() {
    let impls = Impls::load();
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            for c in 1u8..=255 {
                check(&impls, &[a, b, c]);
            }
        }
    }
}
