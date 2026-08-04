use SlothLang::parser;
use std::io::Write;

fn write_temp(name: &str, contents: &[u8]) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("slothlang_test_{}.sloth", name));
    let mut f = std::fs::File::create(&path).expect("create temp file");
    f.write_all(contents).expect("write temp file");
    path
}

fn parse_codes(name: &str, contents: &[u8]) -> Vec<u8> {
    let path = write_temp(name, contents);
    let prog = parser::parse(path.to_str().unwrap()).expect("parse returned None");
    prog.codes
}

#[test]
fn test_parse_just_slothy() {
    // "slothy\n" -> 1 newline, numCodes = 3.
    // Output: [PUSH(1), 0 (currentCode), 0 (uninitialized but Rust zeroes)]
    let codes = parse_codes("just_slothy", b"slothy\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_just_nap() {
    // "nap\n" -> [0, 0, 0] (nap writes 0, then hadToken writes currentCode=0)
    let codes = parse_codes("just_nap", b"nap\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 0);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_sloth_and() {
    // "sloth and\n" -> sloth increments currentCode to 1, "and" writes 1 and resets,
    // then hadToken writes 0. Output: [1, 0, 0]
    let codes = parse_codes("push1", b"sloth and\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_two_slothy() {
    // "slothy slothy\n" -> two PUSH bytes (each 1), then trailing currentCode 0.
    // Output: [1, 1, 0]
    let codes = parse_codes("two_slothy", b"slothy slothy\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 1);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_slothy_three_sloth_with_comment() {
    // "slothy sloth sloth sloth # push 3\n" -> PUSH 3, then trailing currentCode 0?
    // Actually: slothy -> byteCode[0]=1, then 3 sloth -> currentCode=3, then '#' -> exits.
    // hadToken triggers byteCode[1]=3. Output: [1, 3, 0]
    let codes = parse_codes("push3", b"slothy sloth sloth sloth # push 3\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 3);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_only_comment_line() {
    // "# nothing\n" -> 1 newline, numCodes=3, but no tokens written.
    // Output: [0, 0, 0]
    let codes = parse_codes("only_comment", b"# nothing\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 0);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_blank_line() {
    // "\n" -> 1 newline, numCodes=3, len==0 => skipped, no tokens.
    let codes = parse_codes("blank", b"\n");
    assert_eq!(codes.len(), 3);
    assert_eq!(codes[0], 0);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
}

#[test]
fn test_parse_comment_then_token() {
    // "# this is a comment\nsloth and\n" -> 2 newlines, numCodes=6.
    // Comment line writes nothing, "sloth and" writes [1, 0]. Rest is zero.
    let codes = parse_codes("comment_token", b"# this is a comment\nsloth and\n");
    assert_eq!(codes.len(), 6);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
    assert_eq!(codes[3], 0);
    assert_eq!(codes[4], 0);
    assert_eq!(codes[5], 0);
}

#[test]
fn test_parse_multiline_program() {
    // 2 lines: "sloth and\n" and "nap\n"
    // Line 1: byteCode[0]=1 (and), byteCode[1]=0 (currentCode trailing)
    // Line 2: byteCode[2]=0 (nap), byteCode[3]=0 (trailing)
    let codes = parse_codes("multiline", b"sloth and\nnap\n");
    assert_eq!(codes.len(), 6);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 0);
    assert_eq!(codes[2], 0);
    assert_eq!(codes[3], 0);
    assert_eq!(codes[4], 0);
    assert_eq!(codes[5], 0);
}

#[test]
fn test_parse_push_then_nap() {
    // "slothy sloth and\nnap\n"
    // Line 1: slothy -> byteCode[0]=1, sloth -> currentCode=1, and -> byteCode[1]=1, count=...
    //         after loop ends, hadToken -> byteCode[2]=0
    // Line 2: nap -> byteCode[3]=0, hadToken -> byteCode[4]=0
    let codes = parse_codes("push_then_nap", b"slothy sloth and\nnap\n");
    assert_eq!(codes.len(), 6);
    assert_eq!(codes[0], 1);
    assert_eq!(codes[1], 1);
    assert_eq!(codes[2], 0);
    assert_eq!(codes[3], 0);
    assert_eq!(codes[4], 0);
    assert_eq!(codes[5], 0);
}

#[test]
fn test_parse_helloworld_first_codes() {
    // From the C tests.c: HelloWorld parse should have codes[0..3] == [1, 3, 1].
    // codes[14] == 8 and codes[17] == 10.
    // We embed a path to the example file in c_src.
    let path = "c_src/Examples/HelloWorld.sloth";
    let prog = parser::parse(path).expect("HelloWorld parse failed");
    assert_eq!(prog.codes[0], 1);
    assert_eq!(prog.codes[1], 3);
    assert_eq!(prog.codes[2], 1);
    assert_eq!(prog.codes[14], 8);
    assert_eq!(prog.codes[17], 10);
}

#[test]
fn test_parse_count_program() {
    // The c_src tests.c verifies that executing Count.sloth returns 11.
    // We additionally check the first few parsed codes.
    let path = "c_src/Examples/Count.sloth";
    let prog = parser::parse(path).expect("Count parse failed");
    // From running C dump_parse: 1 1 10 8 1 1 10 8 2 1 1 2 10 1 11 3 1 1 1 1 3 6 3 9 2 0 ...
    let expected_prefix = [1u8, 1, 10, 8, 1, 1, 10, 8, 2, 1, 1, 2, 10, 1, 11, 3, 1, 1, 1, 1, 3, 6, 3, 9, 2, 0];
    for (i, e) in expected_prefix.iter().enumerate() {
        assert_eq!(prog.codes[i], *e,
            "mismatch at index {}: got {} expected {}", i, prog.codes[i], e);
    }
}

#[test]
fn test_parse_count_executes_to_11() {
    use SlothLang::slothvm::execute;
    let mut prog = parser::parse("c_src/Examples/Count.sloth");
    let r = execute(&mut prog);
    assert_eq!(r, 11);
}

#[test]
fn test_prog_len_counts_newlines_times_three() {
    // prog_len returns number of '\n' characters * 3.
    use std::fs::File;
    let path = write_temp("plen", b"a\nb\nc\n");
    let f = File::open(&path).unwrap();
    let n = parser::prog_len(&f);
    assert_eq!(n, 9); // 3 newlines * 3
}

#[test]
fn test_prog_len_no_newline() {
    use std::fs::File;
    let path = write_temp("plen_none", b"abcdef");
    let f = File::open(&path).unwrap();
    let n = parser::prog_len(&f);
    assert_eq!(n, 0);
}

#[test]
fn test_prog_len_only_newlines() {
    use std::fs::File;
    let path = write_temp("plen_nl", b"\n\n\n\n\n");
    let f = File::open(&path).unwrap();
    let n = parser::prog_len(&f);
    assert_eq!(n, 15);
}

#[test]
fn test_readline_strips_newline() {
    use std::fs::File;
    let path = write_temp("readline_simple", b"hello\nworld\n");
    let f = File::open(&path).unwrap();
    let line = parser::readline(&f).expect("readline returned None");
    // The first line should be "hello" without the trailing newline.
    assert_eq!(line, "hello");
}

#[test]
fn test_free_program_consumes_program() {
    use SlothLang::slothvm::SlothProgram;
    // free_program just consumes the Option; ensure no panic.
    let p = Some(SlothProgram { codes: vec![1, 2, 3], pc: 0 });
    parser::free_program(p);
    parser::free_program(None);
}

#[test]
fn test_parse_returns_some() {
    let path = write_temp("returns_some", b"sloth and\n");
    let prog = parser::parse(path.to_str().unwrap());
    assert!(prog.is_some());
    let p = prog.unwrap();
    assert_eq!(p.pc, 0);
    assert_eq!(p.codes.len(), 3);
}

fn main() {}
