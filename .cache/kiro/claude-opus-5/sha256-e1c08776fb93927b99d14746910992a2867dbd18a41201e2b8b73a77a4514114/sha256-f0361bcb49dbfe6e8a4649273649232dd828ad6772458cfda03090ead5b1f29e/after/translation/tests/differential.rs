//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through `dlopen`/`dlsym` (never the Rust code directly) and asserts the
//! returned C strings are byte-identical. Each row uses many randomized
//! inputs from a fixed seed, plus a third-opinion check against `model()`
//! (an independent transcription of the C algorithm) so that a shared
//! misreading of the C cannot pass unnoticed.
//!
//! Rows 1-4 also exercise `ERRORS.md` row 1 (the `strdup` early return).

mod common;

use common::{model, show, Pair, Rng, SEED};

const A2: &[u8] = b"ab";
const A3: &[u8] = b"abc";

/// Run one case through both `.so`s and against the reference model.
#[track_caller]
fn check(p: &Pair, orig: &[u8], search: &[u8], value: &[u8]) {
    let got = p.assert_same(orig, search, value);
    let got = got.unwrap_or_else(|| {
        panic!(
            "unexpected NULL for orig={} search={} value={}",
            show(orig),
            show(search),
            show(value)
        )
    });
    let want = model(orig, search, value);
    assert_eq!(
        got,
        want,
        "both libraries agree but differ from the reference model\n  orig   = {}\n  search = {}\n  value  = {}\n  libs  -> {}\n  model -> {}",
        show(orig),
        show(search),
        show(value),
        show(&got),
        show(&want),
    );
}

// ---------------------------------------------------------------- rows 1-4
// M = 0 matches: the `strstr(orig, search) == NULL` / `strdup` early return.

#[test]
fn cfg_01_no_match_short() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 1);
    for _ in 0..2000 {
        // Draw the subject from one alphabet and the needle from a disjoint
        // one so a match is impossible by construction.
        let orig = r.word(r.range(1, 31), b"abcdef");
        let search = r.word(r.range(1, 4), b"XYZ");
        let value = r.word(r.range(0, 6), A3);
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_02_no_match_empty_orig() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 2);
    for _ in 0..500 {
        let search = r.word(r.range(1, 4), A2);
        let value = r.word(r.range(0, 6), A3);
        check(&p, b"", &search, &value);
    }
    check(&p, b"", b"a", b"");
    check(&p, b"", b"a", b"zzz");
}

#[test]
fn cfg_03_needle_longer_than_orig() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let n = r.range(0, 8);
        let orig = r.word(n, A2);
        // needle strictly longer than the subject
        let search = r.word(n + r.range(1, 5), A2);
        let value = r.word(r.range(0, 6), A3);
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_04_no_match_long_highbytes() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 4);
    for _ in 0..40 {
        let mut orig = r.high_bytes(4096);
        // guarantee no match: needle uses low bytes only
        orig.iter_mut().for_each(|b| {
            if *b < 0x80 {
                *b = 0x80
            }
        });
        let search = r.word(r.range(1, 5), b"\x01\x02\x7f");
        let value = r.bytes(r.range(0, 20));
        check(&p, &orig, &search, &value);
    }
}

// ---------------------------------------------------------------- rows 5-9
// M = 1 match, crossing P (prefix) x T (tail).

#[test]
fn cfg_05_whole_string_match() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 5);
    for _ in 0..1000 {
        let orig = r.word(r.range(1, 20), A3);
        let value = r.word(orig.len() + r.range(1, 10), b"WXYZ"); // value_len > search_len
        check(&p, &orig, &orig.clone(), &value);
    }
}

#[test]
fn cfg_06_whole_string_delete() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 6);
    for _ in 0..1000 {
        let orig = r.word(r.range(1, 20), A3);
        check(&p, &orig, &orig.clone(), b"");
    }
}

/// Match at offset 0, tail present (no prefix `malloc`, tail `realloc` taken).
#[test]
fn cfg_07_match_at_start_with_tail() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 7);
    for _ in 0..2000 {
        let search = r.word(r.range(1, 5), b"XY");
        let tail = r.word(r.range(1, 20), b"abc");
        let mut orig = search.clone();
        orig.extend_from_slice(&tail);
        let value = r.word(r.range(0, 8), b"pq");
        check(&p, &orig, &search, &value);
    }
}

