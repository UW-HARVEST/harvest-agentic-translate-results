//! High-volume adversarial sweep. Not tied to a single `CONFIGS.md` /
//! `ERRORS.md` row — this randomizes across the ENTIRE parameter space at once
//! to catch interactions the structured rows might miss.

mod harness;
use harness::*;

/// Alphabet biased towards the interesting boundaries of the branch-free
/// classifier so random inputs actually straddle the accept/reject edges.
const ALPHABET: &[u8] = b"0123456789abcdefABCDEF \t\n\r:-_.,;gGhHzZ/@`\x00\x7f\x80\xff!*+[]";

fn random_ignore_set(rng: &mut Rng) -> Option<Vec<u8>> {
    match rng.below(6) {
        0 => None,
        1 => Some(b"".to_vec()),
        2 => Some(b" ".to_vec()),
        3 => Some(b":".to_vec()),
        4 => Some(b" :\n\t\r-".to_vec()),
        _ => {
            // A random ignore set drawn from the alphabet (never containing a
            // NUL, which would truncate it).
            let n = rng.range(1, 6);
            let mut v = Vec::new();
            for _ in 0..n {
                loop {
                    let c = *rng.pick(ALPHABET);
                    if c != 0 {
                        v.push(c);
                        break;
                    }
                }
            }
            Some(v)
        }
    }
}

/// 200k fully-randomized calls across every axis simultaneously.
#[test]
fn stress_full_parameter_space() {
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let cap = 72usize;
    for i in 0..200_000u32 {
        let n = rng.below(65);
        // Mix "structured" and "pure noise" inputs.
        let hex: Vec<u8> = if rng.bool() {
            (0..n).map(|_| *rng.pick(ALPHABET)).collect()
        } else {
            (0..n).map(|_| rng.u8()).collect()
        };

        // hex_len sometimes shorter than the buffer we built (never longer,
        // so neither implementation reads out of bounds).
        let hex_len = if rng.bool() { n } else { rng.below(n + 1) };

        // bin_maxlen spanning 0, tight, exact, generous and SIZE_MAX.
        let bin_maxlen = match rng.below(6) {
            0 => 0,
            1 => rng.below(4),
            2 => hex_len / 2,
            3 => rng.below(cap - 8 + 1),
            4 => usize::MAX,
            _ => rng.below(41),
        };

        let mut call = Call::new(hex)
            .bin_cap(cap)
            .hex_len(hex_len)
            .bin_maxlen(bin_maxlen)
            .poison(rng.u8())
            .hex_end(rng.bool());
        call = match random_ignore_set(&mut rng) {
            None => call.no_ignore(),
            Some(s) => call.ignore(&s),
        };

        assert_same(&call, &format!("stress iteration {i}"));
    }
}

/// Exhaustive 3-character sweep over the interesting alphabet, crossed with
/// the ignore-set and `hex_end_p` options: catches state-parity interactions
/// that need three characters to express (digit / separator / digit).
#[test]
fn stress_exhaustive_triples_over_alphabet() {
    const A: &[u8] = b"0aF9g: \x00\xff/@";
    const SETS: &[Option<&[u8]>] = &[None, Some(b""), Some(b" "), Some(b" :\n\t-")];
    for &c0 in A {
        for &c1 in A {
            for &c2 in A {
                for set in SETS {
                    for he in [true, false] {
                        for maxlen in [0usize, 1, 2, 8] {
                            let mut call = Call::new(vec![c0, c1, c2])
                                .bin_cap(16)
                                .bin_maxlen(maxlen)
                                .hex_end(he);
                            call = match set {
                                None => call.no_ignore(),
                                Some(s) => call.ignore(s),
                            };
                            assert_same(
                                &call,
                                &format!(
                                    "triple {c0:#04x},{c1:#04x},{c2:#04x} maxlen={maxlen} he={he}"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Exhaustive 4-character sweep over a smaller alphabet with a separator-aware
/// ignore set, so every state-parity x separator-position combination for two
/// decoded bytes is covered.
#[test]
fn stress_exhaustive_quads() {
    const A: &[u8] = b"0F: \x00g";
    for &c0 in A {
        for &c1 in A {
            for &c2 in A {
                for &c3 in A {
                    for set in [None, Some(&b" :"[..])] {
                        for he in [true, false] {
                            for maxlen in [0usize, 1, 2, 4] {
                                let mut call = Call::new(vec![c0, c1, c2, c3])
                                    .bin_cap(16)
                                    .bin_maxlen(maxlen)
                                    .hex_end(he);
                                call = match set {
                                    None => call.no_ignore(),
                                    Some(s) => call.ignore(s),
                                };
                                assert_same(
                                    &call,
                                    &format!(
                                        "quad {c0:#04x},{c1:#04x},{c2:#04x},{c3:#04x} \
                                         maxlen={maxlen} he={he}"
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Long inputs (up to 64 KiB of hex) with separators, so any 16-bit/8-bit
/// truncation in position or length arithmetic would show up.
#[test]
fn stress_long_inputs() {
    let mut rng = Rng::new(SEED ^ 0x10_9E_ABCD);
    for _ in 0..24 {
        let nbytes = rng.range(4096, 32768);
        let mut hex = Vec::with_capacity(nbytes * 2 + nbytes / 4);
        for i in 0..nbytes {
            if i > 0 && i % 7 == 0 {
                hex.push(b':');
            }
            hex.push(rng.hex_digit());
            hex.push(rng.hex_digit());
        }
        let n = hex.len();
        for he in [true, false] {
            let call = Call::new(hex.clone())
                .bin_cap(nbytes + 8)
                .bin_maxlen(nbytes)
                .hex_len(n)
                .ignore(b":")
                .hex_end(he);
            assert_same(&call, "stress long input");
        }
        // And a truncating bin_maxlen on the same long input.
        let maxlen = rng.below(nbytes);
        let call = Call::new(hex)
            .bin_cap(nbytes + 8)
            .bin_maxlen(maxlen)
            .hex_len(n)
            .ignore(b":")
            .hex_end(true);
        assert_same(&call, "stress long input truncated");
    }
}
