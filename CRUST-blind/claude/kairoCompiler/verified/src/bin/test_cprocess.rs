use kairoCompiler::cprocess::{
    compile_process_create, compile_process_next_char, compile_process_peek_char,
    compile_process_push_char,
};
use kairoCompiler::lex_process::{lex_process_create, LexProcessFunctions, LexProcess};
use std::fs;
use std::io::Write;

fn dummy_next(_p: &mut LexProcess) -> char { '\u{FFFF}' }
fn dummy_peek(_p: &mut LexProcess) -> char { '\u{FFFF}' }
fn dummy_push(_p: &mut LexProcess, _c: char) {}

fn write_file(path: &str, contents: &str) {
    let mut f = fs::File::create(path).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
}

#[test]
fn test_compile_process_create_missing_file() {
    let r = compile_process_create("/nonexistent/path.txt", "/tmp/out.txt", 0);
    assert!(r.is_none());
}

#[test]
fn test_compile_process_create_valid_files() {
    let in_path = "/tmp/cprocess_test_in.txt";
    let out_path = "/tmp/cprocess_test_out.txt";
    write_file(in_path, "hello");
    let r = compile_process_create(in_path, out_path, 5);
    let p = r.expect("should succeed");
    assert_eq!(p.flags, 5);
    assert_eq!(p.pos.line, 1);
    assert_eq!(p.pos.col, 1);
    assert_eq!(p.pos.filename, Some(in_path.to_string()));
    assert!(p.cfile.fp.is_some());
    assert_eq!(p.cfile.abs_path, Some(in_path.to_string()));
    assert!(p.ofile.is_some());
    assert!(p.node_vec.is_some());
    assert!(p.node_tree_vec.is_some());
    let _ = fs::remove_file(in_path);
    let _ = fs::remove_file(out_path);
}

#[test]
fn test_compile_process_create_no_output() {
    let in_path = "/tmp/cprocess_test_in_no.txt";
    write_file(in_path, "abc");
    let r = compile_process_create(in_path, "", 0);
    let p = r.expect("should succeed");
    assert!(p.ofile.is_none());
    assert!(p.cfile.fp.is_some());
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_next_char_basic() {
    let in_path = "/tmp/cprocess_test_nc.txt";
    write_file(in_path, "abc");
    let process = compile_process_create(in_path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c1 = compile_process_next_char(&mut lp);
    assert_eq!(c1, 'a');
    let c2 = compile_process_next_char(&mut lp);
    assert_eq!(c2, 'b');
    let c3 = compile_process_next_char(&mut lp);
    assert_eq!(c3, 'c');
    // Verify col was incremented
    let compiler = lp.compiler.as_ref().unwrap();
    assert_eq!(compiler.pos.col, 4);
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_next_char_newline_resets_col() {
    let in_path = "/tmp/cprocess_test_nl.txt";
    write_file(in_path, "a\nb");
    let process = compile_process_create(in_path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c1 = compile_process_next_char(&mut lp);
    assert_eq!(c1, 'a');
    let c2 = compile_process_next_char(&mut lp);
    assert_eq!(c2, '\n');
    {
        let compiler = lp.compiler.as_ref().unwrap();
        assert_eq!(compiler.pos.line, 2);
        assert_eq!(compiler.pos.col, 1);
    }
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_peek_char() {
    let in_path = "/tmp/cprocess_test_pk.txt";
    write_file(in_path, "xy");
    let process = compile_process_create(in_path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let p1 = compile_process_peek_char(&mut lp);
    assert_eq!(p1, 'x');
    let p2 = compile_process_peek_char(&mut lp);
    assert_eq!(p2, 'x');  // peek doesn't consume
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_next_char_eof() {
    let in_path = "/tmp/cprocess_test_eof.txt";
    write_file(in_path, "");
    let process = compile_process_create(in_path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c = compile_process_next_char(&mut lp);
    assert_eq!(c as u32, 0xFFFF);
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_push_char() {
    let in_path = "/tmp/cprocess_test_push.txt";
    write_file(in_path, "abc");
    let process = compile_process_create(in_path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c1 = compile_process_next_char(&mut lp);
    assert_eq!(c1, 'a');
    // Push back
    compile_process_push_char(&mut lp, 'a');
    // Should be able to read 'a' again
    let c2 = compile_process_next_char(&mut lp);
    assert_eq!(c2, 'a');
    let _ = fs::remove_file(in_path);
}

fn main() {}
