//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported
//! `searchAndReplace` symbol with many randomized inputs (fixed seed) and
//! requires byte-identical results.

mod common;

use common::{assert_same, Rng, AB, ABC};

/// Needle alphabet, disjoint from the filler alphabet, so that the structural
/// rows (prefix / gap / tail / adjacency) really have the match layout they
/// claim: a filler run can never accidentally contain the needle.
const NEEDLE: &[u8] = b"XY";
const FILL: &[u8] = b"abc";

/// Concatenate pieces.
fn cat(pieces: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in pieces {
        v.extend_from_slice(p);
    }
    v
}

// ---------------------------------------------------------------- C1..C3: no match

#[test]
fn c1_no_match_random() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..2000 {
        let orig = rng.bytes_range(1, 64, FILL);
        let search = rng.bytes_range(1, 8, NEEDLE);
        let value = rng.bytes_range(0, 8, b"abcXY");
        let out = assert_same("C1", &orig, &search, &value);
        assert_eq!(out.as_deref(), Some(&orig[..]), "C1: expected strdup(orig)");
    }
}

#[test]
fn c2_no_match_needle_longer() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..2000 {
        let orig = rng.any_bytes_range(0, 16);
        // needle strictly longer than the haystack -> strstr() cannot match
        let mut search = orig.clone();
        search.extend_from_slice(&rng.any_bytes_range(1, 8));
        let value = rng.any_bytes_range(0, 8);
        let out = assert_same("C2", &orig, &search, &value);
        assert_eq!(out.as_deref(), Some(&orig[..]));
    }
}

#[test]
fn c3_no_match_empty_orig() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..1000 {
        let search = rng.any_bytes_range(1, 8);
        let value = rng.any_bytes_range(0, 8);
        let out = assert_same("C3", b"", &search, &value);
        assert_eq!(out.as_deref(), Some(&b""[..]));
    }
}

// ------------------------------------------- C4..C9: single match, index 0

fn single_at_start(rng: &mut Rng, value_len: impl Fn(&mut Rng, usize) -> usize, tail: bool) {
    for _ in 0..1500 {
        let k = rng.range(1, 8);
        let needle = rng.bytes(k, NEEDLE);
        let t = if tail {
            rng.bytes_range(1, 32, FILL)
        } else {
            Vec::new()
        };
        let orig = cat(&[&needle, &t]);
        let vl = value_len(rng, k);
        let value = rng.bytes(vl, FILL);
        assert_same("C4-C9", &orig, &needle, &value);
    }
}

#[test]
fn c4_single_at_start_tail_same_len() {
    let mut rng = Rng::new(0xC004);
    single_at_start(&mut rng, |_r, k| k, true);
}

#[test]
fn c5_single_at_start_tail_grow() {
    let mut rng = Rng::new(0xC005);
    single_at_start(&mut rng, |r, k| r.range(k + 1, k + 64), true);
}

#[test]
fn c6_single_at_start_tail_shrink() {
    let mut rng = Rng::new(0xC006);
    for _ in 0..1500 {
        let k = rng.range(4, 8);
        let needle = rng.bytes(k, NEEDLE);
        let tail = rng.bytes_range(1, 32, FILL);
        let orig = cat(&[&needle, &tail]);
        let value = rng.bytes_range(1, k - 1, FILL);
        assert_same("C6", &orig, &needle, &value);
    }
}

#[test]
fn c7_single_at_start_tail_delete() {
    let mut rng = Rng::new(0xC007);
    single_at_start(&mut rng, |_r, _k| 0, true);
}

#[test]
fn c8_whole_string_match() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..1500 {
        let needle = rng.bytes_range(1, 16, NEEDLE);
        let value = rng.bytes_range(1, 16, FILL);
        let out = assert_same("C8", &needle, &needle, &value);
        assert_eq!(out.as_deref(), Some(&value[..]));
    }
}

#[test]
fn c9_whole_string_match_delete() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..500 {
        let needle = rng.bytes_range(1, 16, NEEDLE);
        let out = assert_same("C9", &needle, &needle, b"");
        assert_eq!(out.as_deref(), Some(&b""[..]));
    }
}

