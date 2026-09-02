//! Phase B/C — every branch inside `c_src/src/tokenizer.c`.
//!
//! Driven through menu choice 6, which prints each token's type, text, line
//! and column, so a wrong classification or a wrong column shows up directly.

mod common;
use common::assert_same;

/// Feed `text` to the interactive tokenizer and also ask for the token
/// distribution and complexity score afterwards.
fn tok(text: &[u8]) -> Vec<u8> {
    let mut input = b"6\n".to_vec();
    input.extend_from_slice(text);
    input.extend_from_slice(b"\n\n3\n4\n7\n");
    input
}

// --- next_token dispatch ---------------------------------------------------

#[test]
fn newline_tokens() {
    assert_same("nl-single", &tok(b"a\nb"));
    assert_same("nl-many", &tok(b"a\n\n\nb"));
}

#[test]
fn identifiers_and_underscores() {
    assert_same("ident-plain", &tok(b"abc"));
    assert_same("ident-underscore-first", &tok(b"_abc"));
    assert_same("ident-underscore-only", &tok(b"_"));
    assert_same("ident-digits-inside", &tok(b"a1b2_3"));
}

#[test]
fn every_keyword_is_recognised() {
    let keywords: &[&str] = &[
        "if", "else", "while", "for", "return", "int", "char", "float", "double", "void", "struct",
        "typedef", "const", "static", "extern", "auto", "register", "sizeof", "break", "continue",
        "switch", "case", "default", "do", "goto", "enum", "union", "signed", "unsigned", "long",
        "short",
    ];
    for kw in keywords {
        assert_same(&format!("keyword-{kw}"), &tok(kw.as_bytes()));
        // Near-misses must stay identifiers.
        assert_same(&format!("keyword-{kw}-suffixed"), &tok(format!("{kw}x").as_bytes()));
        assert_same(
            &format!("keyword-{kw}-upper"),
            &tok(kw.to_uppercase().as_bytes()),
        );
    }
}

#[test]
fn numbers_including_the_second_decimal_point_break() {
    assert_same("num-int", &tok(b"12345"));
    assert_same("num-decimal", &tok(b"3.14"));
    assert_same("num-trailing-dot", &tok(b"7."));
    assert_same("num-two-dots", &tok(b"1.2.3"));
    assert_same("num-many-dots", &tok(b"1.2.3.4.5"));
    assert_same("num-dot-dot", &tok(b"1..2"));
    // A leading '.' is punctuation, not a number.
    assert_same("num-leading-dot", &tok(b".5"));
    assert_same("num-then-ident", &tok(b"12abc"));
}

#[test]
fn strings_with_both_quote_characters() {
    assert_same("str-double", &tok(b"\"hello\""));
    assert_same("str-single", &tok(b"'c'"));
    assert_same("str-empty-double", &tok(b"\"\""));
    assert_same("str-empty-single", &tok(b"''"));
    assert_same("str-mixed-quotes", &tok(b"\"it's\""));
    assert_same("str-single-holding-double", &tok(b"'\"'"));
}

#[test]
fn strings_that_never_close() {
    // The scan stops at the newline, leaving no closing quote in the token.
    assert_same("str-unterminated-eol", &tok(b"\"open"));
    assert_same("str-unterminated-then-more", &tok(b"\"open\nnext"));
    assert_same("str-lone-double", &tok(b"\""));
    assert_same("str-lone-single", &tok(b"'"));
}

#[test]
fn string_escape_handling() {
    assert_same("str-escaped-quote", b"6\n\"a\\\"b\"\n\n3\n7\n");
    assert_same("str-escaped-backslash", b"6\n\"a\\\\\"\n\n3\n7\n");
    assert_same("str-backslash-at-eol", b"6\n\"a\\\n\n3\n7\n");
    assert_same("str-trailing-backslash", b"6\n\"a\\");
    assert_same("str-escape-newline-pair", b"6\n\"a\\nb\"\n\n3\n7\n");
}

#[test]
fn slash_always_starts_a_comment_scan() {
    // In the C, the comment test is `c == '/' && (peek_char() == '/' || ...)`
    // but peek_char() has not advanced, so it still returns '/'. Every '/'
    // therefore goes to scan_comment and the operator branch is dead for '/'.
    assert_same("slash-lone", &tok(b"/"));
    assert_same("slash-divide", &tok(b"a / b"));
    assert_same("slash-divide-tight", &tok(b"a/b"));
    assert_same("slash-assign", &tok(b"x /= y"));
    assert_same("slash-number", &tok(b"1/2"));
    assert_same("slash-double", &tok(b"//"));
    assert_same("slash-triple", &tok(b"///x"));
    assert_same("slash-star-only", &tok(b"/*"));
    assert_same("slash-close-only", &tok(b"*/"));
}

