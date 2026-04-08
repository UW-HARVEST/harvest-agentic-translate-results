use kairoCompiler::cprocess::compile_process_create;
use std::io::Write;

fn create_temp_file(name: &str, content: &str) -> String {
    let path = format!("/tmp/crust_test_{}.txt", name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.sync_all().unwrap();
    path
}

fn create_out_file(name: &str) -> String {
    let path = format!("/tmp/crust_test_{}_out.txt", name);
    std::fs::File::create(&path).unwrap();
    path
}

#[test]
fn test_compile_process_create_valid_file() {
    let path = create_temp_file("cp_valid", "hello");
    let out_path = create_out_file("cp_valid");
    let result = compile_process_create(&path, &out_path, 0);
    assert!(result.is_some());
    let cp = result.unwrap();
    assert_eq!(cp.flags, 0);
    assert_eq!(cp.file_contents, b"hello");
}

#[test]
fn test_compile_process_create_invalid_file() {
    let result = compile_process_create("/nonexistent/file.txt", "/tmp/out.txt", 0);
    assert!(result.is_none());
}

#[test]
fn test_compile_process_next_char() {
    let path = create_temp_file("cp_next", "AB\nC");
    let out_path = create_out_file("cp_next");
    let cp = compile_process_create(&path, &out_path, 0).unwrap();

    let funcs = kairoCompiler::lex_process::LexProcessFunctions {
        next_char: kairoCompiler::cprocess::compile_process_next_char,
        peek_char: kairoCompiler::cprocess::compile_process_peek_char,
        push_char: kairoCompiler::cprocess::compile_process_push_char,
    };
    let mut lp = kairoCompiler::lex_process::lex_process_create(cp, funcs, None);

    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'A');
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'B');
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), '\n');
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'C');
}

#[test]
fn test_compile_process_peek_char() {
    let path = create_temp_file("cp_peek", "XY");
    let out_path = create_out_file("cp_peek");
    let cp = compile_process_create(&path, &out_path, 0).unwrap();

    let funcs = kairoCompiler::lex_process::LexProcessFunctions {
        next_char: kairoCompiler::cprocess::compile_process_next_char,
        peek_char: kairoCompiler::cprocess::compile_process_peek_char,
        push_char: kairoCompiler::cprocess::compile_process_push_char,
    };
    let mut lp = kairoCompiler::lex_process::lex_process_create(cp, funcs, None);

    assert_eq!(kairoCompiler::cprocess::compile_process_peek_char(&mut lp), 'X');
    assert_eq!(kairoCompiler::cprocess::compile_process_peek_char(&mut lp), 'X');
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'X');
    assert_eq!(kairoCompiler::cprocess::compile_process_peek_char(&mut lp), 'Y');
}

#[test]
fn test_compile_process_push_char() {
    let path = create_temp_file("cp_push", "AB");
    let out_path = create_out_file("cp_push");
    let cp = compile_process_create(&path, &out_path, 0).unwrap();

    let funcs = kairoCompiler::lex_process::LexProcessFunctions {
        next_char: kairoCompiler::cprocess::compile_process_next_char,
        peek_char: kairoCompiler::cprocess::compile_process_peek_char,
        push_char: kairoCompiler::cprocess::compile_process_push_char,
    };
    let mut lp = kairoCompiler::lex_process::lex_process_create(cp, funcs, None);

    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'A');
    kairoCompiler::cprocess::compile_process_push_char(&mut lp, 'A');
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'A');
}

#[test]
fn test_compile_process_eof() {
    let path = create_temp_file("cp_eof", "A");
    let out_path = create_out_file("cp_eof");
    let cp = compile_process_create(&path, &out_path, 0).unwrap();

    let funcs = kairoCompiler::lex_process::LexProcessFunctions {
        next_char: kairoCompiler::cprocess::compile_process_next_char,
        peek_char: kairoCompiler::cprocess::compile_process_peek_char,
        push_char: kairoCompiler::cprocess::compile_process_push_char,
    };
    let mut lp = kairoCompiler::lex_process::lex_process_create(cp, funcs, None);

    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 'A');
    // EOF returns 0xFF
    assert_eq!(kairoCompiler::cprocess::compile_process_peek_char(&mut lp), 0xFF as char);
    assert_eq!(kairoCompiler::cprocess::compile_process_next_char(&mut lp), 0xFF as char);
}

fn main() {}