/// Match at the very end: prefix `malloc` taken, tail `realloc` skipped
/// (`from == orig_len`).
#[test]
fn cfg_08_match_at_end_with_prefix() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 8);
    for _ in 0..2000 {
        let search = r.word(r.range(1, 5), b"XY");
        let mut orig = r.word(r.range(1, 20), b"abc");
        orig.extend_from_slice(&search);
        let value = r.word(r.range(0, 8), b"pq");
        check(&p, &orig, &search, &value);
    }
}

/// Prefix and tail both present.
#[test]
fn cfg_09_match_in_middle() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 9);
    for _ in 0..3000 {
        let search = r.word(r.range(1, 5), b"XY");
        let mut orig = r.word(r.range(1, 20), b"abc");
        orig.extend_from_slice(&search);
        orig.extend_from_slice(&r.word(r.range(1, 20), b"abc"));
        let value = r.word(r.range(0, 8), b"pq");
        check(&p, &orig, &search, &value);
    }
}

// -------------------------------------------------------------- rows 10-13
// M = 2 matches, crossing G (adjacent vs. separated) x P x T.

fn two_match_case(r: &Rng, adjacent: bool, prefix: bool, tail: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let search = r.word(r.range(1, 5), b"XY");
    let filler = |r: &Rng| r.word(r.range(1, 15), b"abc");
    let mut orig = Vec::new();
    if prefix {
        orig.extend_from_slice(&filler(r));
    }
    orig.extend_from_slice(&search);
    if !adjacent {
        orig.extend_from_slice(&filler(r));
    }
    orig.extend_from_slice(&search);
    if tail {
        orig.extend_from_slice(&filler(r));
    }
    let value = r.word(r.range(0, 8), b"pq");
    (orig, search, value)
}

#[test]
fn cfg_10_two_adjacent_no_prefix_no_tail() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 10);
    for _ in 0..2000 {
        let (o, s, v) = two_match_case(&r, true, false, false);
        check(&p, &o, &s, &v);
    }
}

#[test]
fn cfg_11_two_adjacent_prefix_tail() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 11);
    for _ in 0..2000 {
        let (o, s, v) = two_match_case(&r, true, true, true);
        check(&p, &o, &s, &v);
    }
}

#[test]
fn cfg_12_two_separated_no_prefix_no_tail() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 12);
    for _ in 0..2000 {
        let (o, s, v) = two_match_case(&r, false, false, false);
        check(&p, &o, &s, &v);
    }
}

#[test]
fn cfg_13_two_separated_prefix_tail() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 13);
    for _ in 0..2000 {
        let (o, s, v) = two_match_case(&r, false, true, true);
        check(&p, &o, &s, &v);
    }
}

// -------------------------------------------------------------- rows 14-18
// M = many matches.

#[test]
fn cfg_14_many_adjacent_growing() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 14);
    for _ in 0..800 {
        let search = r.word(r.range(1, 4), b"XY");
        let n = r.range(3, 40);
        let orig: Vec<u8> = search.iter().cycle().take(search.len() * n).cloned().collect();
        let value = r.word(search.len() + r.range(1, 6), b"pq");
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_15_many_adjacent_delete() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 15);
    for _ in 0..800 {
        let search = r.word(r.range(1, 4), b"XY");
        let n = r.range(3, 40);
        let orig: Vec<u8> = search.iter().cycle().take(search.len() * n).cloned().collect();
        check(&p, &orig, &search, b"");
    }
}

/// Mixed adjacency, single-byte needle, `value_len == search_len`.
#[test]
fn cfg_16_many_mixed_len_equal() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 16);
    for _ in 0..3000 {
        let orig = r.word(r.range(1, 60), b"aaab");
        check(&p, &orig, b"a", b"Z");
    }
}

/// Mixed adjacency, multi-byte needle, `value_len < search_len`.
#[test]
fn cfg_17_many_mixed_shrinking() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 17);
    for _ in 0..3000 {
        let orig = r.word(r.range(1, 60), b"ab");
        let search = r.word(r.range(2, 4), b"ab");
        let value = r.word(search.len() - 1 - r.below(search.len() - 1), b"Z");
        check(&p, &orig, &search, &value);
    }
}

