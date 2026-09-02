//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row is driven with many randomized
//! inputs from a fixed-seed `SplitMix64`, and both `.so`s are compared
//! byte-for-byte over the whole `calloc`ed region.

mod common;

use common::*;

/// Distinct per-row seed so rows do not all replay the same byte stream.
fn seed_for(row: u64) -> u64 {
    SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Drive one row: `ITERS` randomized inputs produced by `gen`.
fn row<F: FnMut(&mut Rng) -> Vec<u8>>(row_no: u64, mut gen: F) {
    let mut rng = Rng::new(seed_for(row_no));
    for _ in 0..ITERS {
        let input = gen(&mut rng);
        assert_same(&input);
    }
}

// --- Row 1-6: single character class at a time -----------------------------

#[test]
fn cfg_01_upper_only() {
    row(1, |r| {
        let len = r.range(1, 256);
        from_set(r, UPPER, len)
    });
}

#[test]
fn cfg_02_lower_only() {
    row(2, |r| {
        let len = r.range(1, 256);
        from_set(r, LOWER, len)
    });
}

#[test]
fn cfg_03_digits_only() {
    row(3, |r| {
        let len = r.range(1, 256);
        from_set(r, DIGITS, len)
    });
}

#[test]
fn cfg_04_plus_only() {
    row(4, |r| {
        let len = r.range(1, 256);
        vec![b'+'; len]
    });
}

#[test]
fn cfg_05_slash_only() {
    // '/' is accepted by is_base64 but falls through decode() to 63.
    row(5, |r| {
        let len = r.range(1, 256);
        vec![b'/'; len]
    });
}

#[test]
fn cfg_06_equals_only() {
    // '=' also falls through decode() to 63 AND trips both suppression branches.
    row(6, |r| {
        let len = r.range(1, 256);
        vec![b'='; len]
    });
}

// --- Rows 7-10: filtered length modulo 4 ----------------------------------

fn len_with_mod4(r: &mut Rng, m: usize) -> usize {
    // length >= 1 with length % 4 == m
    let groups = r.range(0, 40);
    let l = groups * 4 + m;
    if l == 0 {
        4
    } else {
        l
    }
}

#[test]
fn cfg_07_alphabet_mod4_0() {
    row(7, |r| {
        let len = len_with_mod4(r, 0).max(4);
        from_set(r, ALPHABET, len)
    });
}

#[test]
fn cfg_08_alphabet_mod4_1() {
    row(8, |r| {
        let len = len_with_mod4(r, 1);
        from_set(r, ALPHABET, len)
    });
}

#[test]
fn cfg_09_alphabet_mod4_2() {
    row(9, |r| {
        let len = len_with_mod4(r, 2);
        from_set(r, ALPHABET, len)
    });
}

#[test]
fn cfg_10_alphabet_mod4_3() {
    row(10, |r| {
        let len = len_with_mod4(r, 3);
        from_set(r, ALPHABET, len)
    });
}

// --- Rows 11-12: group counts ---------------------------------------------

#[test]
fn cfg_11_single_group() {
    row(11, |r| from_set(r, ALPHABET_EQ, 4));
}

#[test]
fn cfg_12_two_groups() {
    row(12, |r| from_set(r, ALPHABET_EQ, 8));
}

// --- Rows 13-15: canonical RFC-style padded base64 ------------------------

/// Encode `data` with the standard base64 alphabet and `'='` padding, i.e. what
/// a real consumer of this decoder would feed it.
fn b64_encode(data: &[u8]) -> Vec<u8> {
    const T: &[u8] = ALPHABET;
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize]);
        out.push(T[((n >> 12) & 63) as usize]);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize]);
        } else {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize]);
        } else {
            out.push(b'=');
        }
    }
    out
}

fn random_payload(r: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| (r.below(256)) as u8).collect()
}

#[test]
fn cfg_13_canonical_pad1() {
    // payload len % 3 == 2  =>  exactly one '=' of padding
    row(13, |r| {
        let n = r.range(0, 60) * 3 + 2;
        b64_encode(&random_payload(r, n))
    });
}

#[test]
fn cfg_14_canonical_pad2() {
    // payload len % 3 == 1  =>  exactly two '=' of padding
    row(14, |r| {
        let n = r.range(0, 60) * 3 + 1;
        b64_encode(&random_payload(r, n))
    });
}

#[test]
fn cfg_15_canonical_pad0() {
    // payload len % 3 == 0  =>  no padding
    row(15, |r| {
        let n = r.range(1, 60) * 3;
        b64_encode(&random_payload(r, n))
    });
}

// --- Row 16: '=' at arbitrary interior positions ---------------------------

#[test]
fn cfg_16_equals_interior_random() {
    row(16, |r| {
        let len = r.range(1, 120);
        let mut v = from_set(r, ALPHABET, len);
        let inject = r.range(1, 6);
        for _ in 0..inject {
            let pos = r.below(v.len());
            v[pos] = b'=';
        }
        v
    });
}

// --- Rows 17-19: is_base64 filtering --------------------------------------

#[test]
fn cfg_17_mixed_with_noise() {
    row(17, |r| {
        let len = r.range(1, 200);
        (0..len)
            .map(|_| {
                if r.below(3) == 0 {
                    r.pick(NOISE)
                } else {
                    r.pick(ALPHABET_EQ)
                }
            })
            .collect()
    });
}

