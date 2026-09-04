//! Differential tests for menu entry `1` (analyze text) and, through it, every
//! branch of `tokenizer_next_token`.

mod common;

use common::assert_same;

/// Build a session that analyses `text`, then dumps the distribution and the
/// complexity score, then exits.
fn analyze(text: &[u8]) -> Vec<u8> {
    let mut input = Vec::from(&b"1\n"[..]);
    input.extend_from_slice(text);
    input.extend_from_slice(b"\n3\n4\n7\n");
    input
}

#[test]
fn empty_text() {
    // The very first line is empty, so nothing is accumulated at all.
    assert_same("analyze_empty", b"1\n\n3\n4\n7\n");
}

#[test]
fn eof_during_text_entry() {
    assert_same("eof_right_after_1", b"1\n");
    assert_same("eof_mid_text", b"1\nabc");
    assert_same("eof_after_full_line", b"1\nabc\n");
}

#[test]
fn single_item() {
    assert_same("one_word", &analyze(b"hello"));
    assert_same("one_number", &analyze(b"42"));
    assert_same("one_operator", &analyze(b"+"));
    assert_same("one_punct", &analyze(b";"));
    assert_same("one_unknown", &analyze(b"@"));
}

#[test]
fn every_keyword() {
    let keywords = "if else while for return int char float double void struct \
                    typedef const static extern auto register sizeof break \
                    continue switch case default do goto enum union signed \
                    unsigned long short";
    assert_same("keywords", &analyze(keywords.as_bytes()));
    // Near-misses must be identifiers, not keywords.
    assert_same("near_keywords", &analyze(b"iff els INT Int _if if_ ifelse"));
}

#[test]
fn every_operator() {
    assert_same(
        "two_char_operators",
        &analyze(b"== != <= >= && || ++ -- -> << >>"),
    );
    assert_same(
        "one_char_operators",
        &analyze(b"+ - * % = < > ! & | ^ ~ ? :"),
    );
    // Combinations that are *not* two-character operators.
    assert_same("almost_operators", &analyze(b"=! <> &| ^^ ~~ ?? :: +- -+"));
}

#[test]
fn punctuation_and_error_tokens() {
    assert_same("punctuation", &analyze(b"(){}[];,."));
    assert_same("error_tokens", &analyze(b"@ # $ ` \\ \x01\x02"));
}

#[test]
fn numbers() {
    assert_same("numbers", &analyze(b"0 42 007 3.14 1.2.3 .5 1. 9. 0.0.0"));
    let long_number: Vec<u8> = std::iter::repeat(b'9').take(300).collect();
    assert_same("long_number", &analyze(&long_number));
    let mut dots = Vec::new();
    for _ in 0..100 {
        dots.extend_from_slice(b"1.");
    }
    assert_same("many_dots", &analyze(&dots));
}

#[test]
fn strings() {
    assert_same("double_quoted", &analyze(b"\"hello world\""));
    assert_same("single_quoted", &analyze(b"'c'"));
    assert_same("escaped_quote", &analyze(b"\"a\\\"b\""));
    assert_same("unterminated", &analyze(b"\"abc"));
    assert_same("unterminated_single", &analyze(b"'abc"));
    assert_same("lone_quote", &analyze(b"\""));
    assert_same("lone_apostrophe", &analyze(b"'"));
    assert_same("empty_string", &analyze(b"\"\" ''"));
    assert_same("backslash_at_end", &analyze(b"\"abc\\"));
    assert_same("quote_last_byte", b"1\nx \"\n\n3\n7\n");
}

#[test]
fn string_length_boundaries() {
    // scan_string stops at MAX_TOKEN_LENGTH - 2 == 254 accumulated bytes; the
    // closing quote can push the buffer to 256, which create_token truncates
    // back to 255.
    for n in [252usize, 253, 254, 255, 256, 257, 300] {
        let mut text = Vec::from(&b"\""[..]);
        text.extend(std::iter::repeat(b'a').take(n));
        text.extend_from_slice(b"\"");
        assert_same(&format!("string_len_{n}"), &analyze(&text));
    }
    // The same boundary reached through escape pairs, which append two bytes at
    // a time and can therefore overshoot the limit.
    for n in [125usize, 126, 127, 128] {
        let mut text = Vec::from(&b"\""[..]);
        for _ in 0..n {
            text.extend_from_slice(b"\\a");
        }
        text.extend_from_slice(b"\"");
        assert_same(&format!("string_escapes_{n}"), &analyze(&text));
    }
}

#[test]
fn identifier_length_boundaries() {
    for n in [254usize, 255, 256, 257, 300] {
        let ident: Vec<u8> = std::iter::repeat(b'q').take(n).collect();
        assert_same(&format!("ident_len_{n}"), &analyze(&ident));
    }
}

