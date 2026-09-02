//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported C symbols and
//! compares byte-for-byte. Inputs are randomized from a fixed seed.

mod common;

use common::*;

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row << 32))
}

// ===========================================================================
// w_utf8_drop — lowest-level entry point
// ===========================================================================

#[test]
fn row01_drop_empty() {
    let p = pair();
    assert_drop_eq(&p, b"", "row01 empty");
}

#[test]
fn row02_drop_pure_ascii() {
    let p = pair();
    let mut rng = rng_for(2);
    for _ in 0..2000 {
        let len = rng.range(1, 256);
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0x7F)).collect();
        assert_drop_eq(&p, &s, "row02 ascii");
    }
}

#[test]
fn row06_drop_valid_2byte_only() {
    let p = pair();
    // Exhaustive over every valid 2-byte sequence.
    for lead in 0xC2u8..=0xDF {
        for cont in 0x80u8..=0xBF {
            assert_drop_eq(&p, &[lead, cont], "row06 exhaustive 2-byte");
        }
    }
    // Then randomly concatenated runs.
    let mut rng = rng_for(6);
    for _ in 0..1000 {
        let n = rng.range(1, 64);
        assert_drop_eq(&p, &valid_mixed(&mut rng, n, &[2]), "row06 concat");
    }
}

#[test]
fn row07_drop_valid_3byte_only() {
    let p = pair();
    let mut rng = rng_for(7);
    // Exhaustive over (lead, cont1) with a random cont2.
    for lead in 0xE0u8..=0xEF {
        let (lo, hi) = match lead {
            0xE0 => (0xA0u8, 0xBFu8),
            0xED => (0x80, 0x9F),
            _ => (0x80, 0xBF),
        };
        for c1 in lo..=hi {
            for _ in 0..4 {
                let c2 = rng.byte_in(0x80, 0xBF);
                assert_drop_eq(&p, &[lead, c1, c2], "row07 exhaustive 3-byte");
            }
        }
    }
    for _ in 0..1000 {
        let n = rng.range(1, 64);
        assert_drop_eq(&p, &valid_mixed(&mut rng, n, &[3]), "row07 concat");
    }
}

#[test]
fn row08_drop_valid_4byte_only() {
    let p = pair();
    let mut rng = rng_for(8);
    for lead in 0xF0u8..=0xF4 {
        let (lo, hi) = match lead {
            0xF0 => (0x90u8, 0xBFu8),
            0xF4 => (0x80, 0x8F),
            _ => (0x80, 0xBF),
        };
        for c1 in lo..=hi {
            for _ in 0..4 {
                let c2 = rng.byte_in(0x80, 0xBF);
                let c3 = rng.byte_in(0x80, 0xBF);
                assert_drop_eq(&p, &[lead, c1, c2, c3], "row08 exhaustive 4-byte");
            }
        }
    }
    for _ in 0..1000 {
        let n = rng.range(1, 64);
        assert_drop_eq(&p, &valid_mixed(&mut rng, n, &[4]), "row08 concat");
    }
}

#[test]
fn row09_drop_mixed_valid_widths() {
    let p = pair();
    let mut rng = rng_for(9);
    for _ in 0..3000 {
        let n = rng.range(0, 96);
        assert_drop_eq(&p, &valid_mixed(&mut rng, n, &[1, 2, 3, 4]), "row09 mixed");
    }
}

#[test]
fn row10_drop_valid_prefix_then_invalid() {
    let p = pair();
    let mut rng = rng_for(10);
    for _ in 0..4000 {
        let n = rng.range(0, 40);
        let mut s = valid_mixed(&mut rng, n, &[1, 2, 3, 4]);
        s.push(definitely_invalid_byte(&mut rng));
        let tail = rng.range(0, 16);
        s.extend(random_bytes(&mut rng, tail));
        assert_drop_eq(&p, &s, "row10 prefix+invalid+tail");
    }
}

#[test]
fn row11_drop_truncated_sequences_at_end_of_buffer() {
    let p = pair();
    let mut rng = rng_for(11);
    // Every possible truncation of every width, with and without a valid prefix.
    for prefix_cp in [0usize, 1, 5] {
        for _ in 0..64 {
            let prefix = valid_mixed(&mut rng, prefix_cp, &[1, 2, 3, 4]);
            for width in 2usize..=4 {
                let mut full = Vec::new();
                match width {
                    2 => valid2(&mut rng, &mut full),
                    3 => valid3(&mut rng, &mut full),
                    _ => valid4(&mut rng, &mut full),
                }
                for keep in 1..width {
                    let mut s = prefix.clone();
                    s.extend_from_slice(&full[..keep]);
                    assert_drop_eq(&p, &s, "row11 truncated");
                }
            }
        }
    }
}

