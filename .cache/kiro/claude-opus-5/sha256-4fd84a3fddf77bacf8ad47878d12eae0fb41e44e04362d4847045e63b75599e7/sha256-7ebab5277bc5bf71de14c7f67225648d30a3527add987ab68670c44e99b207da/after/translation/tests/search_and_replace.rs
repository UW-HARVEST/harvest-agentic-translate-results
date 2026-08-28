//! Behavioural equivalence tests for `searchAndReplace`, the single public
//! entry point declared in `c_src/include/lib.h`.
//!
//! NOTE: an empty `search` string makes the C implementation loop forever
//! (`strstr(x, "")` always matches, and `inx_start` never advances), so that
//! input is deliberately never exercised.

mod common;
use common::assert_same;

#[test]
fn no_match_returns_copy() {
    assert_same(b"hello world", b"xyz", b"REPL");
    assert_same(b"", b"a", b"b");
    assert_same(b"short", b"a much longer needle", b"x");
    assert_same(b"abc", b"abcd", b"z");
}

#[test]
fn single_match_positions() {
    // match at the very beginning
    assert_same(b"abcdef", b"abc", b"XY");
    // match in the middle
    assert_same(b"xxABCyy", b"ABC", b"-");
    // match at the very end
    assert_same(b"tailABC", b"ABC", b"12345");
    // whole string is the match
    assert_same(b"ABC", b"ABC", b"replacement");
}

#[test]
fn empty_replacement_deletes() {
    assert_same(b"ABC", b"ABC", b"");
    assert_same(b"xABCy", b"ABC", b"");
    assert_same(b"aXbXc", b"X", b"");
    assert_same(b"XXXX", b"X", b"");
    assert_same(b"prefixXX", b"X", b"");
    assert_same(b"XXsuffix", b"X", b"");
}

#[test]
fn adjacent_and_repeated_matches() {
    assert_same(b"aa", b"a", b"X");
    assert_same(b"aaa", b"a", b"X");
    assert_same(b"aaaa", b"aa", b"X");
    assert_same(b"aaaaa", b"aa", b"X");
    assert_same(b"abab", b"ab", b"Z");
    assert_same(b"ababab", b"ab", b"");
    assert_same(b"..a..a..a..", b"a", b"[]");
}

#[test]
fn overlapping_needles() {
    // glibc strstr finds the leftmost match; the C code then skips
    // search_len bytes, so overlaps are consumed non-greedily.
    assert_same(b"aaa", b"aa", b"X");
    assert_same(b"aaaaaa", b"aaa", b"b");
    assert_same(b"abababa", b"aba", b"Q");
    assert_same(b"aXaXaXa", b"aXa", b"-");
}

#[test]
fn replacement_longer_and_shorter() {
    assert_same(b"a.b.c", b".", b"---------");
    assert_same(b"a---------b", b"---------", b".");
    assert_same(b"key=value", b"=", b" -> ");
    assert_same(b"one two three", b" ", b"");
}

#[test]
fn search_equals_value() {
    assert_same(b"abcabc", b"abc", b"abc");
    assert_same(b"aaa", b"a", b"a");
}

#[test]
fn non_ascii_and_binary_bytes() {
    assert_same(b"caf\xc3\xa9 latte", b"\xc3\xa9", b"e");
    assert_same(b"\xff\xfe\xff\xfe", b"\xff\xfe", b"\x01\x02\x03");
    assert_same(b"\x01\x02\x03", b"\x02", b"\xff");
    assert_same("naïve façade".as_bytes(), "ï".as_bytes(), b"i");
}

#[test]
fn whitespace_and_newlines() {
    assert_same(b"line1\nline2\nline3", b"\n", b"\r\n");
    assert_same(b"\t\t\t", b"\t", b"    ");
    assert_same(b"a b  c   d", b"  ", b" ");
}

#[test]
fn long_inputs() {
    let orig = "ab".repeat(2000);
    assert_same(orig.as_bytes(), b"ab", b"cd");
    assert_same(orig.as_bytes(), b"ba", b"");
    assert_same(orig.as_bytes(), b"ab", b"LONGER-REPLACEMENT");

    let big = format!("{}NEEDLE{}", "x".repeat(5000), "y".repeat(5000));
    assert_same(big.as_bytes(), b"NEEDLE", b"n");
    assert_same(big.as_bytes(), b"x", b"");

    let value = "V".repeat(500);
    assert_same(b"a|b|c|d|e", b"|", value.as_bytes());
}

#[test]
fn single_byte_edge_cases() {
    assert_same(b"a", b"a", b"b");
    assert_same(b"a", b"a", b"");
    assert_same(b"a", b"b", b"c");
    assert_same(b"a", b"aa", b"c");
}

/// Deterministic pseudo-random sweep over a tiny alphabet so that matches,
/// overlaps, gaps and boundary positions occur densely.
#[test]
fn randomized_sweep() {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let alphabet = b"aab.";

    for _ in 0..3000 {
        let olen = (next() % 24) as usize;
        let slen = 1 + (next() % 4) as usize; // never empty
        let vlen = (next() % 5) as usize;

        let orig: Vec<u8> = (0..olen)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        let search: Vec<u8> = (0..slen)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        let value: Vec<u8> = (0..vlen)
            .map(|_| b"XYZ-"[(next() % 4) as usize])
            .collect();

        assert_same(&orig, &search, &value);
    }
}

/// Exhaustive sweep over every short binary-ish string built from a two
/// character alphabet, crossed with every short needle and replacement.
#[test]
fn exhaustive_short_strings() {
    fn words(alphabet: &[u8], len: usize) -> Vec<Vec<u8>> {
        if len == 0 {
            return vec![Vec::new()];
        }
        let mut out = Vec::new();
        for w in words(alphabet, len - 1) {
            for &c in alphabet {
                let mut n = w.clone();
                n.push(c);
                out.push(n);
            }
        }
        out
    }

    let alphabet = b"ab";
    let mut origs = Vec::new();
    for len in 0..=7 {
        origs.extend(words(alphabet, len));
    }
    let mut searches = Vec::new();
    for len in 1..=3 {
        searches.extend(words(alphabet, len));
    }
    let values: Vec<&[u8]> = vec![b"", b"a", b"Z", b"ab", b"ZZZ"];

    for orig in &origs {
        for search in &searches {
            for value in &values {
                assert_same(orig, search, value);
            }
        }
    }
}