// --------------------------------------- C10..C11: prefix present, one match

#[test]
fn c10_prefix_single_tail() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 8, NEEDLE);
        let prefix = rng.bytes_range(1, 32, FILL);
        let tail = rng.bytes_range(1, 32, FILL);
        let orig = cat(&[&prefix, &needle, &tail]);
        let value = rng.bytes_range(0, 32, FILL);
        let out = assert_same("C10", &orig, &needle, &value);
        assert_eq!(out.as_deref(), Some(&cat(&[&prefix, &value, &tail])[..]));
    }
}

#[test]
fn c11_prefix_single_no_tail() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 8, NEEDLE);
        let prefix = rng.bytes_range(1, 32, FILL);
        let orig = cat(&[&prefix, &needle]);
        let value = rng.bytes_range(0, 32, FILL);
        let out = assert_same("C11", &orig, &needle, &value);
        assert_eq!(out.as_deref(), Some(&cat(&[&prefix, &value])[..]));
    }
}

// ------------------------------------------------ C12..C16: multiple matches

#[test]
fn c12_two_matches_with_gap() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let prefix = rng.bytes_range(0, 16, FILL);
        let gap = rng.bytes_range(1, 16, FILL);
        let tail = rng.bytes_range(0, 16, FILL);
        let orig = cat(&[&prefix, &needle, &gap, &needle, &tail]);
        let value = rng.bytes_range(0, 16, FILL);
        assert_same("C12", &orig, &needle, &value);
    }
}

#[test]
fn c13_two_matches_adjacent() {
    let mut rng = Rng::new(0xC013);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let prefix = rng.bytes_range(0, 16, FILL);
        let tail = rng.bytes_range(0, 16, FILL);
        let orig = cat(&[&prefix, &needle, &needle, &tail]);
        let value = rng.bytes_range(0, 16, FILL);
        assert_same("C13", &orig, &needle, &value);
    }
}

#[test]
fn c14_two_matches_start_and_end() {
    let mut rng = Rng::new(0xC014);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let gap = rng.bytes_range(0, 16, FILL);
        let orig = cat(&[&needle, &gap, &needle]);
        let value = rng.bytes_range(0, 16, FILL);
        assert_same("C14", &orig, &needle, &value);
    }
}

#[test]
fn c15_many_adjacent_matches() {
    let mut rng = Rng::new(0xC015);
    for _ in 0..800 {
        let needle = rng.bytes_range(1, 4, NEEDLE);
        let n = rng.range(8, 24);
        let mut orig = rng.bytes_range(0, 8, FILL);
        for _ in 0..n {
            orig.extend_from_slice(&needle);
        }
        orig.extend_from_slice(&rng.bytes_range(0, 8, FILL));
        let value = rng.bytes_range(0, 12, FILL);
        assert_same("C15", &orig, &needle, &value);
    }
}

#[test]
fn c16_many_random_gaps() {
    let mut rng = Rng::new(0xC016);
    for _ in 0..1500 {
        let needle = rng.bytes_range(1, 5, NEEDLE);
        let n = rng.range(2, 16);
        let mut orig = rng.bytes_range(0, 12, FILL);
        for _ in 0..n {
            orig.extend_from_slice(&needle);
            orig.extend_from_slice(&rng.bytes_range(0, 10, FILL));
        }
        let value = rng.bytes_range(0, 12, FILL);
        assert_same("C16", &orig, &needle, &value);
    }
}

// ---------------------------------------------- C17..C18: overlapping matches

#[test]
fn c17_overlapping_even_runs() {
    let mut rng = Rng::new(0xC017);
    for _ in 0..1500 {
        let runs = rng.range(1, 6);
        let mut orig = Vec::new();
        for i in 0..runs {
            if i > 0 {
                orig.extend_from_slice(&rng.bytes_range(1, 4, b"b"));
            }
            orig.extend(std::iter::repeat(b'a').take(2 * rng.range(1, 12)));
        }
        let value = rng.bytes_range(0, 4, b"bc");
        assert_same("C17", &orig, b"aa", &value);
    }
}