#[test]
fn row12_drop_uniform_random_short() {
    let p = pair();
    let mut rng = rng_for(12);
    for _ in 0..20000 {
        let len = rng.range(0, 64);
        assert_drop_eq(&p, &random_bytes(&mut rng, len), "row12 random short");
    }
}

#[test]
fn row13_drop_uniform_random_long() {
    let p = pair();
    let mut rng = rng_for(13);
    for _ in 0..40 {
        let len = rng.range(4000, 20000);
        assert_drop_eq(&p, &random_bytes(&mut rng, len), "row13 random long");
    }
}

#[test]
fn row14_drop_biased_random() {
    let p = pair();
    let mut rng = rng_for(14);
    for _ in 0..8000 {
        let len = rng.range(0, 128);
        assert_drop_eq(&p, &biased_bytes(&mut rng, len), "row14 biased");
    }
    for _ in 0..40 {
        let len = rng.range(4000, 12000);
        assert_drop_eq(&p, &biased_bytes(&mut rng, len), "row14 biased long");
    }
}

// ===========================================================================
// w_utf8_filter, replacement = 0 (drop mode)
// ===========================================================================

#[test]
fn row15_filter_drop_empty() {
    let p = pair();
    assert_filter_eq(&p, b"", 0, "row15 empty");
}

#[test]
fn row16_filter_drop_valid_ascii_strdup_shortcut() {
    let p = pair();
    let mut rng = rng_for(16);
    for _ in 0..2000 {
        let len = rng.range(1, 256);
        let s: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0x7F)).collect();
        assert_filter_eq(&p, &s, 0, "row16 ascii strdup");
    }
}

#[test]
fn row17_filter_drop_valid_mixed_strdup_shortcut() {
    let p = pair();
    let mut rng = rng_for(17);
    for _ in 0..3000 {
        let n = rng.range(0, 96);
        let s = valid_mixed(&mut rng, n, &[1, 2, 3, 4]);
        assert_filter_eq(&p, &s, 0, "row17 mixed strdup");
    }
}

#[test]
fn row18_filter_drop_invalid_at_offset_zero() {
    let p = pair();
    let mut rng = rng_for(18);
    for _ in 0..4000 {
        let mut s = vec![definitely_invalid_byte(&mut rng)];
        let n = rng.range(0, 24);
        s.extend(valid_mixed(&mut rng, n, &[1, 2, 3, 4]));
        assert_filter_eq(&p, &s, 0, "row18 invalid@0");
    }
}

#[test]
fn row19_filter_drop_invalid_in_middle() {
    let p = pair();
    let mut rng = rng_for(19);
    for _ in 0..4000 {
        let mut s = valid_mixed_n(&mut rng, 1, 24, &[1, 2, 3, 4]);
        s.push(definitely_invalid_byte(&mut rng));
        s.extend(valid_mixed_n(&mut rng, 1, 24, &[1, 2, 3, 4]));
        assert_filter_eq(&p, &s, 0, "row19 invalid middle");
    }
}

#[test]
fn row20_filter_drop_invalid_as_last_byte() {
    let p = pair();
    let mut rng = rng_for(20);
    for _ in 0..4000 {
        let mut s = valid_mixed_n(&mut rng, 1, 32, &[1, 2, 3, 4]);
        s.push(definitely_invalid_byte(&mut rng));
        assert_filter_eq(&p, &s, 0, "row20 invalid last");
    }
}

#[test]
fn row21_filter_drop_all_invalid() {
    let p = pair();
    let mut rng = rng_for(21);
    for len in 1..=300 {
        assert_filter_eq(&p, &invalid_run(&mut rng, len), 0, "row21 all invalid");
    }
}

#[test]
fn row22_filter_drop_each_width_after_first_invalid() {
    let p = pair();
    let mut rng = rng_for(22);
    for width in 1u8..=4 {
        for _ in 0..500 {
            let mut s = vec![definitely_invalid_byte(&mut rng)];
            s.extend(valid_mixed_n(&mut rng, 1, 40, &[width]));
            assert_filter_eq(&p, &s, 0, "row22 width run");
        }
    }
}