/// Mixed adjacency, multi-byte needle, `value_len > search_len`.
#[test]
fn cfg_18_many_mixed_growing() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 18);
    for _ in 0..3000 {
        let orig = r.word(r.range(1, 60), b"ab");
        let search = r.word(r.range(2, 4), b"ab");
        let value = r.word(search.len() + r.range(1, 5), b"YZ");
        check(&p, &orig, &search, &value);
    }
}

// -------------------------------------------------------------- rows 19-20
// O = self-overlapping needles (the re-search skips overlaps).

#[test]
fn cfg_19_overlapping_runs() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 19);
    for run in 0usize..=24 {
        for needle in [&b"aa"[..], &b"aaa"[..]] {
            for value in [&b""[..], &b"Z"[..], &b"ZZZZ"[..]] {
                let orig = vec![b'a'; run];
                check(&p, &orig, needle, value);
                // same, wrapped in non-matching context
                let mut wrapped = b"xy".to_vec();
                wrapped.extend_from_slice(&orig);
                wrapped.extend_from_slice(b"xy");
                check(&p, &wrapped, needle, value);
            }
        }
    }
    for _ in 0..2000 {
        let orig = r.word(r.range(1, 40), b"aaaab");
        let search = r.word(r.range(2, 4), b"a");
        let value = r.word(r.range(0, 5), b"Z");
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_20_overlapping_aba() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 20);
    for reps in 1usize..=20 {
        let mut orig = b"ab".repeat(reps);
        orig.push(b'a');
        for value in [&b""[..], &b"Q"[..], &b"QQQ"[..]] {
            check(&p, &orig, b"aba", value);
            check(&p, &orig, b"ab", value);
            check(&p, &orig, b"ba", value);
        }
    }
    for _ in 0..2000 {
        let orig = r.word(r.range(1, 40), b"ab");
        let value = r.word(r.range(0, 5), b"Q");
        check(&p, &orig, b"aba", &value);
    }
}

// -------------------------------------------------------------- rows 21-22
// L = long subjects with hundreds of matches (many realloc rounds).

#[test]
fn cfg_21_long_many_matches() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 21);
    for _ in 0..30 {
        let len = r.range(4096, 16384);
        let orig = r.word(len, b"aabbc");
        let search = r.word(r.range(1, 3), b"ab");
        let value = r.word(search.len() + r.range(1, 4), b"XYZ");
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_22_long_many_matches_delete() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 22);
    for _ in 0..30 {
        let len = r.range(4096, 16384);
        let orig = r.word(len, b"aabbc");
        let search = r.word(r.range(1, 3), b"ab");
        check(&p, &orig, &search, b"");
    }
}

// ------------------------------------------------------------------- row 23
// B = high bytes everywhere (char-signedness sensitive paths).

#[test]
fn cfg_23_high_bytes_many_matches() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 23);
    let alphabet: &[u8] = &[0x80, 0x81, 0xfe, 0xff];
    for _ in 0..2000 {
        let orig = r.word(r.range(1, 50), alphabet);
        let search = r.word(r.range(1, 3), alphabet);
        let value = r.word(r.range(0, 6), &[0x90, 0xa0, 0xff]);
        check(&p, &orig, &search, &value);
    }
    // mixed high/low
    for _ in 0..2000 {
        let orig = r.word(r.range(1, 50), &[b'a', 0x80, 0xff, b'b']);
        let search = r.word(r.range(1, 3), &[b'a', 0x80, 0xff]);
        let value = r.bytes(r.range(0, 6));
        check(&p, &orig, &search, &value);
    }
}

// ------------------------------------------------------------------- row 24

#[test]
fn cfg_24_single_byte() {
    let p = Pair::new();
    for b in [1u8, b'a', 0x7f, 0x80, 0xff] {
        let orig = vec![b];
        check(&p, &orig, &orig.clone(), b"");
        check(&p, &orig, &orig.clone(), b"Z");
        check(&p, &orig, &orig.clone(), b"ZZZZZ");
        check(&p, &orig, b"q", b"Z");
        check(&p, &orig, &[b, b], b"Z");
    }
}

// -------------------------------------------------------------- rows 25-26

#[test]
fn cfg_25_value_contains_needle() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 25);
    for _ in 0..2000 {
        let search = r.word(r.range(1, 3), b"ab");
        let orig = r.word(r.range(1, 40), b"abc");
        let mut value = r.word(r.range(0, 3), b"xy");
        value.extend_from_slice(&search);
        value.extend_from_slice(&r.word(r.range(0, 3), b"xy"));
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_26_identity_replacement() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 26);
    for _ in 0..3000 {
        let orig = r.word(r.range(0, 40), b"abc");
        let search = r.word(r.range(1, 4), b"abc");
        check(&p, &orig, &search, &search.clone());
    }
}

