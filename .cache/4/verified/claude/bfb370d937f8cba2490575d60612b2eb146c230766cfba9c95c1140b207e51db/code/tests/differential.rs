//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`, each driven with many randomized
//! inputs from a fixed seed. Both implementations are invoked through their
//! `.so` exports (see `tests/harness/mod.rs`).

mod harness;

use harness::*;

/// Build a haystack with an exact, known number of `search` occurrences:
/// `filler[0] search filler[1] search ... search filler[n]`.
/// The filler alphabet is disjoint from the search alphabet, so no accidental
/// occurrences can appear.
fn compose(rng: &mut Rng, search: &[u8], fillers: &[usize]) -> Vec<u8> {
    assert!(!fillers.is_empty());
    let mut out = Vec::new();
    for (i, &f) in fillers.iter().enumerate() {
        out.extend_from_slice(&rng.bytes(f, FILL));
        if i + 1 < fillers.len() {
            out.extend_from_slice(search);
        }
    }
    out
}

fn rnd_search(rng: &mut Rng) -> Vec<u8> {
    let n = rng.range(1, 4);
    rng.bytes(n, SEARCH)
}

fn rnd_value(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(9);
    rng.bytes(n, VALUE)
}

// ------------------------------------------------- harness self-check
/// Sanity: the two `.so`s really are two distinct libraries (different symbol
/// addresses) and both produce the documented behaviour, so a divergence really
/// would be observable.
#[test]
fn row00_harness_selfcheck() {
    let (cf, rf) = fns();
    assert_ne!(
        cf as usize, rf as usize,
        "C and Rust symbols resolved to the SAME address — the two libraries are not both loaded"
    );
    let c = unsafe { call(cf, b"hello world", b"world", b"there") };
    let r = unsafe { call(rf, b"hello world", b"world", b"there") };
    assert_eq!(c.bytes, b"hello there".to_vec());
    assert_eq!(r.bytes, b"hello there".to_vec());
    // a hand-computed multi-match case
    let c2 = unsafe { call(cf, b"aXbXXc", b"X", b"--") };
    let r2 = unsafe { call(rf, b"aXbXXc", b"X", b"--") };
    assert_eq!(c2.bytes, b"a--b----c".to_vec());
    assert_eq!(r2.bytes, c2.bytes);
    // negative control: the comparison really distinguishes outputs
    let c3 = unsafe { call(cf, b"aXb", b"X", b"Y") };
    assert_ne!(c3.bytes, c.bytes);
}

