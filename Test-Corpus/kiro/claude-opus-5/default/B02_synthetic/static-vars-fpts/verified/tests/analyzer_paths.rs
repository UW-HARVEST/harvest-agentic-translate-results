//! Phase B/C — `c_src/src/analyzer.c`: token distribution, complexity score
//! and pattern search, including the branches that depend on accumulated
//! static state.

mod common;
use common::assert_same;

// --- case 3: print_token_distribution -------------------------------------

#[test]
fn distribution_before_anything_is_analyzed() {
    // No counts are above zero, so only the two headers print and the
    // "most common words" list is empty.
    assert_same("dist-cold", b"3\n7\n");
}

#[test]
fn distribution_prints_only_nonzero_rows() {
    assert_same("dist-words-only", b"1\nalpha beta\n\n3\n7\n");
    assert_same("dist-numbers-only", b"1\n1 2 3\n\n3\n7\n");
    assert_same("dist-errors-only", b"1\n@ # $\n\n3\n7\n");
}

#[test]
fn distribution_covers_all_twelve_rows_it_can_reach() {
    assert_same(
        "dist-all-kinds",
        b"1\nident 12 if \"s\" // c\n/* b */ + ; @\n\n3\n7\n",
    );
}

#[test]
fn distribution_bubble_sort_ordering() {
    assert_same("sort-distinct", b"1\naa bb aa cc bb aa dd\n\n3\n7\n");
    // Equal counts: the sort only swaps on strict `<`, so ties keep their
    // relative order after each pass.
    assert_same(
        "sort-ties",
        b"1\na a a b b b c c c d d d e e e\n\n3\n7\n",
    );
    assert_same("sort-single-word", b"1\nonly\n\n3\n7\n");
    assert_same("sort-two-words", b"1\nx y y\n\n3\n7\n");
}

#[test]
fn distribution_prints_at_most_ten_words() {
    for n in [9usize, 10, 11, 25] {
        let mut text = Vec::new();
        for i in 0..n {
            if i > 0 {
                text.push(b' ');
            }
            // Give each word a distinct count so the ordering is unambiguous.
            let word = format!("w{i}");
            for k in 0..=i {
                if k > 0 {
                    text.push(b' ');
                }
                text.extend_from_slice(word.as_bytes());
            }
        }
        let mut input = b"1\n".to_vec();
        input.extend_from_slice(&text);
        input.extend_from_slice(b"\n\n3\n7\n");
        assert_same(&format!("dist-top10-{n}"), &input);
    }
}

#[test]
fn word_tracking_stops_at_one_hundred_distinct_words() {
    for n in [99usize, 100, 101, 150] {
        let mut input = b"1\n".to_vec();
        for i in 0..n {
            if i > 0 {
                input.push(b' ');
            }
            input.extend_from_slice(format!("w{i}").as_bytes());
        }
        input.extend_from_slice(b"\n\n3\n7\n");
        assert_same(&format!("track-{n}"), &input);
    }
}

#[test]
fn word_tracking_accumulates_across_separate_analyses() {
    assert_same(
        "track-accumulate",
        b"1\nfoo bar\n\n1\nfoo foo baz\n\n3\n7\n",
    );
}

// --- case 4: calculate_complexity_score -----------------------------------

#[test]
fn complexity_of_nothing_is_zero() {
    assert_same("score-cold", b"4\n7\n");
}

#[test]
fn complexity_is_clamped_at_zero_when_comments_dominate() {
    // score -= comment_count can drive the total negative; the C clamps to 0.
    assert_same("score-clamped", b"1\n// a\n// b\n// c\n// d\n\n4\n7\n");
}

#[test]
fn complexity_low_medium_and_high_bands() {
    // score < 10
    assert_same("score-low", b"1\nif else\n\n4\n7\n");
    // 10 <= score < 50
    assert_same("score-medium", b"1\nif else if else if else\n\n4\n7\n");
    // score >= 50
    let mut input = b"1\n".to_vec();
    for _ in 0..40 {
        input.extend_from_slice(b"if ");
    }
    input.extend_from_slice(b"\n\n4\n7\n");
    assert_same("score-high", &input);
}