#[test]
fn cfg_18_mixed_with_high_bit() {
    // Bytes >= 0x80 are negative as `char` on x86-64 Linux, so every
    // `c >= 'A'` style test in is_base64 fails and they are dropped.
    row(18, |r| {
        let len = r.range(1, 200);
        (0..len)
            .map(|_| {
                if r.below(2) == 0 {
                    (0x80 + r.below(128)) as u8
                } else {
                    r.pick(ALPHABET_EQ)
                }
            })
            .collect()
    });
}

#[test]
fn cfg_19_all_noise_empty_result() {
    // Nothing survives the filter => l == 0 => decode loop never runs.
    // The C returns a NON-NULL zero-filled buffer here, not NULL.
    row(19, |r| {
        let len = r.range(1, 64);
        from_set(r, NOISE, len)
    });
    // Pin the exact contract for this shape.
    let out = assert_same(b"!!!");
    match out {
        Outcome::Buffer { full, c_strlen, .. } => {
            assert_eq!(c_strlen, 0, "all-noise input must decode to the empty string");
            assert!(
                full.iter().all(|&b| b == 0),
                "buffer must be entirely zero-filled: {}",
                hex(&full)
            );
            assert_eq!(full.len(), 3 + 1 + 13);
        }
        Outcome::Null => panic!("C returns non-NULL for all-noise input; got NULL"),
    }
}

// --- Row 20: interior NUL bytes in the decoded output ---------------------

#[test]
fn cfg_20_output_with_interior_nuls() {
    // "AAAA" decodes to 00 00 00, so any payload with zero bytes produces
    // interior NULs that a strlen-based comparison would silently miss.
    for fixed in [
        &b"AAAA"[..],
        b"AAAAAAAA",
        b"QUJDAAAAQUJD",
        b"AA==",
        b"AAA=",
        b"////AAAA////",
    ] {
        let out = assert_same(fixed);
        if let Outcome::Buffer { full, c_strlen, .. } = &out {
            assert!(full.iter().any(|&b| b == 0));
            let _ = c_strlen;
        }
    }
    // Randomized: payloads that are mostly zero bytes.
    row(20, |r| {
        let n = r.range(1, 90);
        let payload: Vec<u8> = (0..n)
            .map(|_| if r.below(2) == 0 { 0 } else { (r.below(256)) as u8 })
            .collect();
        b64_encode(&payload)
    });
}

// --- Row 21: characters one step outside every range ---------------------

#[test]
fn cfg_21_range_boundary_chars() {
    const BOUNDARY: &[u8] = b"@[`{/:*,.-_ +=";
    // Every boundary char alone, and every ordered pair of them.
    for &a in BOUNDARY {
        assert_same(&[a]);
        for &b in BOUNDARY {
            assert_same(&[a, b]);
            assert_same(&[a, b, b'A']);
            assert_same(&[a, b, b'A', b'z']);
        }
    }
    row(21, |r| {
        let len = r.range(1, 60);
        (0..len)
            .map(|_| {
                if r.below(2) == 0 {
                    r.pick(BOUNDARY)
                } else {
                    r.pick(ALPHABET)
                }
            })
            .collect()
    });
}

// --- Row 22: arbitrary byte fuzz -----------------------------------------

#[test]
fn cfg_22_arbitrary_bytes_fuzz() {
    row(22, |r| {
        let len = r.range(1, 512);
        (0..len).map(|_| r.nonnul_byte()).collect()
    });
}

// --- Rows 23-24: exhaustive short inputs ---------------------------------

#[test]
fn cfg_23_exhaustive_single_char() {
    for b in 1u16..=255 {
        assert_same(&[b as u8]);
    }
}

#[test]
fn cfg_24_exhaustive_char_pairs() {
    // 65 x 65 = 4225 pairs over the accepted alphabet plus '='.
    for &a in ALPHABET_EQ {
        for &b in ALPHABET_EQ {
            assert_same(&[a, b]);
        }
    }
    // And every (accepted, accepted, accepted) triple over a reduced but
    // representative set covering each decode() class and both fall-throughs.
    const REP: &[u8] = b"AZaz09+/=";
    for &a in REP {
        for &b in REP {
            for &c in REP {
                assert_same(&[a, b, c]);
                for &d in REP {
                    assert_same(&[a, b, c, d]);
                }
            }
        }
    }
}

// --- Rows 25-26: scale and '='-heavy ASCII fuzz --------------------------

#[test]
fn cfg_25_long_inputs() {
    let mut rng = Rng::new(seed_for(25));
    for _ in 0..40 {
        let len = rng.range(1000, 4096);
        let input = from_set(&mut rng, ALPHABET_EQ, len);
        assert_same(&input);
    }
}

#[test]
fn cfg_26_ascii_fuzz_equals_heavy() {
    row(26, |r| {
        let len = r.range(1, 300);
        (0..len)
            .map(|_| match r.below(4) {
                0 => b'=',
                1 => r.pick(NOISE),
                _ => (0x20 + r.below(0x5f)) as u8, // printable ASCII
            })
            .collect()
    });
}