#[test]
fn comments() {
    assert_same("line_comment", &analyze(b"// a comment"));
    assert_same("line_comment_then_code", b"1\n// c\nint x;\n\n3\n4\n7\n");
    assert_same("empty_line_comment", &analyze(b"//"));
    assert_same("block_comment", &analyze(b"/* a comment */"));
    assert_same("empty_block_comment", &analyze(b"/**/"));
    assert_same("stars_block_comment", &analyze(b"/***/"));
    assert_same("unterminated_block", &analyze(b"/* abc"));
    assert_same("star_only", &analyze(b"/*"));
    assert_same("lone_slash", &analyze(b"/"));
    assert_same("slash_equals", &analyze(b"/= /% //= /*/"));
    assert_same("close_without_open", &analyze(b"*/"));
    assert_same("multiline_block", b"1\n/* one\ntwo\nthree */\n\n3\n4\n7\n");
}

#[test]
fn comment_length_boundaries() {
    for n in [252usize, 253, 254, 255, 256, 300] {
        let mut line = Vec::from(&b"//"[..]);
        line.extend(std::iter::repeat(b'c').take(n));
        assert_same(&format!("line_comment_{n}"), &analyze(&line));

        let mut block = Vec::from(&b"/*"[..]);
        block.extend(std::iter::repeat(b'c').take(n));
        block.extend_from_slice(b"*/");
        assert_same(&format!("block_comment_{n}"), &analyze(&block));
    }
}

#[test]
fn whitespace_handling() {
    // skip_whitespace eats every isspace() byte except '\n'.
    assert_same("vertical_tab", &analyze(b"a\x0bb\x0cc\rd\te"));
    assert_same("leading_ws", &analyze(b"    \t  x"));
    assert_same("only_ws", &analyze(b"    \t  "));
    assert_same("crlf_text", b"1\nint x;\r\nint y;\r\n\n3\n4\n7\n");
}

#[test]
fn newline_tokens_and_line_numbers() {
    assert_same("many_lines", b"1\na\nb\nc\nd\n\n3\n4\n7\n");
    assert_same("blank_inside_impossible", b"1\na\n \nb\n\n3\n4\n7\n");
}

#[test]
fn high_bytes_and_nuls() {
    assert_same("high_bytes", b"1\n\xc3\xa9\xff\x80abc\n\n3\n4\n7\n");
    // strncat copies up to the first NUL, so everything after it is dropped.
    assert_same("nul_in_text", b"1\nabc\0def\n\n3\n4\n7\n");
    assert_same("nul_first", b"1\n\0abc\n\n3\n4\n7\n");
    assert_same("all_bytes", &{
        let mut v = Vec::from(&b"1\n"[..]);
        v.extend((1u16..10).map(|b| b as u8));
        v.extend((11u16..256).map(|b| b as u8));
        v.extend_from_slice(b"\n\n3\n4\n7\n");
        v
    });
}

#[test]
fn line_length_boundaries() {
    // fgets reads at most 255 bytes, so lines around that length are split and
    // the pieces are concatenated without an intervening newline.
    for n in [253usize, 254, 255, 256, 257, 511] {
        let mut text = vec![b'z'; n];
        text.push(b'\n');
        let mut input = Vec::from(&b"1\n"[..]);
        input.extend_from_slice(&text);
        input.extend_from_slice(b"\n3\n4\n7\n");
        assert_same(&format!("line_len_{n}"), &input);
    }
}

#[test]
fn fills_the_input_buffer() {
    // MAX_INPUT_SIZE is 4096 and strncat clamps to the remaining space.
    let mut input = Vec::from(&b"1\n"[..]);
    for _ in 0..25 {
        input.extend(std::iter::repeat(b'a').take(254));
        input.push(b'\n');
    }
    input.extend_from_slice(b"\n3\n4\n7\n");
    assert_same("fill_buffer_words", &input);

    let mut input = Vec::from(&b"1\n"[..]);
    for i in 0..25 {
        input.extend_from_slice(format!("w{i} ").as_bytes());
        input.extend(std::iter::repeat(b'x').take(240));
        input.push(b'\n');
    }
    input.extend_from_slice(b"\n3\n4\n7\n");
    assert_same("fill_buffer_mixed", &input);
}

#[test]
fn repeated_analysis_is_cumulative() {
    assert_same(
        "two_analyses",
        b"1\nint x = 1;\n\n1\nfloat y = 2.5;\n\n3\n4\n7\n",
    );
    assert_same(
        "three_analyses",
        b"1\nif (a) b++;\n\n1\n// c\n\n1\n\"s\"\n\n3\n4\n7\n",
    );
}

#[test]
fn realistic_code() {
    let code = b"1\n/* demo */\nint main(void) {\n  int i = 0;\n  for (i = 0; i < 10; i++) {\n    printf(\"%d\\n\", i);\n  }\n  return 0; // done\n}\n\n3\n4\n5\ni\n7\n";
    assert_same("realistic_code", code);
}