// ---------------------------------------------------------------- row 1
#[test]
fn row01_no_match_nonempty() {
    let mut rng = Rng::new(1);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let orig = rng.bytes_r(1, 64, FILL);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 0);
        let out = check("row01", &orig, &search, &value);
        assert!(!out.null);
        assert_eq!(out.bytes, orig, "no-match path must return a copy of orig");
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_empty_orig() {
    let mut rng = Rng::new(2);
    for _ in 0..500 {
        let search = rnd_search(&mut rng);
        let value = rnd_value(&mut rng);
        let out = check("row02", b"", &search, &value);
        assert!(!out.null);
        assert!(out.bytes.is_empty());
    }
    // also the fully-empty orig with a 1-byte search / empty value
    check("row02b", b"", b"X", b"");
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_search_longer_than_orig() {
    let mut rng = Rng::new(3);
    for _ in 0..1000 {
        let orig = rng.bytes_b(17, FILL);
        // search = orig + 1..3 extra bytes  => strictly longer, cannot match
        let mut search = orig.clone();
        search.extend_from_slice(&rng.bytes_r(1, 3, FILL));
        let value = rnd_value(&mut rng);
        let out = check("row03", &orig, &search, &value);
        assert!(!out.null);
        assert_eq!(out.bytes, orig);
        // and a completely unrelated, longer search
        let slen = orig.len() + rng.range(1, 5);
        let s2 = rng.bytes(slen, SEARCH);
        check("row03b", &orig, &s2, &value);
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_one_match_at_zero_with_tail() {
    let mut rng = Rng::new(4);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let t = rng.range(1, 24);
        let orig = compose(&mut rng, &search, &[0, t]);
        let value = rng.bytes_r(1, 8, VALUE);
        assert_eq!(count_matches(&orig, &search), 1);
        assert_eq!(first_match(&orig, &search), Some(0));
        check("row04", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_search_equals_orig() {
    let mut rng = Rng::new(5);
    for _ in 0..1000 {
        let search = rng.bytes_r(1, 8, SEARCH);
        let value = rnd_value(&mut rng);
        check("row05", &search.clone(), &search, &value);
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_one_match_prefix_and_tail() {
    let mut rng = Rng::new(6);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let (p0, t0) = (rng.range(1, 20), rng.range(1, 20));
        let orig = compose(&mut rng, &search, &[p0, t0]);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 1);
        check("row06", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_one_match_at_end() {
    let mut rng = Rng::new(7);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let p0 = rng.range(1, 20);
        let orig = compose(&mut rng, &search, &[p0, 0]);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 1);
        assert!(orig.ends_with(&search));
        check("row07", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_empty_value_deletion() {
    let mut rng = Rng::new(8);
    for _ in 0..1500 {
        let search = rnd_search(&mut rng);
        for shape in 0..3 {
            let (a, b) = (rng.range(1, 16), rng.range(1, 16));
            let fillers = match shape {
                0 => vec![0, a],    // at offset 0, tail
                1 => vec![a, b],    // middle
                _ => vec![a, 0],    // at the end
            };
            let orig = compose(&mut rng, &search, &fillers);
            check("row08", &orig, &search, b"");
        }
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_two_adjacent_matches_at_zero() {
    let mut rng = Rng::new(9);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let t = rng.below(16);
        let orig = compose(&mut rng, &search, &[0, 0, t]);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 2);
        check("row09", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_two_matches_with_gap() {
    let mut rng = Rng::new(10);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let (p0, g0, t0) = (rng.below(12), rng.range(1, 12), rng.below(12));
        let orig = compose(&mut rng, &search, &[p0, g0, t0]);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 2);
        check("row10", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_two_matches_second_at_end() {
    let mut rng = Rng::new(11);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let (p0, g0) = (rng.below(12), rng.range(1, 12));
        let orig = compose(&mut rng, &search, &[p0, g0, 0]);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), 2);
        assert!(orig.ends_with(&search));
        check("row11", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_many_matches_random_gaps() {
    let mut rng = Rng::new(12);
    for _ in 0..3000 {
        let search = rnd_search(&mut rng);
        let n = rng.range(5, 12);
        let fillers: Vec<usize> = (0..=n).map(|_| rng.below(7)).collect();
        let orig = compose(&mut rng, &search, &fillers);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), n);
        check("row12", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_orig_is_search_repeated() {
    let mut rng = Rng::new(13);
    for _ in 0..1000 {
        let search = rnd_search(&mut rng);
        let k = rng.range(1, 8);
        let orig = compose(&mut rng, &search, &vec![0usize; k + 1]);
        let value = rnd_value(&mut rng);
        assert_eq!(orig.len(), k * search.len());
        check("row13", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 14/15
#[test]
fn row14_overlapping_even() {
    let mut rng = Rng::new(14);
    for n in 1..=12usize {
        for _ in 0..40 {
            let c = rng.pick(FILL);
            let search = vec![c, c];
            let orig = vec![c; 2 * n];
            let value = rnd_value(&mut rng);
            check("row14", &orig, &search, &value);
        }
    }
}

#[test]
fn row15_overlapping_odd() {
    let mut rng = Rng::new(15);
    for n in 1..=12usize {
        for _ in 0..40 {
            let c = rng.pick(FILL);
            let search = vec![c, c];
            let orig = vec![c; 2 * n + 1];
            let value = rnd_value(&mut rng);
            check("row15", &orig, &search, &value);
        }
        // odd-length runs of 3 with a 3-byte search, and 2-byte search on 3-run
        let c = rng.pick(FILL);
        check("row15b", &vec![c; 3 * n + 1], &vec![c; 3], &rnd_value(&mut rng));
        check("row15c", &vec![c; 3 * n + 2], &vec![c; 3], &rnd_value(&mut rng));
    }
}

// ---------------------------------------------------------------- rows 16/17/18
fn value_len_row(row: &str, seed: u64, vlen: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..1500 {
        let search = rng.bytes(3, SEARCH);
        let n = rng.range(1, 8);
        let fillers: Vec<usize> = (0..=n).map(|_| rng.below(6)).collect();
        let orig = compose(&mut rng, &search, &fillers);
        let value = rng.bytes(vlen, VALUE);
        assert_eq!(count_matches(&orig, &search), n);
        check(row, &orig, &search, &value);
    }
}

#[test]
fn row16_value_longer_than_search() {
    value_len_row("row16", 16, 5);
    value_len_row("row16b", 160, 9);
}

#[test]
fn row17_value_shorter_than_search() {
    value_len_row("row17", 17, 1);
    value_len_row("row17b", 170, 2);
}

#[test]
fn row18_value_same_length_as_search() {
    value_len_row("row18", 18, 3);
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_value_contains_search() {
    let mut rng = Rng::new(19);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let mut value = rng.bytes_b(4, VALUE);
        value.extend_from_slice(&search);
        value.extend_from_slice(&rng.bytes_b(4, VALUE));
        let n = rng.range(1, 6);
        let fillers: Vec<usize> = (0..=n).map(|_| rng.below(6)).collect();
        let orig = compose(&mut rng, &search, &fillers);
        check("row19", &orig, &search, &value);
        // value == search exactly (identity), and value = search+search
        check("row19b", &orig, &search, &search.clone());
        let mut dbl = search.clone();
        dbl.extend_from_slice(&search);
        check("row19c", &orig, &search, &dbl);
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn row20_dense_single_byte_search() {
    let mut rng = Rng::new(20);
    let alpha = b"ab";
    for _ in 0..5000 {
        let orig = rng.bytes_b(81, alpha);
        let search = vec![rng.pick(alpha)];
        let value = rng.bytes_b(5, b"abZ");
        check("row20", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn row21_dense_multibyte_search() {
    let mut rng = Rng::new(21);
    for _ in 0..5000 {
        let asize = rng.range(2, 4);
        let alpha = &b"abcd"[..asize];
        let orig = rng.bytes_b(121, alpha);
        let search = rng.bytes_r(2, 8, alpha);
        let value = rng.bytes_b(6, b"abZ9");
        check("row21", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 22
#[test]
fn row22_high_and_full_byte_range() {
    let mut rng = Rng::new(22);
    let high: Vec<u8> = (0x80u8..=0xff).collect();
    let full: Vec<u8> = (0x01u8..=0xff).collect();
    for _ in 0..2000 {
        // narrow high-byte alphabet -> dense matches with non-UTF8 bytes
        let alpha: Vec<u8> = (0..3).map(|_| rng.pick(&high)).collect();
        let orig = rng.bytes_b(60, &alpha);
        let search = rng.bytes_r(1, 3, &alpha);
        let value = rng.bytes_b(5, &alpha);
        check("row22", &orig, &search, &value);
        // wide alphabet over the whole non-NUL byte range (mostly no-match)
        let orig2 = rng.bytes_b(60, &full);
        let search2 = rng.bytes_r(1, 2, &full);
        let value2 = rng.bytes_b(5, &full);
        check("row22b", &orig2, &search2, &value2);
    }
}

// ---------------------------------------------------------------- row 23
#[test]
fn row23_long_inputs() {
    let mut rng = Rng::new(23);
    for _ in 0..10 {
        let alpha = b"ab";
        let len = rng.range(4096, 16384);
        let orig = rng.bytes(len, alpha);
        let search = rng.bytes_r(1, 3, alpha);
        let value = rng.bytes_r(0, 1024, b"VW");
        assert!(count_matches(&orig, &search) > 50);
        check("row23", &orig, &search, &value);
    }
    // one long input with exactly one match in the middle, huge value
    let mut orig = rng.bytes(8000, FILL);
    orig.extend_from_slice(b"XY");
    orig.extend_from_slice(&rng.bytes(8000, FILL));
    check("row23b", &orig, b"XY", &rng.bytes(4096, VALUE));
    // long input, match at the very end / very start
    let mut o2 = rng.bytes(8000, FILL);
    o2.extend_from_slice(b"XY");
    check("row23c", &o2, b"XY", &rng.bytes(100, VALUE));
    let mut o3 = b"XY".to_vec();
    o3.extend_from_slice(&rng.bytes(8000, FILL));
    check("row23d", &o3, b"XY", &rng.bytes(100, VALUE));
}

// ---------------------------------------------------------------- row 24
#[test]
fn row24_single_byte_inputs() {
    let alpha = b"ab";
    for &o in alpha {
        for &s in alpha {
            for v in [b"".to_vec(), b"z".to_vec(), b"zz".to_vec(), b"a".to_vec()] {
                check("row24", &[o], &[s], &v);
            }
        }
    }
}

// ---------------------------------------------------------------- row 25
#[test]
fn row25_identity_and_empty_result() {
    let mut rng = Rng::new(25);
    for _ in 0..500 {
        let s = rng.bytes_r(1, 10, SEARCH);
        check("row25a", &s.clone(), &s, &s.clone()); // orig == search == value
        check("row25b", &s.clone(), &s, b""); // whole string deleted -> ""
        let mut two = s.clone();
        two.extend_from_slice(&s);
        check("row25c", &two, &s, b""); // both copies deleted -> ""
    }
}

// ---------------------------------------------------------------- row 26
#[test]
fn row26_matches_at_both_ends() {
    let mut rng = Rng::new(26);
    for _ in 0..2000 {
        let search = rnd_search(&mut rng);
        let n = rng.range(2, 6);
        let mut fillers: Vec<usize> = (0..=n).map(|_| rng.below(8)).collect();
        let last = fillers.len() - 1;
        fillers[0] = 0; // match at the very start
        fillers[last] = 0; // match at the very end
        let orig = compose(&mut rng, &search, &fillers);
        let value = rnd_value(&mut rng);
        assert_eq!(count_matches(&orig, &search), n);
        check("row26", &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- row 27
#[test]
fn row27_full_random_sweep() {
    let mut rng = Rng::new(27);
    let pool = b"abXY01";
    for _ in 0..20000 {
        let asize = rng.range(1, pool.len());
        let alpha = &pool[..asize];
        let orig = rng.bytes_b(65, alpha);
        let search = rng.bytes_r(1, 8, alpha); // never empty (see ERRORS rows 10/11)
        let value = rng.bytes_b(9, alpha);
        check("row27", &orig, &search, &value);
    }
}