#[test]
fn c18_overlapping_odd_runs() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..1500 {
        let runs = rng.range(1, 6);
        let mut orig = Vec::new();
        for i in 0..runs {
            if i > 0 {
                orig.extend_from_slice(&rng.bytes_range(1, 4, b"b"));
            }
            orig.extend(std::iter::repeat(b'a').take(2 * rng.range(1, 12) + 1));
        }
        let value = rng.bytes_range(0, 4, b"bc");
        assert_same("C18", &orig, b"aa", &value);
        assert_same("C18", &orig, b"aaa", &value);
    }
}

// ------------------------------------------------- C19..C20: needle length

#[test]
fn c19_single_byte_needle() {
    let mut rng = Rng::new(0xC019);
    for _ in 0..4000 {
        let orig = rng.bytes_range(1, 64, AB);
        let value = rng.bytes_range(0, 6, ABC);
        assert_same("C19", &orig, b"a", &value);
        assert_same("C19", &orig, b"b", &value);
    }
}

#[test]
fn c20_long_needle() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..1500 {
        let needle = rng.bytes_range(8, 24, NEEDLE);
        let n = rng.range(1, 3);
        let mut orig = rng.bytes_range(0, 24, FILL);
        for _ in 0..n {
            orig.extend_from_slice(&needle);
            orig.extend_from_slice(&rng.bytes_range(0, 24, FILL));
        }
        let value = rng.bytes_range(0, 24, FILL);
        assert_same("C20", &orig, &needle, &value);
    }
}

// --------------------------------------- C21..C22: value related to search

#[test]
fn c21_value_contains_search() {
    let mut rng = Rng::new(0xC021);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 4, NEEDLE);
        let value = cat(&[
            &rng.bytes_range(0, 4, FILL),
            &needle,
            &rng.bytes_range(0, 4, FILL),
        ]);
        let n = rng.range(1, 6);
        let mut orig = rng.bytes_range(0, 8, FILL);
        for _ in 0..n {
            orig.extend_from_slice(&needle);
            orig.extend_from_slice(&rng.bytes_range(0, 6, FILL));
        }
        assert_same("C21", &orig, &needle, &value);
    }
}

#[test]
fn c22_value_equals_search() {
    let mut rng = Rng::new(0xC022);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let n = rng.range(1, 8);
        let mut orig = rng.bytes_range(0, 8, FILL);
        for _ in 0..n {
            orig.extend_from_slice(&needle);
            orig.extend_from_slice(&rng.bytes_range(0, 6, FILL));
        }
        let out = assert_same("C22", &orig, &needle, &needle);
        assert_eq!(out.as_deref(), Some(&orig[..]), "C22: identity replace");
    }
}

// ------------------------------------------------------ C23: byte domain

#[test]
fn c23_high_bit_bytes() {
    let mut rng = Rng::new(0xC023);
    let high: Vec<u8> = (0x80u8..=0xffu8).collect();
    for _ in 0..3000 {
        let orig = rng.bytes_range(0, 48, &high);
        let search = if !orig.is_empty() && rng.bool() {
            // a real substring of `orig`, so matches actually occur
            let len = rng.range(1, orig.len().min(4));
            let at = rng.range(0, orig.len() - len);
            orig[at..at + len].to_vec()
        } else {
            rng.bytes_range(1, 4, &high)
        };
        let value = rng.bytes_range(0, 8, &high);
        assert_same("C23", &orig, &search, &value);
    }
}

// ------------------------------------------- C24..C25: size extremes

