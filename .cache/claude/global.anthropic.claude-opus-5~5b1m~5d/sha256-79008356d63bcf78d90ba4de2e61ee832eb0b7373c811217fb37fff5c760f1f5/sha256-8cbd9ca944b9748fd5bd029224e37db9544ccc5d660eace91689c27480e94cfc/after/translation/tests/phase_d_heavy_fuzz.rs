//! Phase D — heavy / exhaustive differential fuzzing.
//!
//! The per-row tests in `phase_b_configs.rs` and `phase_c_errors.rs` cover the
//! documented axes. These tests go further and make the coverage EXHAUSTIVE for
//! short inputs, which is what actually pins down the `strtod` `endptr`
//! placement, the `offset` advance and the saturation/truncation boundaries.

mod common;

use common::*;

/// Exhaustive: every 1- and 2-byte input over the FULL 256-value byte space.
/// 256 + 65_536 = 65_792 inputs, each compared C vs Rust.
#[test]
fn fuzz_exhaustive_all_bytes_len1_len2() {
    for a in 0u16..=255 {
        diff_str("exh1", &[a as u8]);
    }
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            let s = [a as u8, b as u8];
            diff_str("exh2", &s);
        }
    }
}

/// Exhaustive: every 3-byte input over the accepted character class
/// (`0-9 + - e E .`, 15 values) => 15^3 = 3_375, plus every 3-byte input whose
/// first two bytes are accepted and third is ANY byte (15*15*256 = 57_600).
#[test]
fn fuzz_exhaustive_numeric_len3_plus_any_tail() {
    let alpha = NUMERIC_ALPHABET;
    for &a in alpha {
        for &b in alpha {
            for &c in alpha {
                diff_str("exh3n", &[a, b, c]);
            }
        }
    }
    for &a in alpha {
        for &b in alpha {
            for t in 0u16..=255 {
                diff_str("exh3t", &[a, b, t as u8]);
            }
        }
    }
}

/// Exhaustive: every 4-byte input over the accepted character class.
/// 15^4 = 50_625.
#[test]
fn fuzz_exhaustive_numeric_len4() {
    let alpha = NUMERIC_ALPHABET;
    for &a in alpha {
        for &b in alpha {
            for &c in alpha {
                for &d in alpha {
                    diff_str("exh4n", &[a, b, c, d]);
                }
            }
        }
    }
}

/// Exhaustive length 5 over a reduced but still representative alphabet
/// (`0 1 9 + - . e E`, 8 values) => 8^5 = 32_768.
#[test]
fn fuzz_exhaustive_reduced_len5_len6() {
    const A: &[u8] = b"019+-.eE";
    for &a in A {
        for &b in A {
            for &c in A {
                for &d in A {
                    for &e in A {
                        diff_str("exh5", &[a, b, c, d, e]);
                    }
                }
            }
        }
    }
    // length 6 over an even smaller alphabet: 5^6 = 15_625
    const B: &[u8] = b"1.e-9";
    for &a in B {
        for &b in B {
            for &c in B {
                for &d in B {
                    for &e in B {
                        for &f in B {
                            diff_str("exh6", &[a, b, c, d, e, f]);
                        }
                    }
                }
            }
        }
    }
}

/// Randomized: 300_000 windows over the accepted class with randomized
/// `length` / `offset` / `depth` / `item` pre-state.
#[test]
fn fuzz_numeric_windows_bulk() {
    let rng = Rng::new(SEED ^ 0x1111_2222_3333_4444);
    for k in 0..300_000u32 {
        let n = rng.range(0, 26) as usize;
        let s = rand_from(&rng, NUMERIC_ALPHABET, n);
        let length = rng.below(s.len() as u64 + 1) as usize;
        let offset = rng.below(length as u64 + 2) as usize;
        let item_pre = CJson {
            type_: rng.next_u64() as i32,
            valueint: rng.next_u64() as i32,
            valuedouble: f64::from_bits(rng.next_u64()),
        };
        diff(
            &format!("bulkN#{k}"),
            Some(&s),
            length,
            offset,
            rng.next_u64() as usize,
            item_pre,
        );
    }
}

/// Randomized: 200_000 windows over the FULL byte space.
#[test]
fn fuzz_full_bytespace_bulk() {
    let rng = Rng::new(SEED ^ 0x9999_8888_7777_6666);
    for k in 0..200_000u32 {
        let n = rng.range(0, 48) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let length = rng.below(s.len() as u64 + 1) as usize;
        let offset = rng.below(length as u64 + 2) as usize;
        diff(
            &format!("bulkB#{k}"),
            Some(&s),
            length,
            offset,
            rng.next_u64() as usize,
            CJson::garbage(),
        );
    }
}

