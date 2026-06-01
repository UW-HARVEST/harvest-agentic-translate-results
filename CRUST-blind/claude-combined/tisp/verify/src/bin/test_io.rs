use tisp_proj::io::{count_parens, read_file};

#[test]
fn test_count_parens_balanced() {
    let s = "(foo (bar))";
    assert_eq!(count_parens(s, s.len() as i32), 0);
}

#[test]
fn test_count_parens_unbalanced_open() {
    let s = "(foo";
    assert_eq!(count_parens(s, s.len() as i32), 1);
}

#[test]
fn test_count_parens_unbalanced_close() {
    let s = "foo)";
    assert_eq!(count_parens(s, s.len() as i32), -1);
}

#[test]
fn test_count_parens_brackets() {
    let s = "[foo";
    assert_eq!(count_parens(s, s.len() as i32), 1);
}

#[test]
fn test_count_parens_braces() {
    let s = "{foo";
    assert_eq!(count_parens(s, s.len() as i32), 1);
}

#[test]
fn test_count_parens_empty() {
    let s = "";
    assert_eq!(count_parens(s, 0), 0);
}

#[test]
fn test_count_parens_text_only() {
    let s = "hello world";
    assert_eq!(count_parens(s, s.len() as i32), 0);
}

#[test]
fn test_count_parens_paren_takes_precedence() {
    // pcount nonzero -> return pcount; bcount and ccount ignored
    let s = "(foo[bar";
    assert_eq!(count_parens(s, s.len() as i32), 1);
}

#[test]
fn test_count_parens_nested() {
    let s = "((()))";
    assert_eq!(count_parens(s, s.len() as i32), 0);
}

#[test]
fn test_read_file_empty_name() {
    let s = read_file("");
    assert_eq!(s, "");
}

#[test]
fn test_read_file_nonexistent() {
    let s = read_file("/this/file/does/not/exist/anywhere/12345");
    assert_eq!(s, "");
}

fn main() {}