#[test]
fn c24_exhaustive_tiny_shapes() {
    // Exhaustive over the 2-byte alphabet {a,b}:
    //   |orig| in 0..=3, |search| in 1..=3, |value| in 0..=2
    fn all(alpha: &[u8], len: usize) -> Vec<Vec<u8>> {
        if len == 0 {
            return vec![Vec::new()];
        }
        let mut out = Vec::new();
        for rest in all(alpha, len - 1) {
            for &c in alpha {
                let mut v = vec![c];
                v.extend_from_slice(&rest);
                out.push(v);
            }
        }
        out
    }
    let origs: Vec<Vec<u8>> = (0..=3).flat_map(|l| all(AB, l)).collect();
    let searches: Vec<Vec<u8>> = (1..=3).flat_map(|l| all(AB, l)).collect();
    let values: Vec<Vec<u8>> = (0..=2).flat_map(|l| all(AB, l)).collect();
    let mut n = 0usize;
    for o in &origs {
        for s in &searches {
            for v in &values {
                assert_same("C24", o, s, v);
                n += 1;
            }
        }
    }
    assert_eq!(n, origs.len() * searches.len() * values.len());
    assert!(n >= 1400, "expected the full tiny cross-product, got {n}");
}

#[test]
fn c25_large_input_many_matches() {
    let mut rng = Rng::new(0xC025);
    for _ in 0..6 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let mut orig = Vec::with_capacity(70_000);
        while orig.len() < 64 * 1024 {
            orig.extend_from_slice(&rng.bytes_range(0, 40, FILL));
            orig.extend_from_slice(&needle);
        }
        orig.extend_from_slice(&rng.bytes_range(0, 40, FILL));
        let value = rng.bytes_range(0, 32, FILL);
        assert_same("C25", &orig, &needle, &value);
    }
}

// -------------------------------------- C26..C28: combined guard coverage

#[test]
fn c26_prefix_mixed_gaps_no_tail() {
    let mut rng = Rng::new(0xC026);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 5, NEEDLE);
        let mut orig = rng.bytes_range(1, 16, FILL); // prefix > 0
        let n = rng.range(3, 10);
        for i in 0..n {
            orig.extend_from_slice(&needle);
            if i + 1 < n {
                // sometimes adjacent (gap == 0), sometimes separated
                orig.extend_from_slice(&rng.bytes_range(0, 8, FILL));
            }
        }
        // ends exactly with a needle -> no tail copy
        let value = rng.bytes_range(0, 10, FILL);
        assert_same("C26", &orig, &needle, &value);
    }
}

#[test]
fn c27_partial_match_prefixes() {
    let mut rng = Rng::new(0xC027);
    let needles: [&[u8]; 6] = [b"aa", b"aaa", b"aab", b"aba", b"abab", b"ba"];
    for _ in 0..4000 {
        let orig = rng.bytes_range(1, 48, AB);
        let needle = *rng.pick(&needles);
        let value = rng.bytes_range(0, 5, ABC);
        assert_same("C27", &orig, needle, &value);
    }
}

#[test]
fn c28_one_byte_tail() {
    let mut rng = Rng::new(0xC028);
    for _ in 0..2000 {
        let needle = rng.bytes_range(1, 6, NEEDLE);
        let prefix = rng.bytes_range(0, 16, FILL);
        let tail = rng.bytes(1, FILL); // from == orig_len - 1
        let orig = cat(&[&prefix, &needle, &tail]);
        let value = rng.bytes_range(0, 16, FILL);
        let out = assert_same("C28", &orig, &needle, &value);
        assert_eq!(out.as_deref(), Some(&cat(&[&prefix, &value, &tail])[..]));
    }
}

// ------------------------------------------------- C29..C30: property sweeps

#[test]
fn c29_property_sweep_small_alphabet() {
    let mut rng = Rng::new(0xC029);
    for _ in 0..20_000 {
        let orig = rng.bytes_range(0, 40, AB);
        let search = rng.bytes_range(1, 3, AB);
        let value = rng.bytes_range(0, 4, ABC);
        assert_same("C29", &orig, &search, &value);
    }
}

#[test]
fn c30_property_sweep_full_byte_range() {
    let mut rng = Rng::new(0xC030);
    for _ in 0..20_000 {
        let orig = rng.any_bytes_range(0, 48);
        let search = if !orig.is_empty() && rng.range(0, 2) != 0 {
            let len = rng.range(1, orig.len().min(4));
            let at = rng.range(0, orig.len() - len);
            orig[at..at + len].to_vec()
        } else {
            rng.any_bytes_range(1, 4)
        };
        let value = rng.any_bytes_range(0, 6);
        assert_same("C30", &orig, &search, &value);
    }
}