/// Randomized: real `f64` bit patterns rendered as decimal text, so `strtod`
/// round-tripping and the `(int)` / saturation branches see realistic values
/// across the whole double range (including subnormals and huge magnitudes).
#[test]
fn fuzz_roundtrip_f64_bitpatterns() {
    let rng = Rng::new(SEED ^ 0xDEAD_BEEF_CAFE_F00D);
    let mut done = 0u32;
    while done < 120_000 {
        let bits = rng.next_u64();
        let x = f64::from_bits(bits);
        if !x.is_finite() {
            continue; // "NaN"/"inf" text is not in the accepted class
        }
        done += 1;
        for text in [
            format!("{x:?}"),
            format!("{x:e}"),
            format!("{x:.0}"),
            format!("{x:.17e}"),
        ] {
            // `{:?}`/`{:e}` never emit characters outside `[0-9.eE+-]` for
            // finite values, so the whole string is inside the accepted class.
            diff_str("rt64", text.as_bytes());
        }
    }
}

/// Randomized: values densely clustered around the two saturation boundaries
/// and around every power of ten, where the `>= INT_MAX` / `<= INT_MIN` /
/// truncate decision flips.
#[test]
fn fuzz_saturation_neighbourhood() {
    let rng = Rng::new(SEED ^ 0x0F0F_0F0F_0F0F_0F0F);
    for k in 0..120_000u32 {
        let pivot: i64 = *rng.pick(&[
            2_147_483_647i64,
            -2_147_483_648,
            2_147_483_648,
            -2_147_483_649,
            0,
            1,
            -1,
            1_000_000_000,
            -1_000_000_000,
            4_294_967_296,
            9_007_199_254_740_992,
        ]);
        let delta = rng.range(0, 8) as i64 - 4;
        let v = pivot.saturating_add(delta);
        let text = match rng.below(5) {
            0 => format!("{v}"),
            1 => format!("{v}.{}", rng.below(1_000_000_000)),
            2 => format!("{v}e0"),
            3 => format!("{v}.5"),
            _ => format!("{v}.99999999999999999999"),
        };
        diff_str(&format!("sat#{k}"), text.as_bytes());
    }
}

/// Randomized: very long inputs (up to 4 KiB) so the temporary `malloc` size,
/// the `memcpy` length and the `'.'`-replacement loop are all stressed.
#[test]
fn fuzz_long_inputs() {
    let rng = Rng::new(SEED ^ 0xABCD_1234_ABCD_1234);
    for k in 0..4_000u32 {
        let n = rng.range(200, 4096) as usize;
        let s = rand_from(&rng, NUMERIC_ALPHABET, n);
        let length = rng.below(s.len() as u64 + 1) as usize;
        let offset = rng.below(length as u64 + 1) as usize;
        diff(&format!("long#{k}"), Some(&s), length, offset, 0, CJson::garbage());
    }
    // pathological: thousands of '.' (forces the replacement loop over the
    // whole buffer) and thousands of digits (exact decimal conversion)
    for n in [256usize, 1024, 4096] {
        let dots = vec![b'.'; n];
        diff_str("long/dots", &dots);
        let mut d = vec![b'1'];
        d.extend(vec![b'0'; n]);
        diff_str("long/digits", &d);
        let mut m = vec![b'0', b'.'];
        m.extend(vec![b'9'; n]);
        diff_str("long/frac", &m);
        let mut e = b"1e".to_vec();
        e.extend(vec![b'9'; n]);
        diff_str("long/exp", &e);
    }
}

/// Every accepted byte inserted at every position of a valid number — catches
/// off-by-one errors in the scan/`memcpy`/`offset`-advance interaction.
#[test]
fn fuzz_single_byte_insertions() {
    let bases: [&[u8]; 6] = [b"1", b"12", b"1.5", b"-1.5e3", b"0", b"1e10"];
    for base in bases {
        for pos in 0..=base.len() {
            for &ins in NUMERIC_ALPHABET {
                let mut v = base[..pos].to_vec();
                v.push(ins);
                v.extend_from_slice(&base[pos..]);
                diff_str("ins", &v);
                // and with every length/offset over the mutated string
                for length in 0..=v.len() {
                    diff("ins/win", Some(&v), length, 0, 0, CJson::garbage());
                }
            }
        }
    }
}
