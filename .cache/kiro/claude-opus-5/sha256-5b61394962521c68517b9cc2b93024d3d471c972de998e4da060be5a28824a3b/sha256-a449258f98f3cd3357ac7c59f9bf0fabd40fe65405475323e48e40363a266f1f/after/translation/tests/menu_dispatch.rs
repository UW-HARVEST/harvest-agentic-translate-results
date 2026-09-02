//! Phase B — the driver loop in `c_src/src/main.c`.
//!
//! Every branch of `main`'s `switch (choice)` plus the `sscanf`/`fgets`
//! decisions that guard it.

mod common;
use common::assert_same;

// --- the loop's own exits ---------------------------------------------------

#[test]
fn empty_stdin_ends_the_loop_immediately() {
    // First fgets returns NULL -> `break` -> return 0.
    assert_same("empty-stdin", b"");
}

#[test]
fn choice_7_prints_goodbye_and_returns() {
    assert_same("exit-7", b"7\n");
}

#[test]
fn choice_7_without_trailing_newline() {
    // fgets still returns the partial line, so sscanf sees "7".
    assert_same("exit-7-no-newline", b"7");
}

#[test]
fn eof_after_the_menu_is_shown_again_and_again() {
    assert_same("many-menus", b"3\n3\n4\n4\n");
}

// --- sscanf("%d") outcomes -------------------------------------------------

#[test]
fn non_numeric_choice_is_invalid_input() {
    // sscanf returns 0 -> "Invalid input".
    assert_same("choice-alpha", b"abc\n7\n");
}

#[test]
fn blank_line_choice_is_invalid_input() {
    // %d skips the whitespace, hits end of buffer -> sscanf returns EOF.
    assert_same("choice-blank", b"\n7\n");
}

#[test]
fn whitespace_only_choice_is_invalid_input() {
    assert_same("choice-spaces", b"   \t \n7\n");
}

#[test]
fn out_of_range_choices_hit_the_default_arm() {
    assert_same("choice-zero", b"0\n7\n");
    assert_same("choice-negative", b"-1\n7\n");
    assert_same("choice-eight", b"8\n7\n");
    assert_same("choice-big", b"99\n7\n");
}

#[test]
fn choice_accepts_leading_whitespace_and_a_sign() {
    assert_same("choice-leading-space", b"   4\n7\n");
    assert_same("choice-plus", b"+4\n7\n");
    assert_same("choice-padded", b"  +6  \n\n7\n");
    assert_same("choice-leading-zeros", b"007\n");
}

#[test]
fn choice_stops_at_the_first_non_digit() {
    assert_same("choice-trailing-junk", b"7x\n");
    assert_same("choice-hex-like", b"0x7\n7\n"); // parses 0, not 7
    assert_same("choice-decimal", b"4.9\n7\n");
}

#[test]
fn choice_integer_conversion_truncates_the_way_the_c_does() {
    // glibc converts with strtol (saturating at LONG_MAX/LONG_MIN) and then
    // stores the low 32 bits into `int choice`.
    assert_same("choice-int-max", b"2147483647\n7\n");
    assert_same("choice-int-max-plus-1", b"2147483648\n7\n");
    assert_same("choice-int-min-minus-1", b"-2147483649\n7\n");
    assert_same("choice-wraps-to-1", b"4294967297\n7\n"); // == 1 mod 2^32
    assert_same("choice-wraps-to-7", b"4294967303\n"); // == 7 mod 2^32
    assert_same("choice-long-range", b"99999999999\n7\n");
    assert_same("choice-beyond-long", b"99999999999999999999999\n7\n");
}

#[test]
fn choice_line_longer_than_the_256_byte_buffer_is_split_by_fgets() {
    let mut input = b"1".to_vec();
    input.extend(std::iter::repeat(b' ').take(300));
    input.extend_from_slice(b"\n7\n");
    assert_same("choice-overlong-line", &input);
}

#[test]
fn nul_byte_truncates_the_choice_buffer() {
    // sscanf reads a C string, so the bytes past the NUL are invisible.
    assert_same("choice-nul-first", b"\x007\n7\n");
    assert_same("choice-nul-after-digit", b"7\x009\n");
}

// --- case 1: analyze typed text -------------------------------------------

#[test]
fn analyze_with_no_text_at_all() {
    assert_same("analyze-immediate-blank", b"1\n\n7\n");
}

#[test]
fn analyze_eof_instead_of_text() {
    assert_same("analyze-eof", b"1\n");
    assert_same("analyze-eof-mid-line", b"1\nfoo");
}

#[test]
fn analyze_a_single_token() {
    assert_same("analyze-one-word", b"1\nhello\n\n7\n");
    assert_same("analyze-one-keyword", b"1\nreturn\n\n7\n");
    assert_same("analyze-one-number", b"1\n42\n\n7\n");
}

#[test]
fn analyze_a_mixed_line() {
    assert_same(
        "analyze-mixed",
        b"1\nint x = 42; // note\n\"text\" /* block */ a->b\n\n7\n",
    );
}

#[test]
fn analyze_whitespace_only_text() {
    assert_same("analyze-spaces", b"1\n   \n\n7\n");
    assert_same("analyze-tab", b"1\n\t\n\n7\n");
}

#[test]
fn analyze_accumulates_tokenizer_statistics_across_calls() {
    // tokenizer_reset() deliberately keeps the running totals, so the second
    // report shows the sum of both runs.
    assert_same("analyze-twice", b"1\nfoo bar\n\n1\nfoo baz\n\n3\n4\n7\n");
}

#[test]
fn analyze_many_lines_until_the_input_buffer_fills() {
    let mut input = b"1\n".to_vec();
    for i in 0..800 {
        input.extend_from_slice(format!("line{i}\n").as_bytes());
    }
    input.extend_from_slice(b"\n7\n");
    assert_same("analyze-many-lines", &input);
}

// --- case 6: interactive tokenizer ---------------------------------------

#[test]
fn interactive_tokenizer_with_no_text() {
    assert_same("tok-empty", b"6\n\n7\n");
}

#[test]
fn interactive_tokenizer_eof_instead_of_text() {
    assert_same("tok-eof", b"6\n");
    assert_same("tok-eof-no-newline", b"6");
}

#[test]
fn interactive_tokenizer_one_of_each_token_kind() {
    assert_same(
        "tok-one-of-each",
        b"6\nident 12 KEYWORD_no if 3.5 \"s\" 'c' // line\n/* blk */ + == ; @\n\n7\n",
    );
}

#[test]
fn interactive_tokenizer_truncates_past_100_tokens() {
    for n in [99usize, 100, 101, 102, 200] {
        let mut input = b"6\n".to_vec();
        for i in 0..n {
            if i > 0 {
                input.push(b' ');
            }
            input.push(b'x');
        }
        input.extend_from_slice(b"\n\n7\n");
        assert_same(&format!("tok-count-{n}"), &input);
    }
}

#[test]
fn interactive_tokenizer_state_survives_into_later_menu_choices() {
    assert_same("tok-then-report", b"6\nif (a) b++;\n\n3\n4\n5\nb\n7\n");
}