#[test]
fn row23_filter_drop_uniform_random_short() {
    let p = pair();
    let mut rng = rng_for(23);
    for _ in 0..20000 {
        let len = rng.range(0, 64);
        assert_filter_eq(&p, &random_bytes(&mut rng, len), 0, "row23 random");
    }
}

#[test]
fn row24_filter_drop_biased_random() {
    let p = pair();
    let mut rng = rng_for(24);
    for _ in 0..8000 {
        let len = rng.range(0, 512);
        assert_filter_eq(&p, &biased_bytes(&mut rng, len), 0, "row24 biased");
    }
}

#[test]
fn row25_filter_drop_long_random() {
    let p = pair();
    let mut rng = rng_for(25);
    for _ in 0..40 {
        let len = rng.range(4000, 20000);
        assert_filter_eq(&p, &random_bytes(&mut rng, len), 0, "row25 long");
        let len = rng.range(4000, 20000);
        assert_filter_eq(&p, &biased_bytes(&mut rng, len), 0, "row25 long biased");
    }
}

#[test]
fn row26_filter_drop_truncated_sequences() {
    let p = pair();
    let mut rng = rng_for(26);
    for prefix_cp in [0usize, 1, 5] {
        for _ in 0..64 {
            let prefix = valid_mixed(&mut rng, prefix_cp, &[1, 2, 3, 4]);
            for width in 2usize..=4 {
                let mut full = Vec::new();
                match width {
                    2 => valid2(&mut rng, &mut full),
                    3 => valid3(&mut rng, &mut full),
                    _ => valid4(&mut rng, &mut full),
                }
                for keep in 1..width {
                    let mut s = prefix.clone();
                    s.extend_from_slice(&full[..keep]);
                    assert_filter_eq(&p, &s, 0, "row26 truncated");
                }
            }
        }
    }
}

// ===========================================================================
// w_utf8_filter, replacement = 1 (U+FFFD substitution; realloc accounting live)
// ===========================================================================

#[test]
fn row27_filter_repl_empty() {
    let p = pair();
    assert_filter_eq(&p, b"", 1, "row27 empty");
}

#[test]
fn row28_filter_repl_valid_input_strdup_shortcut() {
    let p = pair();
    let mut rng = rng_for(28);
    for _ in 0..3000 {
        let n = rng.range(0, 96);
        let s = valid_mixed(&mut rng, n, &[1, 2, 3, 4]);
        assert_filter_eq(&p, &s, 1, "row28 valid strdup");
    }
}

#[test]
fn row29_filter_repl_single_invalid_byte() {
    let p = pair();
    let mut rng = rng_for(29);
    for _ in 0..4000 {
        let mut s = valid_mixed_n(&mut rng, 0, 24, &[1, 2, 3, 4]);
        s.push(definitely_invalid_byte(&mut rng));
        s.extend(valid_mixed_n(&mut rng, 0, 24, &[1, 2, 3, 4]));
        assert_filter_eq(&p, &s, 1, "row29 one invalid");
    }
}

#[test]
fn row30_filter_repl_two_invalid_bytes() {
    let p = pair();
    let mut rng = rng_for(30);
    for _ in 0..4000 {
        let mut s = valid_mixed_n(&mut rng, 0, 16, &[1, 2, 3, 4]);
        s.push(definitely_invalid_byte(&mut rng));
        s.extend(valid_mixed_n(&mut rng, 0, 16, &[1, 2, 3, 4]));
        s.push(definitely_invalid_byte(&mut rng));
        s.extend(valid_mixed_n(&mut rng, 0, 16, &[1, 2, 3, 4]));
        assert_filter_eq(&p, &s, 1, "row30 two invalid");
    }
}

#[test]
fn row31_filter_repl_invalid_at_offset_zero() {
    let p = pair();
    let mut rng = rng_for(31);
    for _ in 0..4000 {
        let mut s = vec![definitely_invalid_byte(&mut rng)];
        s.extend(valid_mixed_n(&mut rng, 0, 24, &[1, 2, 3, 4]));
        assert_filter_eq(&p, &s, 1, "row31 invalid@0");
    }
}

