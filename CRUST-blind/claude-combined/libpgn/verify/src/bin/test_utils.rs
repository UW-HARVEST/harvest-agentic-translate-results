use libpgn::utils::buffer::{PgnBuffer, PGN_BUFFER_INITIAL_SIZE, PGN_BUFFER_GROW_SIZE};
use libpgn::utils::cursor::{
    pgn_cursor_revisit_whitespace, pgn_cursor_skip_comment, pgn_cursor_skip_newline,
    pgn_cursor_skip_whitespace,
};
use libpgn::utils::export::__pgn_export;

#[test]
fn test_buffer_constants() {
    assert_eq!(PGN_BUFFER_INITIAL_SIZE, 16);
    assert_eq!(PGN_BUFFER_GROW_SIZE, 32);
}

#[test]
fn test_buffer_new_empty() {
    let b = PgnBuffer::new();
    assert!(b.is_empty());
    assert_eq!(b.len(), 0);
    assert_eq!(b.as_str(), "");
}

#[test]
fn test_buffer_append_and_concat() {
    let mut b = PgnBuffer::new();
    b.append('a');
    b.append('b');
    b.append('c');
    assert_eq!(b.as_str(), "abc");
    assert_eq!(b.len(), 3);

    b.concat("def");
    assert_eq!(b.as_str(), "abcdef");
    assert_eq!(b.len(), 6);
}

#[test]
fn test_buffer_reset() {
    let mut b = PgnBuffer::new();
    b.concat("hello");
    assert_eq!(b.as_str(), "hello");
    b.reset();
    assert!(b.is_empty());
    assert_eq!(b.as_str(), "");
}

#[test]
fn test_buffer_grow_no_panic() {
    let mut b = PgnBuffer::new();
    // Should not panic.
    b.grow();
    b.grow();
    // Buffer should still work.
    b.append('x');
    assert_eq!(b.as_str(), "x");
}

#[test]
fn test_buffer_detach() {
    let mut b = PgnBuffer::new();
    b.concat("hello");
    let s = b.detach();
    assert_eq!(s, "hello");
}

#[test]
fn test_buffer_equal() {
    let mut b = PgnBuffer::new();
    b.concat("foo");
    assert!(b.equal("foo"));
    assert!(!b.equal("bar"));
}

#[test]
fn test_buffer_append_null_terminator_no_panic() {
    let mut b = PgnBuffer::new();
    b.append_null_terminator();
    // Rust strings aren't null-terminated so this should be a no-op.
    assert_eq!(b.as_str(), "");
    assert_eq!(b.len(), 0);
}

#[test]
fn test_cursor_skip_whitespace_none() {
    let mut cursor = 0usize;
    let s = "abc";
    let skipped = pgn_cursor_skip_whitespace(s, &mut cursor);
    assert!(!skipped);
    assert_eq!(cursor, 0);
}

#[test]
fn test_cursor_skip_whitespace() {
    let mut cursor = 0usize;
    let s = "   abc";
    let skipped = pgn_cursor_skip_whitespace(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 3);
}

#[test]
fn test_cursor_skip_whitespace_at_end() {
    let mut cursor = 0usize;
    let s = "   ";
    let skipped = pgn_cursor_skip_whitespace(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 3);
}

#[test]
fn test_cursor_revisit_whitespace_none() {
    let mut cursor = 3usize;
    let s = "abc";
    let revisited = pgn_cursor_revisit_whitespace(s, &mut cursor);
    assert!(!revisited);
    assert_eq!(cursor, 3);
}

#[test]
fn test_cursor_revisit_whitespace() {
    let mut cursor = 6usize;
    let s = "abc   ";
    let revisited = pgn_cursor_revisit_whitespace(s, &mut cursor);
    assert!(revisited);
    assert_eq!(cursor, 3);
}

#[test]
fn test_cursor_skip_comment_simple() {
    let mut cursor = 0usize;
    let s = "{hello}";
    let skipped = pgn_cursor_skip_comment(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 7);
}

#[test]
fn test_cursor_skip_comment_nested() {
    let mut cursor = 0usize;
    let s = "{a {b} c}";
    let skipped = pgn_cursor_skip_comment(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 9);
}

#[test]
fn test_cursor_skip_comment_not_a_comment() {
    let mut cursor = 0usize;
    let s = "abc";
    let skipped = pgn_cursor_skip_comment(s, &mut cursor);
    assert!(!skipped);
    assert_eq!(cursor, 0);
}

#[test]
fn test_cursor_skip_newline_lf() {
    let mut cursor = 0usize;
    let s = "\nabc";
    let skipped = pgn_cursor_skip_newline(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 1);
}

#[test]
fn test_cursor_skip_newline_crlf() {
    let mut cursor = 0usize;
    let s = "\r\nabc";
    let skipped = pgn_cursor_skip_newline(s, &mut cursor);
    assert!(skipped);
    assert_eq!(cursor, 2);
}

#[test]
fn test_export_no_panic() {
    // Just ensure it can be called.
    __pgn_export();
}

fn main() {}
