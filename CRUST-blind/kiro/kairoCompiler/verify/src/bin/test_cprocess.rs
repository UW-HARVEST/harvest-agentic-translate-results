use kairoCompiler::cprocess::*;
use kairoCompiler::compiler::{LexProcess, LexProcessFunctions, Pos};
use kairoCompiler::vector::vector_create;
use std::io::Write;

fn create_temp_file(name: &str, content: &str) -> String {
    let path = format!("/tmp/crust_test_cp_{}", name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn make_lp(path: &str) -> LexProcess {
    let cp = compile_process_create(path, "", 0).unwrap();
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    LexProcess {
        compiler: Some(Box::new(cp)),
        function: Some(funcs),
        token_vec: Some(vector_create(1)),
        pos: Pos { line: 1, col: 1, filename: None },
        ..Default::default()
    }
}

#[test]
fn test_compile_process_create_valid() {
    let path = create_temp_file("valid.txt", "hello");
    let cp = compile_process_create(&path, "", 0);
    assert!(cp.is_some());
    let cp = cp.unwrap();
    assert_eq!(cp.flags, 0);
    assert!(cp.cfile.fp.is_some());
    assert_eq!(cp.cfile.abs_path, Some(path));
}

#[test]
fn test_compile_process_create_invalid() {
    let cp = compile_process_create("/nonexistent/file.txt", "", 0);
    assert!(cp.is_none());
}

#[test]
fn test_compile_process_next_char() {
    let path = create_temp_file("next.txt", "AB");
    let mut lp = make_lp(&path);
    assert_eq!(compile_process_next_char(&mut lp), 'A');
    assert_eq!(compile_process_next_char(&mut lp), 'B');
}

#[test]
fn test_compile_process_peek_char() {
    let path = create_temp_file("peek.txt", "XY");
    let mut lp = make_lp(&path);
    assert_eq!(compile_process_peek_char(&mut lp), 'X');
    assert_eq!(compile_process_peek_char(&mut lp), 'X');
    assert_eq!(compile_process_next_char(&mut lp), 'X');
    assert_eq!(compile_process_peek_char(&mut lp), 'Y');
}

#[test]
fn test_compile_process_push_char() {
    let path = create_temp_file("push.txt", "AB");
    let mut lp = make_lp(&path);
    assert_eq!(compile_process_next_char(&mut lp), 'A');
    compile_process_push_char(&mut lp, 'A');
    assert_eq!(compile_process_next_char(&mut lp), 'A');
}

fn main() {}
