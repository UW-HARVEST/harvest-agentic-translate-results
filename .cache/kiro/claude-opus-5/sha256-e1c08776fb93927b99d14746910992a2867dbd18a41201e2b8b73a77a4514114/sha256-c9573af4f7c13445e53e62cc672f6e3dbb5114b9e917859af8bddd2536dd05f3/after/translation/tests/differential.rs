//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every row runs many randomized inputs
//! from a fixed seed and compares the C `.so` against every Rust `.so`
//! byte-for-byte through the exported `searchAndReplace` symbol.

mod common;

use common::{all_bytes, assert_same, Rng, AB, ABC, HIGH};

/// Filler alphabet, disjoint from `NEEDLE` so a constructed shape cannot pick up
/// accidental extra matches (the shape a row claims to test is the shape tested).
const FILL: &[u8] = b"ab";
/// Alphabet used for `search` in shape-controlled rows.
const NEEDLE: &[u8] = b"XY";

const CASES: usize = 300;

/// Sanity check on the harness itself: the C `.so` and *both* Rust profiles must
/// be the things actually being compared (a silently-empty implementation list
/// would make every row below vacuously pass).
#[test]
fn row00_harness_loads_both_shared_objects() {
    let c = common::c_impl();
    assert!(c.path.ends_with("c_src/build/libdriver.so"), "{:?}", c.path);
    let names: Vec<&str> = common::rust_impls().iter().map(|i| i.name).collect();
    eprintln!("C  : {}", c.path.display());
    for i in common::rust_impls() {
        eprintln!("{}: {}", i.name, i.path.display());
    }
    assert!(
        names.contains(&"rust-release"),
        "release .so not loaded (run `cargo build --release`), loaded: {names:?}"
    );
    assert!(
        names.contains(&"rust-debug"),
        "debug .so not loaded, loaded: {names:?}"
    );
    // Smoke: the exported symbol really works through both handles.
    assert_eq!(
        common::call(c, b"hello", b"l", b"L"),
        Some(b"heLLo".to_vec())
    );
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

// ---------------------------------------------------------------- rows 1..4:
// A1 — no match at all, `strdup(orig)` early-out.

#[test]
fn row01_no_match_empty_orig() {
    let mut r = Rng::new(0x0101);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 6, ABC);
        let value = r.bytes_range(0, 6, ABC);
        assert_same(b"", &search, &value);
    }
}

