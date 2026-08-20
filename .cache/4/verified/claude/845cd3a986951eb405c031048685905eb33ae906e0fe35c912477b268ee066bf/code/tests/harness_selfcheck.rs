//! Harness self-check: proves the differential tests really observe the
//! libraries' data (a harness that silently compared two empty vectors would
//! make every other test vacuous).  The expected values are read off the C
//! source by hand.

mod common;

use common::*;

#[test]
fn tokens_are_real_and_identical() {
    let _g = lock();
    let p = libs();
    let text = b"if x1 /*c*/ \"s\" 1.5 >= ; #";
    assert_eq!(p.c.load_text(text), 0);
    assert_eq!(p.rust.load_text(text), 0);
    let tc = p.c.drain_tokens();
    let tr = p.rust.drain_tokens();
    assert_eq!(tc, tr);

    // 8 tokens + the final TOKEN_EOF
    assert_eq!(tc.len(), 9, "{:?}", tc);

    let expect: [(i32, &[u8], i32); 9] = [
        (TOKEN_KEYWORD, b"if", 1),
        (TOKEN_IDENTIFIER, b"x1", 4),
        (TOKEN_COMMENT, b"/*c*/", 7),
        (TOKEN_STRING, b"\"s\"", 13),
        (TOKEN_NUMBER, b"1.5", 17),
        (TOKEN_OPERATOR, b">=", 21),
        (TOKEN_PUNCTUATION, b";", 24),
        (TOKEN_ERROR, b"#", 26),
        (TOKEN_EOF, b"", 27),
    ];
    for (i, (ty, val, col)) in expect.iter().enumerate() {
        assert_eq!(tc[i].ttype, *ty, "token {} type", i);
        assert_eq!(tc[i].value, val.to_vec(), "token {} value", i);
        assert_eq!(tc[i].length, val.len(), "token {} length", i);
        assert_eq!(tc[i].line, 1, "token {} line", i);
        // create_token: column = current_column - token.length
        assert_eq!(tc[i].column, *col, "token {} column", i);
    }
}

#[test]
fn stdout_capture_sees_library_output() {
    let _g = lock();
    let p = libs();
    let c = p.c.captured(|| (p.c.print_menu)());
    let r = p.rust.captured(|| (p.rust.print_menu)());
    assert!(c.starts_with(b"\n=== Text Analyzer ===\n"), "{}", show(&c));
    assert!(c.ends_with(b"Choice: "), "{}", show(&c));
    assert!(c.len() > 100, "unexpectedly short menu: {}", show(&c));
    assert_eq!(show(&c), show(&r));
}

#[test]
fn analysis_result_is_non_trivial() {
    let _g = lock();
    let p = libs();
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());
    let text = b"int x = 1; // c\nreturn \"s\";\n";
    let rc = p.c.analyze(text);
    let rr = p.rust.analyze(text);
    assert_eq!(rc, rr);
    assert_eq!(rc.word_count, 1); // x
    assert_eq!(rc.keyword_count, 2); // int, return
    assert_eq!(rc.number_count, 1); // 1
    assert_eq!(rc.operator_count, 1); // =
    assert_eq!(rc.comment_count, 1); // // c
    assert_eq!(rc.string_count, 1); // "s"
    // line_count/char_count are taken from the *cumulative* tokenizer counters,
    // which other tests in this process may already have advanced.
    assert!(rc.char_count >= text.len());
    assert!(rc.line_count >= 2);
}
