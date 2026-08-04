use kairoCompiler::cprocess::{compile_process_create, compile_process_next_char, compile_process_peek_char, compile_process_push_char};
use kairoCompiler::lex_process::{lex_process_create, LexProcessFunctions, LexProcess};

fn dn(_p: &mut LexProcess) -> char { '\0' }
fn dp(_p: &mut LexProcess) -> char { '\0' }
fn dpush(_p: &mut LexProcess, _c: char) {}

fn make_test_file(path: &str, content: &[u8]) {
    std::fs::write(path, content).expect("write test file");
}

#[test]
fn test_compile_process_create_valid_file() {
    let path = "/tmp/test_cprocess_input1.txt";
    make_test_file(path, b"abc");
    let p = compile_process_create(path, "/tmp/test_cprocess_out1.bin", 5);
    assert!(p.is_some());
    let cp = p.unwrap();
    assert_eq!(cp.flags, 5);
    assert!(cp.cfile.abs_path.is_some());
}

#[test]
fn test_compile_process_create_nonexistent_input() {
    let p = compile_process_create("/tmp/nonexistent_path_123456789", "/tmp/out.bin", 0);
    assert!(p.is_none());
}

#[test]
fn test_compile_process_next_and_peek() {
    let path = "/tmp/test_cprocess_input2.txt";
    make_test_file(path, b"abc");
    let cp = compile_process_create(path, "", 0).unwrap();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let mut lp = lex_process_create(cp, funcs, None);

    // peek shouldn't consume
    let p1 = compile_process_peek_char(&mut lp);
    assert_eq!(p1, 'a');
    let p2 = compile_process_peek_char(&mut lp);
    assert_eq!(p2, 'a');

    // next consumes
    let n1 = compile_process_next_char(&mut lp);
    assert_eq!(n1, 'a');
    let n2 = compile_process_next_char(&mut lp);
    assert_eq!(n2, 'b');
    let n3 = compile_process_next_char(&mut lp);
    assert_eq!(n3, 'c');

    // After EOF, returns -1 cast
    let n4 = compile_process_next_char(&mut lp);
    assert_eq!(n4 as u8 as i8, -1i8);
}

#[test]
fn test_compile_process_push_char() {
    let path = "/tmp/test_cprocess_input3.txt";
    make_test_file(path, b"xy");
    let cp = compile_process_create(path, "", 0).unwrap();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let mut lp = lex_process_create(cp, funcs, None);

    let n1 = compile_process_next_char(&mut lp);
    assert_eq!(n1, 'x');

    // push it back
    compile_process_push_char(&mut lp, 'x');
    let p = compile_process_peek_char(&mut lp);
    assert_eq!(p, 'x');
    let n2 = compile_process_next_char(&mut lp);
    assert_eq!(n2, 'x');
}

#[test]
fn test_compile_process_newline_increments_line() {
    let path = "/tmp/test_cprocess_input4.txt";
    make_test_file(path, b"a\nb");
    let cp = compile_process_create(path, "", 0).unwrap();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let mut lp = lex_process_create(cp, funcs, None);

    let _ = compile_process_next_char(&mut lp); // 'a'
    // Compiler pos: col was 0, then col +=1 (=1)
    {
        let c = lp.compiler.as_ref().unwrap();
        assert_eq!(c.pos.col, 1);
        assert_eq!(c.pos.line, 0);
    }

    let _ = compile_process_next_char(&mut lp); // '\n'
    {
        let c = lp.compiler.as_ref().unwrap();
        assert_eq!(c.pos.line, 1);
        assert_eq!(c.pos.col, 1);
    }
}

fn main() {}