#[test]
fn row02_no_match_search_longer_than_orig() {
    let mut r = Rng::new(0x0202);
    for _ in 0..CASES {
        let olen = r.range(0, 5);
        let orig = r.bytes(olen, AB);
        let extra = r.range(1, 4);
        let search = r.bytes(olen + extra, AB);
        let value = r.bytes_range(0, 6, AB);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row03_no_match_search_same_length_as_orig() {
    let mut r = Rng::new(0x0303);
    for _ in 0..CASES {
        let len = r.range(1, 8);
        let orig = r.bytes(len, AB);
        let mut search = r.bytes(len, AB);
        if search == orig {
            let i = r.below(len);
            search[i] = if search[i] == b'a' { b'b' } else { b'a' };
        }
        let value = r.bytes_range(0, 6, AB);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row04_no_match_disjoint_alphabets() {
    let mut r = Rng::new(0x0404);
    for _ in 0..CASES {
        let orig = r.bytes_range(1, 40, FILL);
        let search = r.bytes_range(1, 5, NEEDLE);
        let value = r.bytes_range(0, 6, FILL);
        assert_same(&orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 5..10:
// A2 — single match at offset 0 (prefix `malloc` branch skipped).

fn single_match_at_start(r: &mut Rng, slen: usize, vlen: usize, tail: bool) {
    let search = r.bytes(slen, NEEDLE);
    let tail_b = if tail {
        r.bytes_range(1, 20, FILL)
    } else {
        Vec::new()
    };
    let orig = cat(&[&search, &tail_b]);
    let value = r.bytes(vlen, FILL);
    assert_same(&orig, &search, &value);
}

#[test]
fn row05_match_at_start_tail_empty_value() {
    let mut r = Rng::new(0x0505);
    for _ in 0..CASES {
        let slen = r.range(1, 5);
        single_match_at_start(&mut r, slen, 0, true);
    }
}

#[test]
fn row06_match_at_start_tail_value_shorter() {
    let mut r = Rng::new(0x0606);
    for _ in 0..CASES {
        let slen = r.range(2, 6);
        let vlen = r.range(1, slen - 1);
        single_match_at_start(&mut r, slen, vlen, true);
    }
}

#[test]
fn row07_match_at_start_tail_value_same_length() {
    let mut r = Rng::new(0x0707);
    for _ in 0..CASES {
        let slen = r.range(1, 6);
        single_match_at_start(&mut r, slen, slen, true);
    }
}

#[test]
fn row08_match_at_start_tail_value_longer() {
    let mut r = Rng::new(0x0808);
    for _ in 0..CASES {
        let slen = r.range(1, 5);
        let vlen = slen + r.range(1, 8);
        single_match_at_start(&mut r, slen, vlen, true);
    }
}

#[test]
fn row09_whole_string_matches_value_nonempty() {
    let mut r = Rng::new(0x0909);
    for _ in 0..CASES {
        let slen = r.range(1, 8);
        let vlen = r.range(1, 8);
        single_match_at_start(&mut r, slen, vlen, false);
    }
}

#[test]
fn row10_whole_string_matches_value_empty() {
    let mut r = Rng::new(0x0a0a);
    for _ in 0..CASES {
        let slen = r.range(1, 8);
        single_match_at_start(&mut r, slen, 0, false);
    }
}

// ---------------------------------------------------------------- rows 11..13:
// prefix present (`inx_start > 0`, prefix `malloc` branch taken).

#[test]
fn row11_prefix_single_match_middle_empty_value() {
    let mut r = Rng::new(0x0b0b);
    for _ in 0..CASES {
        let prefix = r.bytes_range(1, 20, FILL);
        let search = r.bytes_range(1, 5, NEEDLE);
        let tail = r.bytes_range(1, 20, FILL);
        let orig = cat(&[&prefix, &search, &tail]);
        assert_same(&orig, &search, b"");
    }
}

#[test]
fn row12_prefix_single_match_middle_value_longer() {
    let mut r = Rng::new(0x0c0c);
    for _ in 0..CASES {
        let prefix = r.bytes_range(1, 20, FILL);
        let search = r.bytes_range(1, 5, NEEDLE);
        let tail = r.bytes_range(1, 20, FILL);
        let orig = cat(&[&prefix, &search, &tail]);
        let extra = r.range(1, 10);
        let value = r.bytes(search.len() + extra, FILL);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row13_prefix_match_at_end_no_tail() {
    let mut r = Rng::new(0x0d0d);
    for _ in 0..CASES {
        let prefix = r.bytes_range(1, 20, FILL);
        let search = r.bytes_range(1, 5, NEEDLE);
        let orig = cat(&[&prefix, &search]);
        let value = r.bytes_range(0, 8, FILL);
        assert_same(&orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 14..16:
// A4 — gap branch present / absent between two matches.

#[test]
fn row14_two_adjacent_matches_no_gap() {
    let mut r = Rng::new(0x0e0e);
    for _ in 0..CASES {
        let prefix = r.bytes_range(0, 10, FILL);
        let search = r.bytes_range(1, 4, NEEDLE);
        let tail = r.bytes_range(0, 10, FILL);
        let orig = cat(&[&prefix, &search, &search, &tail]);
        let value = r.bytes_range(0, 8, FILL);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row15_two_matches_with_gap() {
    let mut r = Rng::new(0x0f0f);
    for _ in 0..CASES {
        let prefix = r.bytes_range(0, 10, FILL);
        let search = r.bytes_range(1, 4, NEEDLE);
        let gap = r.bytes_range(1, 10, FILL);
        let tail = r.bytes_range(0, 10, FILL);
        let orig = cat(&[&prefix, &search, &gap, &search, &tail]);
        let value = r.bytes_range(0, 8, FILL);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row16_matches_at_both_ends() {
    let mut r = Rng::new(0x1010);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 4, NEEDLE);
        let gap = r.bytes_range(0, 12, FILL);
        let orig = cat(&[&search, &gap, &search]);
        let value = r.bytes_range(0, 8, FILL);
        assert_same(&orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 17:
// A8 — overlapping occurrences must not be re-matched.

#[test]
fn row17_overlapping_occurrences() {
    let mut r = Rng::new(0x1111);
    for _ in 0..CASES {
        let mut orig = Vec::new();
        let segments = r.range(1, 6);
        for _ in 0..segments {
            let run = r.range(1, 12);
            orig.extend(std::iter::repeat(b'a').take(run));
            let sep = r.range(0, 3);
            orig.extend(std::iter::repeat(b'b').take(sep));
        }
        let slen = r.range(2, 3);
        let search: Vec<u8> = std::iter::repeat(b'a').take(slen).collect();
        let value = r.bytes_range(0, 5, b"aXb");
        assert_same(&orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 18..21:
// A3/A6/A11 — many matches, tiny search/value, self-referential value.

fn many_matches(r: &mut Rng, occurrences: usize, search: &[u8], allow_zero_gap: bool) -> Vec<u8> {
    let mut orig = r.bytes_range(0, 8, FILL);
    for i in 0..occurrences {
        orig.extend_from_slice(search);
        if i + 1 < occurrences {
            let lo = if allow_zero_gap { 0 } else { 1 };
            let g = r.range(lo, 6);
            orig.extend(r.bytes(g, FILL));
        }
    }
    orig.extend(r.bytes_range(0, 8, FILL));
    orig
}

#[test]
fn row18_many_matches_random_gaps() {
    let mut r = Rng::new(0x1212);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 4, NEEDLE);
        let n = r.range(8, 200);
        let orig = many_matches(&mut r, n, &search, true);
        let value = r.bytes_range(0, 6, FILL);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row19_single_byte_search_dense() {
    let mut r = Rng::new(0x1313);
    for _ in 0..CASES {
        let orig = r.bytes_range(0, 40, AB);
        let search = b"a".to_vec();
        let value = r.bytes_range(0, 4, b"abX");
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row20_single_byte_value_many_matches() {
    let mut r = Rng::new(0x1414);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 3, NEEDLE);
        let n = r.range(8, 60);
        let orig = many_matches(&mut r, n, &search, true);
        assert_same(&orig, &search, b"z");
    }
}

#[test]
fn row21_value_contains_search() {
    let mut r = Rng::new(0x1515);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 4, NEEDLE);
        let n = r.range(2, 12);
        let orig = many_matches(&mut r, n, &search, true);
        let value = cat(&[
            &r.bytes_range(0, 3, FILL),
            &search,
            &r.bytes_range(0, 3, FILL),
        ]);
        assert_same(&orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 22..24:
// A10/A9 — byte range and size.

#[test]
fn row22_high_bytes_exhaustive() {
    // Exhaustive over a 2-letter high-byte alphabet: guards against any
    // signed-`char` divergence between C `strstr`/`strncpy` and the Rust `u8`
    // comparisons, for every shape up to 6 bytes.
    const HI2: &[u8] = &[0x80, 0xff];
    let values: [&[u8]; 4] = [b"", &[0x80], &[0xff, 0x7f], &[0x80, 0x80, 0xff]];
    for orig in all_strings(6, HI2) {
        for search in all_strings(3, HI2) {
            if search.is_empty() {
                continue;
            }
            for value in values {
                assert_same(&orig, &search, value);
            }
        }
    }
}

#[test]
fn row22_high_bytes_only() {
    let mut r = Rng::new(0x1616);
    for _ in 0..CASES {
        let orig = r.bytes_range(0, 40, HIGH);
        let search = r.bytes_range(1, 4, HIGH);
        let value = r.bytes_range(0, 6, HIGH);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row23_full_byte_alphabet() {
    let all = all_bytes();
    let mut r = Rng::new(0x1717);
    for _ in 0..CASES {
        let orig = r.bytes_range(0, 64, &all);
        // Bias towards actually hitting matches half the time by sometimes
        // lifting `search` out of `orig`.
        let search = if !orig.is_empty() && r.below(2) == 0 {
            let slen = r.range(1, 3.min(orig.len()));
            let at = r.below(orig.len() - slen + 1);
            orig[at..at + slen].to_vec()
        } else {
            r.bytes_range(1, 3, &all)
        };
        let value = r.bytes_range(0, 8, &all);
        assert_same(&orig, &search, &value);
    }
}

#[test]
fn row24_large_input_many_reallocs() {
    let mut r = Rng::new(0x1818);
    for _ in 0..12 {
        let len = r.range(4 * 1024, 64 * 1024);
        let orig = r.bytes(len, ABC);
        let search = r.bytes_range(1, 3, ABC);
        let value = r.bytes_range(0, 6, ABC);
        assert_same(&orig, &search, &value);
    }
    // Large *replacement*: the buffer grows by value_len on every match, so this
    // is the opposite growth pattern from the case above.
    let mut r = Rng::new(0x18f8);
    for _ in 0..6 {
        let orig = r.bytes_range(200, 2000, AB);
        let search = r.bytes_range(1, 2, AB);
        let value = r.bytes_range(2048, 8192, ABC);
        assert_same(&orig, &search, &value);
    }
}

/// Unbounded random differential soak over the full byte alphabet. Runs a modest
/// number of cases by default; `SOAK_ITERS=500000 cargo test soak -- --nocapture`
/// turns it into a long fuzz run. Seed is fixed unless `SOAK_SEED` is given.
#[test]
fn soak_random_fuzz() {
    let iters: usize = std::env::var("SOAK_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    let seed: u64 = std::env::var("SOAK_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5_0A_C0_DE_u64);
    let all = all_bytes();
    let mut r = Rng::new(seed);
    for _ in 0..iters {
        // Mix alphabets so that both "matches are rare" and "matches are dense"
        // regimes are hit.
        let alpha: &[u8] = match r.below(4) {
            0 => AB,
            1 => ABC,
            2 => HIGH,
            _ => &all,
        };
        let orig = r.bytes_range(0, 80, alpha);
        let search = if !orig.is_empty() && r.below(2) == 0 {
            let slen = r.range(1, std::cmp::min(6, orig.len()));
            let at = r.below(orig.len() - slen + 1);
            orig[at..at + slen].to_vec()
        } else {
            r.bytes_range(1, 6, alpha)
        };
        let value = r.bytes_range(0, 10, alpha);
        assert_same(&orig, &search, &value);
    }
    eprintln!("soak: {iters} random cases, seed {seed}");
}

// ---------------------------------------------------------------- rows 25..27:
// exhaustive boundary sweeps.

fn all_strings(max_len: usize, alphabet: &[u8]) -> Vec<Vec<u8>> {
    let mut out = vec![Vec::new()];
    let mut level = vec![Vec::<u8>::new()];
    for _ in 0..max_len {
        let mut next = Vec::new();
        for s in &level {
            for &c in alphabet {
                let mut t = s.clone();
                t.push(c);
                next.push(t);
            }
        }
        out.extend(next.iter().cloned());
        level = next;
    }
    out
}

#[test]
fn row25_tiny_orig_boundary_sweep() {
    let values: [&[u8]; 3] = [b"", b"a", b"XY"];
    for orig in all_strings(1, AB) {
        for search in all_strings(3, AB) {
            if search.is_empty() {
                continue; // empty search never terminates — see ERRORS.md rows 10/11
            }
            for value in values {
                assert_same(&orig, &search, value);
            }
        }
    }
}

#[test]
fn row26_empty_value_many_matches_no_prefix() {
    let mut r = Rng::new(0x1a1a);
    for _ in 0..CASES {
        let search = r.bytes_range(1, 4, NEEDLE);
        let n = r.range(2, 40);
        let mut orig = Vec::new();
        for i in 0..n {
            orig.extend_from_slice(&search);
            if i + 1 < n {
                orig.extend(r.bytes_range(0, 5, FILL));
            }
        }
        assert_same(&orig, &search, b"");
    }
}

#[test]
fn row27_exhaustive_small_cross_product() {
    // Exhaustive over the 2-letter alphabet: every `orig` up to length 8, every
    // `search` of length 1..3, and values that are empty / shorter / longer /
    // themselves made of the search alphabet. This brute-forces every
    // combination of the A1..A8 branches that fits in 8 bytes.
    let values: [&[u8]; 5] = [b"", b"a", b"XY", b"ab", b"aaa"];
    let origs = all_strings(8, AB);
    let searches: Vec<Vec<u8>> = all_strings(3, AB)
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    let mut n = 0usize;
    for orig in &origs {
        for search in &searches {
            for value in values {
                assert_same(orig, search, value);
                n += 1;
            }
        }
    }
    assert!(n >= 30_000, "expected a broad cross-product, ran {n}");
}
