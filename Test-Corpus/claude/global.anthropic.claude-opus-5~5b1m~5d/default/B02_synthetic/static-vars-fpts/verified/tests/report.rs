//! Differential tests for menu entries `3` (token distribution), `4`
//! (complexity score), `5` (find pattern) and `6` (interactive tokenizer).

mod common;

use common::assert_same;

#[test]
fn distribution_before_any_analysis() {
    // Every counter is zero, so only the two headings are printed.
    assert_same("distribution_fresh", b"3\n7\n");
    assert_same("distribution_twice", b"3\n3\n7\n");
}

#[test]
fn distribution_after_analysis() {
    assert_same("distribution", b"1\nint x = 1; // c\n\n3\n7\n");
    assert_same("distribution_sorted", b"1\na b b c c c d d d d\n\n3\n7\n");
}

#[test]
fn distribution_word_table_boundaries() {
    // track_word keeps at most 100 distinct words; only the top 10 are printed.
    for n in [1usize, 9, 10, 11, 99, 100, 101, 150] {
        let words: Vec<String> = (0..n).map(|i| format!("w{i}")).collect();
        let input = format!("1\n{}\n\n3\n7\n", words.join(" "));
        assert_same(&format!("words_{n}"), input.as_bytes());
    }
}

#[test]
fn distribution_with_tied_counts() {
    // The bubble sort only swaps on a strict `<`, so ties keep insertion order.
    assert_same("ties", b"1\na a b b c c d d e e f f\n\n3\n7\n");
    assert_same("ties_desc", b"1\na a a b b c\n\n3\n7\n");
    assert_same("ties_asc", b"1\na b b c c c\n\n3\n7\n");
}

#[test]
fn distribution_with_long_words() {
    let long: String = std::iter::repeat('u').take(300).collect();
    let input = format!("1\n{long} {long} short\n\n3\n7\n");
    assert_same("long_words", input.as_bytes());
}

#[test]
fn complexity_score_bands() {
    // score < 10 -> Low, < 50 -> Medium, otherwise High.
    assert_same("score_zero", b"4\n7\n");
    assert_same("score_low", b"1\nint x;\n\n4\n7\n");

    let medium = "if ".repeat(10);
    assert_same(
        "score_medium",
        format!("1\n{medium}\n\n4\n7\n").as_bytes(),
    );

    let high = "if while for return int ".repeat(10);
    assert_same("score_high", format!("1\n{high}\n\n4\n7\n").as_bytes());

    // Comments subtract, and the score is clamped at zero.
    assert_same("score_clamped", b"1\n// a\n// b\n// c\n\n4\n7\n");
    let comments = (0..20).map(|i| format!("// c{i}\n")).collect::<String>();
    assert_same(
        "score_many_comments",
        format!("1\n{comments}\n4\n7\n").as_bytes(),
    );

    // Punctuation contributes count/10, so the boundary at 10 matters.
    for n in [9usize, 10, 19, 20] {
        let puncts = ";".repeat(n);
        assert_same(
            &format!("score_punct_{n}"),
            format!("1\n{puncts}\n\n4\n7\n").as_bytes(),
        );
    }
}

#[test]
fn find_pattern_before_any_analysis() {
    // The tokenizer buffer is still empty, so nothing is found.
    assert_same("pattern_fresh", b"5\nx\n7\n");
    assert_same("pattern_fresh_empty", b"5\n\n7\n");
}

#[test]
fn find_pattern_eof_after_the_choice() {
    assert_same("eof_after_5", b"5\n");
}

#[test]
fn find_pattern_matches() {
    assert_same("pattern_hit", b"1\nabc abd xbc\n\n5\nab\n7\n");
    assert_same("pattern_miss", b"1\nabc\n\n5\nzz\n7\n");
    // An empty pattern makes strstr match every single token.
    assert_same("pattern_empty", b"1\nint x = 1;\n\n5\n\n7\n");
    assert_same("pattern_operator", b"1\na == b != c\n\n5\n=\n7\n");
    assert_same("pattern_quote", b"1\n\"hi\" 'c'\n\n5\n\"\n7\n");
    assert_same("pattern_slash", b"1\n// note\n/* b */\n\n5\n/\n7\n");
    assert_same("pattern_newline_token", b"1\na\nb\n\n5\nb\n7\n");
}

#[test]
fn find_pattern_repeats_and_resets() {
    // find_patterns calls reset(), so the same search can be repeated and the
    // cumulative character counter keeps growing.
    assert_same("pattern_twice", b"1\nfoo foo\n\n5\nfoo\n5\nfoo\n1\nbar\n\n5\nfoo\n7\n");
}

#[test]
fn find_pattern_long_and_odd_bytes() {
    let long: String = std::iter::repeat('z').take(300).collect();
    assert_same(
        "pattern_long",
        format!("1\nabc\n\n5\n{long}\n7\n").as_bytes(),
    );
    assert_same("pattern_high_byte", b"1\nabc\n\n5\n\xff\n7\n");
    assert_same("pattern_nul", b"1\nabc\n\n5\n\0abc\n7\n");
    assert_same("pattern_spaces", b"1\nabc def\n\n5\n  \n7\n");
}

#[test]
fn interactive_tokenizer_empty() {
    assert_same("interactive_empty", b"6\n\n7\n");
    assert_same("interactive_eof", b"6\n");
    assert_same("interactive_eof_mid", b"6\nabc");
}

#[test]
fn interactive_tokenizer_basic() {
    assert_same("interactive_basic", b"6\nint x = 1; // c\n\n7\n");
    assert_same("interactive_multiline", b"6\na\nb\nc\n\n7\n");
    assert_same(
        "interactive_all_kinds",
        b"6\nif (a >= 1) { \"s\" /*c*/ @ }\n\n7\n",
    );
}

#[test]
fn interactive_tokenizer_truncation_boundary() {
    // The loop stops after printing token 101 ("count > 100").
    for n in [99usize, 100, 101, 102, 105] {
        let words = vec!["a"; n].join(" ");
        assert_same(
            &format!("interactive_{n}_tokens"),
            format!("6\n{words}\n\n7\n").as_bytes(),
        );
    }
}

#[test]
fn interactive_tokenizer_fills_the_buffer() {
    let mut input = Vec::from(&b"6\n"[..]);
    for _ in 0..25 {
        input.extend(std::iter::repeat(b'a').take(254));
        input.push(b'\n');
    }
    input.extend_from_slice(b"\n7\n");
    assert_same("interactive_fill", &input);
}

#[test]
fn interactive_tokenizer_negative_columns() {
    // create_token computes `current_column - token.length`, which goes negative
    // for a token that follows a newline and is longer than the column counter.
    assert_same("negative_columns", b"6\n\"abcdefgh\"\nxyzw\n\n7\n");
    assert_same("column_after_ws", b"6\n   abcdef\n\n7\n");
}

#[test]
fn interactive_then_analysis_shares_state() {
    // The interactive tokenizer loads text into the same static buffer but does
    // not touch the analyzer counters.
    assert_same("interactive_then_3", b"6\nint x;\n\n3\n4\n7\n");
    assert_same("interactive_then_5", b"6\nint x;\n\n5\nint\n7\n");
    assert_same("analysis_then_interactive", b"1\nint x;\n\n6\nfloat y;\n\n3\n5\nfloat\n7\n");
}

#[test]
fn full_session_over_every_menu_entry() {
    assert_same(
        "full_session",
        b"1\nint main(void) { return 0; }\n\n2\n/nonexistent\n3\n4\n5\nmain\n6\na+b\n\n9\nzz\n7\n",
    );
}