/// `repl` walks 0 -> 4096 -> 4093 -> ... -> 1 -> 4097 -> ...; the first realloc
/// covers 1365 replacements, later ones 1366. Sweep both boundaries.
#[test]
fn row32_filter_repl_first_realloc_cadence() {
    let p = pair();
    let mut rng = rng_for(32);
    for count in [1363usize, 1364, 1365, 1366, 1367] {
        for _ in 0..3 {
            // pure invalid run
            assert_filter_eq(&p, &invalid_run(&mut rng, count), 1, "row32 pure run");
            // invalid bytes separated by valid code points
            let mut s = Vec::new();
            for _ in 0..count {
                s.extend(valid_mixed_n(&mut rng, 0, 2, &[1, 2, 3, 4]));
                s.push(definitely_invalid_byte(&mut rng));
            }
            assert_filter_eq(&p, &s, 1, "row32 interleaved");
        }
    }
}

#[test]
fn row33_filter_repl_third_realloc_cadence() {
    let p = pair();
    let mut rng = rng_for(33);
    for count in [2729usize, 2730, 2731, 2732, 2733, 4095, 4096, 4097] {
        assert_filter_eq(&p, &invalid_run(&mut rng, count), 1, "row33 pure run");
        let mut s = Vec::new();
        for _ in 0..count {
            s.extend(valid_mixed_n(&mut rng, 0, 2, &[1, 2, 3, 4]));
            s.push(definitely_invalid_byte(&mut rng));
        }
        assert_filter_eq(&p, &s, 1, "row33 interleaved");
    }
}

#[test]
fn row34_filter_repl_many_reallocs() {
    let p = pair();
    let mut rng = rng_for(34);
    for _ in 0..20 {
        let count = rng.range(4000, 8000);
        assert_filter_eq(&p, &invalid_run(&mut rng, count), 1, "row34 many reallocs");
    }
}

#[test]
fn row35_filter_repl_all_invalid_small() {
    let p = pair();
    let mut rng = rng_for(35);
    for len in 1..=300 {
        assert_filter_eq(&p, &invalid_run(&mut rng, len), 1, "row35 all invalid");
    }
}

#[test]
fn row36_filter_repl_interleaved_each_width() {
    let p = pair();
    let mut rng = rng_for(36);
    for width in 1u8..=4 {
        for _ in 0..500 {
            let mut s = Vec::new();
            for _ in 0..rng.range(1, 20) {
                s.extend(valid_mixed_n(&mut rng, 0, 3, &[width]));
                s.push(definitely_invalid_byte(&mut rng));
            }
            assert_filter_eq(&p, &s, 1, "row36 interleaved width");
        }
    }
}

#[test]
fn row37_filter_repl_uniform_random_short() {
    let p = pair();
    let mut rng = rng_for(37);
    for _ in 0..20000 {
        let len = rng.range(0, 64);
        assert_filter_eq(&p, &random_bytes(&mut rng, len), 1, "row37 random");
    }
}

#[test]
fn row38_filter_repl_biased_random() {
    let p = pair();
    let mut rng = rng_for(38);
    for _ in 0..8000 {
        let len = rng.range(0, 512);
        assert_filter_eq(&p, &biased_bytes(&mut rng, len), 1, "row38 biased");
    }
}

#[test]
fn row39_filter_repl_long_random() {
    let p = pair();
    let mut rng = rng_for(39);
    for _ in 0..40 {
        let len = rng.range(4000, 20000);
        assert_filter_eq(&p, &random_bytes(&mut rng, len), 1, "row39 long");
        let len = rng.range(4000, 20000);
        assert_filter_eq(&p, &biased_bytes(&mut rng, len), 1, "row39 long biased");
    }
}

#[test]
fn row40_filter_repl_length_around_replacement_inc() {
    let p = pair();
    let mut rng = rng_for(40);
    for total in [4094usize, 4095, 4096, 4097, 4098, 8191, 8192, 8193] {
        for pos in [0usize, 1, total / 2, total - 1] {
            let mut s: Vec<u8> = (0..total).map(|_| rng.byte_in(0x01, 0x7F)).collect();
            s[pos] = definitely_invalid_byte(&mut rng);
            for m in CANONICAL_MODES {
                assert_filter_eq(&p, &s, m, "row40 around 4096");
            }
        }
    }
}