#[test]
fn line_comments() {
    assert_same("comment-line", &tok(b"// hello"));
    assert_same("comment-line-then-code", &tok(b"a // hello\nb"));
    assert_same("comment-line-empty", &tok(b"//\nb"));
}

#[test]
fn block_comments() {
    assert_same("comment-block", &tok(b"/* hello */"));
    assert_same("comment-block-multiline", &tok(b"/* a\nb\nc */ x"));
    assert_same("comment-block-unterminated", &tok(b"/* open"));
    assert_same("comment-block-stars", &tok(b"/*** x ***/"));
    assert_same("comment-block-star-then-not-slash", &tok(b"/* a*b */"));
    assert_same("comment-block-immediate-close", &tok(b"/**/"));
    assert_same("comment-block-shortest", &tok(b"/*/"));
    assert_same("comment-block-after-close", &tok(b"/* a */ b"));
}

#[test]
fn one_and_two_character_operators() {
    assert_same("op-two-char", &tok(b"== != <= >= && || ++ -- -> << >>"));
    assert_same("op-one-char", &tok(b"= ! < > & | + - * % ^ ~ ? :"));
    // Combinations that are *not* two-character operators.
    assert_same("op-not-pairs", &tok(b"=! <- >< |& ^^ ~~ ?? :: +- -+"));
    assert_same("op-triple", &tok(b"=== <<< >>> &&& |||"));
    assert_same("op-arrow-chain", &tok(b"a->b->c"));
    assert_same("op-increment-chain", &tok(b"a+++b"));
}

#[test]
fn punctuation() {
    assert_same("punct-all", &tok(b"(){}[];,."));
    assert_same("punct-nested", &tok(b"f(a[0], b.c);"));
}

#[test]
fn unknown_characters_become_error_tokens() {
    assert_same("err-at", &tok(b"@"));
    assert_same("err-hash", &tok(b"#include"));
    assert_same("err-dollar", &tok(b"$x"));
    assert_same("err-backtick", &tok(b"`"));
    assert_same("err-backslash-bare", &tok(b"\\"));
    assert_same("err-high-byte", &tok(b"\xff\x80"));
}

#[test]
fn whitespace_is_skipped_except_newlines() {
    // skip_whitespace() stops at '\n' so newlines still become tokens.
    assert_same("ws-space-tab", &tok(b"a \t b"));
    assert_same("ws-vertical-tab", &tok(b"a\x0bb"));
    assert_same("ws-form-feed", &tok(b"a\x0cb"));
    assert_same("ws-carriage-return", &tok(b"a\rb"));
    assert_same("ws-leading", &tok(b"    a"));
    assert_same("ws-all-kinds", b"6\na \t\x0b\x0c\rb\n\n3\n7\n");
}

#[test]
fn crlf_line_endings_throughout() {
    assert_same("crlf-menu-and-text", b"1\r\nfoo bar\r\n\r\n3\r\n4\r\n7\r\n");
}

// --- exhaustive single-byte sweep -----------------------------------------

#[test]
fn every_single_byte_value_classifies_identically() {
    // 0x0a would end the line and 0x00 truncates the C string, so both are
    // covered separately below.
    for b in 1u8..=255 {
        if b == b'\n' {
            continue;
        }
        let mut input = b"6\n".to_vec();
        input.push(b);
        input.extend_from_slice(b"\n\n3\n4\n7\n");
        assert_same(&format!("byte-{b:#04x}"), &input);
    }
}

#[test]
fn nul_bytes_truncate_the_c_strings_they_appear_in() {
    // strncat copies from a C string, so everything from the NUL on is lost.
    assert_same("nul-leading-in-text", b"1\n\x00abc\ndef\n\n3\n7\n");
    assert_same("nul-middle-in-text", b"1\nab\x00cd\n\n3\n7\n");
    assert_same("nul-in-tokenizer", b"6\nab\x00cd\n\n7\n");
    assert_same("nul-only-line", b"6\n\x00\n\n7\n");
}

// --- column arithmetic ----------------------------------------------------

#[test]
fn token_columns_wrap_negative_for_tokens_at_the_line_start() {
    // create_token() computes `current_column - token.length` in size_t and
    // truncates back to int, so a token starting at column 1 reports a
    // non-positive column.
    assert_same("col-line-start", &tok(b"identifier"));
    assert_same("col-multiline", &tok(b"aaaa\nbbbbbbbb\nc"));
    assert_same("col-after-spaces", &tok(b"      word"));
    let long = vec![b'z'; 255];
    let mut input = b"6\n".to_vec();
    input.extend_from_slice(&long);
    input.extend_from_slice(b"\n\n7\n");
    assert_same("col-long-token", &input);
}