// -------------------------------------------------------------- rows 27-28
// Blanket fuzz: hits every M/P/G/O/T/V combination by construction.

#[test]
fn cfg_27_fuzz_alphabet2() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 27);
    for _ in 0..200_000 {
        let orig = r.word(r.range(0, 40), A2);
        let search = r.word(r.range(1, 4), A2);
        let value = r.word(r.range(0, 6), A2);
        check(&p, &orig, &search, &value);
    }
}

#[test]
fn cfg_28_fuzz_bytes() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 28);
    for _ in 0..200_000 {
        let orig = r.bytes(r.range(0, 64));
        let search = r.bytes(r.range(1, 8));
        let value = r.bytes(r.range(0, 16));
        check(&p, &orig, &search, &value);
        // and a variant guaranteed to match somewhere
        if !orig.is_empty() {
            let n = r.range(1, orig.len().min(8));
            let at = r.below(orig.len() - n + 1);
            let needle = orig[at..at + n].to_vec();
            check(&p, &orig, &needle, &value);
        }
    }
}

// ---------------------------------------------------------- ERRORS.md row 1

/// `ERRORS.md` row 1: when the needle is absent the C returns `strdup(orig)`
/// — a non-NULL, freshly allocated, exact copy — regardless of `value`.
#[test]
fn err_01_no_match_returns_copy() {
    let p = Pair::new();
    let r = Rng::new(SEED ^ 101);
    for _ in 0..3000 {
        let orig = r.word(r.range(0, 40), b"abc");
        let search = r.word(r.range(1, 4), b"XYZ");
        let value = r.word(r.range(0, 8), b"pq");
        let c = p.c.call(&orig, &search, &value);
        let rs = p.rust.call(&orig, &search, &value);
        assert_eq!(c, rs, "no-match divergence for orig={}", show(&orig));
        assert_eq!(
            c.as_deref(),
            Some(&orig[..]),
            "no-match result must be a verbatim copy of orig"
        );
    }
}

// ------------------------------------------------- exhaustive supplement
// Randomized rows sample the input space; these two tests cover it
// exhaustively for small shapes, which subsumes every M/P/G/O/T/V
// combination of `CONFIGS.md` at those sizes with no sampling luck involved.

/// Every string over `{a,b}` of length 0..=12 (8 191 subjects) x every needle
/// of length 1..=3 (14) x every replacement of length 0..=2 (7)
/// = 802 718 exhaustive cases.
#[test]
fn exhaustive_alphabet2() {
    let p = Pair::new();
    let origs = all_words(b"ab", 0, 12);
    let searches = all_words(b"ab", 1, 3);
    let values = all_words(b"ab", 0, 2);
    let mut n = 0usize;
    for o in &origs {
        for s in &searches {
            for v in &values {
                check(&p, o, s, v);
                n += 1;
            }
        }
    }
    assert_eq!(n, origs.len() * searches.len() * values.len());
    assert_eq!(n, 802_718, "exhaustive case count changed");
}

/// Every string over `{a,b,c}` of length 0..=7 (3 280) x needles of length
/// 1..=2 (12) x replacements of length 0..=1 (4) = 157 440 exhaustive cases,
/// adding a third distinct byte so that "gap content differs from needle
/// content" is covered exhaustively too.
#[test]
fn exhaustive_alphabet3() {
    let p = Pair::new();
    let origs = all_words(b"abc", 0, 7);
    let searches = all_words(b"abc", 1, 2);
    let values = all_words(b"abc", 0, 1);
    for o in &origs {
        for s in &searches {
            for v in &values {
                check(&p, o, s, v);
            }
        }
    }
}

/// All strings over `alphabet` with length in `lo..=hi`.
fn all_words(alphabet: &[u8], lo: usize, hi: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for len in lo..=hi {
        let total = alphabet.len().pow(len as u32);
        for mut idx in 0..total {
            let mut w = Vec::with_capacity(len);
            for _ in 0..len {
                w.push(alphabet[idx % alphabet.len()]);
                idx /= alphabet.len();
            }
            out.push(w);
        }
    }
    out
}