#[test]
fn complexity_band_boundaries_exactly() {
    // 5 keywords * 2 = 10 -> Medium (not Low).
    assert_same("score-exactly-10", b"1\nif if if if if\n\n4\n7\n");
    // 4 keywords * 2 + 1 operator = 9 -> Low.
    assert_same("score-exactly-9", b"1\nif if if if +\n\n4\n7\n");
    // 25 keywords * 2 = 50 -> High.
    let mut fifty = b"1\n".to_vec();
    for _ in 0..25 {
        fifty.extend_from_slice(b"do ");
    }
    fifty.extend_from_slice(b"\n\n4\n7\n");
    assert_same("score-exactly-50", &fifty);
    // 24 keywords * 2 + 1 operator = 49 -> Medium.
    let mut forty_nine = b"1\n".to_vec();
    for _ in 0..24 {
        forty_nine.extend_from_slice(b"do ");
    }
    forty_nine.extend_from_slice(b"+\n\n4\n7\n");
    assert_same("score-exactly-49", &forty_nine);
}

#[test]
fn complexity_punctuation_contributes_by_integer_division() {
    // punctuation_count / 10 truncates toward zero.
    for n in [9usize, 10, 19, 20, 99] {
        let mut input = b"1\n".to_vec();
        input.extend(std::iter::repeat(b';').take(n));
        input.extend_from_slice(b"\n\n4\n3\n7\n");
        assert_same(&format!("score-punct-{n}"), &input);
    }
}

#[test]
fn complexity_reflects_state_from_the_interactive_tokenizer_too() {
    assert_same("score-after-tokenize", b"6\nif (a && b) { c++; }\n\n4\n7\n");
}

// --- case 5: find_patterns ------------------------------------------------

#[test]
fn pattern_search_with_nothing_loaded() {
    assert_same("pat-cold", b"5\nfoo\n7\n");
}

#[test]
fn pattern_search_eof_instead_of_a_pattern() {
    // The C's `break` leaves the switch, not the loop, so the menu reprints.
    assert_same("pat-eof", b"5\n");
    assert_same("pat-eof-no-newline", b"5");
}

#[test]
fn pattern_search_basic_matches() {
    assert_same("pat-hit", b"1\nfoo foobar bar\n\n5\nfoo\n7\n");
    assert_same("pat-miss", b"1\nfoo\n\n5\nzzz\n7\n");
    assert_same("pat-substring", b"1\nabcdef\n\n5\ncde\n7\n");
}

#[test]
fn an_empty_pattern_matches_every_token() {
    // strstr(s, "") returns s, which is never NULL.
    assert_same("pat-empty", b"1\na b 1 + ;\n\n5\n\n7\n");
}

#[test]
fn pattern_search_reuses_the_buffer_after_reset() {
    // find_patterns() only calls reset(), so it re-scans the text that the
    // previous analyze/tokenize call left in the buffer.
    assert_same("pat-after-analyze", b"1\nalpha beta\n\n5\na\n7\n");
    assert_same("pat-after-tokenize", b"6\nalpha beta\n\n5\na\n7\n");
    assert_same("pat-twice", b"1\nalpha beta\n\n5\na\n5\nb\n7\n");
}

#[test]
fn pattern_can_match_punctuation_strings_and_comments() {
    assert_same("pat-newline-token", b"1\na\nb\n\n5\n;\n7\n");
    assert_same("pat-quote", b"1\na'b\n\n5\n'\n7\n");
    assert_same("pat-slash", b"1\n// note\n\n5\n/\n7\n");
    assert_same("pat-brace", b"1\n{ }\n\n5\n{\n7\n");
}

#[test]
fn pattern_containing_a_nul_is_truncated() {
    assert_same("pat-nul", b"1\nfoo\n\n5\nfo\x00o\n7\n");
}

#[test]
fn pattern_longer_than_any_token_never_matches() {
    let mut input = b"1\nshort\n\n5\n".to_vec();
    input.extend(std::iter::repeat(b'q').take(200));
    input.extend_from_slice(b"\n7\n");
    assert_same("pat-overlong", &input);
}