#[test]
fn row41_filter_repl_truncated_sequences() {
    let p = pair();
    let mut rng = rng_for(41);
    for prefix_cp in [0usize, 1, 5] {
        for _ in 0..64 {
            let prefix = valid_mixed(&mut rng, prefix_cp, &[1, 2, 3, 4]);
            for width in 2usize..=4 {
                let mut full = Vec::new();
                match width {
                    2 => valid2(&mut rng, &mut full),
                    3 => valid3(&mut rng, &mut full),
                    _ => valid4(&mut rng, &mut full),
                }
                for keep in 1..width {
                    let mut s = prefix.clone();
                    s.extend_from_slice(&full[..keep]);
                    assert_filter_eq(&p, &s, 1, "row41 truncated");
                }
            }
        }
    }
}

// ===========================================================================
// Non-canonical `_Bool` bytes
// ===========================================================================

const NONCANONICAL: [u8; 5] = [2, 3, 0x7F, 0x80, 0xFF];

#[test]
fn row42_filter_noncanonical_bool_valid_input() {
    let p = pair();
    let mut rng = rng_for(42);
    for _ in 0..1000 {
        let s = valid_mixed_n(&mut rng, 0, 48, &[1, 2, 3, 4]);
        for m in NONCANONICAL {
            assert_filter_eq(&p, &s, m, "row42 noncanonical bool, valid input");
        }
    }
}

#[test]
fn row43_filter_noncanonical_bool_invalid_at_zero() {
    let p = pair();
    let mut rng = rng_for(43);
    for _ in 0..1000 {
        let mut s = vec![definitely_invalid_byte(&mut rng)];
        s.extend(valid_mixed_n(&mut rng, 0, 24, &[1, 2, 3, 4]));
        for m in NONCANONICAL {
            assert_filter_eq(&p, &s, m, "row43 noncanonical bool, invalid@0");
        }
    }
}

#[test]
fn row44_filter_noncanonical_bool_random() {
    let p = pair();
    let mut rng = rng_for(44);
    for _ in 0..4000 {
        let len = rng.range(0, 64);
        let s = random_bytes(&mut rng, len);
        for m in NONCANONICAL {
            assert_filter_eq(&p, &s, m, "row44 noncanonical bool, random");
        }
    }
}

/// A non-canonical boolean must also drive the realloc path identically.
#[test]
fn row45_filter_noncanonical_bool_realloc_path() {
    let p = pair();
    let mut rng = rng_for(45);
    for m in NONCANONICAL {
        for count in [1400usize, 2800] {
            assert_filter_eq(&p, &invalid_run(&mut rng, count), m, "row45 noncanonical realloc");
        }
    }
}

// ===========================================================================
// Composed pipeline
// ===========================================================================

/// Both entry points, all seven mode bytes, on the same buffer.
#[test]
fn row46_composed_drop_then_filter_all_modes() {
    let p = pair();
    let mut rng = rng_for(46);
    for _ in 0..3000 {
        let len = rng.range(0, 96);
        let s = if rng.bool() {
            random_bytes(&mut rng, len)
        } else {
            biased_bytes(&mut rng, len)
        };
        assert_all_eq(&p, &s, "row46 composed");
    }
    // valid-only and all-invalid extremes through the same path
    for _ in 0..300 {
        let s = valid_mixed_n(&mut rng, 0, 32, &[1, 2, 3, 4]);
        assert_all_eq(&p, &s, "row46 composed valid");
        let s = invalid_run_n(&mut rng, 1, 32);
        assert_all_eq(&p, &s, "row46 composed invalid");
    }
}

/// Feed each implementation's own filter output back into both scanners: the
/// second pass must agree too. This catches divergences that only appear on
/// filtered (already-normalised) data.
#[test]
fn row47_composed_second_pass() {
    let p = pair();
    let mut rng = rng_for(47);
    for _ in 0..3000 {
        let len = rng.range(0, 128);
        let s = biased_bytes(&mut rng, len);
        for m in MODES {
            let mut cin = s.clone();
            cin.push(0);
            let c_out = call_filter(p.c.filter_fn, &cin, m).expect("C filter returned NULL");
            let r_out = call_filter(p.rs.filter_fn, &cin, m).expect("Rust filter returned NULL");
            assert_eq!(c_out, r_out, "row47 first pass mismatch, mode {m}");
            // second pass over the filtered bytes
            assert_drop_eq(&p, &c_out, "row47 second pass drop");
            assert_filter_eq(&p, &c_out, m, "row47 second pass filter");
        }
    }
}
