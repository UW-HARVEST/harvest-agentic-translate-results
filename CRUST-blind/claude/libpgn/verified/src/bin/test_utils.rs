#[allow(unused_imports)]
use libpgn::utils::buffer::PgnBuffer;
#[allow(unused_imports)]
use libpgn::utils::cursor::{
    pgn_cursor_revisit_whitespace, pgn_cursor_skip_comment, pgn_cursor_skip_newline,
    pgn_cursor_skip_whitespace,
};

#[test]
fn test_buffer_new_empty() {
    let b = PgnBuffer::new();
    assert_eq!(b.as_str(), "");
}

#[test]
fn test_buffer_append() {
    let mut b = PgnBuffer::new();
    b.append('h');
    b.append('i');
    assert_eq!(b.as_str(), "hi");
}

#[test]
fn test_buffer_concat() {
    let mut b = PgnBuffer::new();
    b.concat("hello");
    b.concat(" world");
    assert_eq!(b.as_str(), "hello world");
}

#[test]
fn test_buffer_reset() {
    let mut b = PgnBuffer::new();
    b.concat("data");
    b.reset();
    assert_eq!(b.as_str(), "");
}

#[test]
fn test_buffer_equals() {
    let mut b = PgnBuffer::new();
    b.concat("foo");
    assert!(b.equals("foo"));
    assert!(!b.equals("bar"));
}

#[test]
fn test_buffer_detach() {
    let mut b = PgnBuffer::new();
    b.concat("xyz");
    let s = b.detach();
    assert_eq!(s, "xyz");
}

#[test]
fn test_cursor_skip_whitespace() {
    let mut c = 0usize;
    let s = "   hello";
    let skipped = pgn_cursor_skip_whitespace(s, &mut c);
    assert!(skipped);
    assert_eq!(c, 3);

    let mut c = 0usize;
    let s = "hello";
    let skipped = pgn_cursor_skip_whitespace(s, &mut c);
    assert!(!skipped);
    assert_eq!(c, 0);

    let mut c = 0usize;
    let s = "\t\n hi";
    let skipped = pgn_cursor_skip_whitespace(s, &mut c);
    assert!(skipped);
    assert_eq!(c, 3);
}

#[test]
fn test_cursor_revisit_whitespace() {
    let s = "abc   ";
    let mut c = 6usize;
    let r = pgn_cursor_revisit_whitespace(s, &mut c);
    assert!(r);
    assert_eq!(c, 3);

    let mut c = 3usize;
    let r = pgn_cursor_revisit_whitespace(s, &mut c);
    assert!(!r);
    assert_eq!(c, 3);
}

#[test]
fn test_cursor_skip_comment_simple() {
    let s = "{hello} more";
    let mut c = 0usize;
    let r = pgn_cursor_skip_comment(s, &mut c);
    assert!(r);
    assert_eq!(c, 7);
}

#[test]
fn test_cursor_skip_comment_nested() {
    let s = "{a {b} c} after";
    let mut c = 0usize;
    let r = pgn_cursor_skip_comment(s, &mut c);
    assert!(r);
    assert_eq!(c, 9);
}

#[test]
fn test_cursor_skip_comment_no_brace() {
    let s = "hello";
    let mut c = 0usize;
    let r = pgn_cursor_skip_comment(s, &mut c);
    assert!(!r);
    assert_eq!(c, 0);
}

#[test]
fn test_cursor_skip_newline_lf() {
    let s = "\nhi";
    let mut c = 0usize;
    let r = pgn_cursor_skip_newline(s, &mut c);
    assert!(r);
    assert_eq!(c, 1);
}

#[test]
fn test_cursor_skip_newline_crlf() {
    let s = "\r\nhi";
    let mut c = 0usize;
    let r = pgn_cursor_skip_newline(s, &mut c);
    assert!(r);
    assert_eq!(c, 2);
}

fn main() {}
