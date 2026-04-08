use libpgn::utils::buffer::PgnBuffer;
use libpgn::utils::cursor;
use libpgn::utils::export::__pgn_export;

#[test]
fn test_buffer_new_and_append() {
    let mut buf = PgnBuffer::new();
    buf.append('h');
    buf.append('i');
    let s = buf.detach();
    assert_eq!(s, "hi");
}

#[test]
fn test_buffer_concat() {
    let mut buf = PgnBuffer::new();
    buf.concat("hello");
    let s = buf.detach();
    assert_eq!(s, "hello");
}

#[test]
fn test_buffer_reset() {
    let mut buf = PgnBuffer::new();
    buf.concat("hello");
    buf.reset();
    buf.concat("world");
    let s = buf.detach();
    assert_eq!(s, "world");
}

#[test]
fn test_buffer_grow() {
    let mut buf = PgnBuffer::new();
    buf.grow();
    buf.concat("after grow");
    let s = buf.detach();
    assert_eq!(s, "after grow");
}

#[test]
fn test_buffer_append_null_terminator() {
    let mut buf = PgnBuffer::new();
    buf.concat("test");
    buf.append_null_terminator(); // no-op in Rust
    let s = buf.detach();
    assert_eq!(s, "test");
}

#[test]
fn test_cursor_skip_whitespace() {
    let s = "   hello";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    assert_eq!(skipped, true);
    assert_eq!(cur, 3);
}

#[test]
fn test_cursor_skip_whitespace_none() {
    let s = "hello";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    assert_eq!(skipped, false);
    assert_eq!(cur, 0);
}

#[test]
fn test_cursor_revisit_whitespace() {
    let s = "hi   ";
    let mut cur = 5;
    let skipped = cursor::pgn_cursor_revisit_whitespace(s, &mut cur);
    assert_eq!(skipped, true);
    assert_eq!(cur, 2);
}

#[test]
fn test_cursor_skip_comment() {
    let s = "{comment} rest";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_comment(s, &mut cur);
    assert_eq!(skipped, true);
    assert_eq!(cur, 9);
}

#[test]
fn test_cursor_skip_comment_not_comment() {
    let s = "not a comment";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_comment(s, &mut cur);
    assert_eq!(skipped, false);
    assert_eq!(cur, 0);
}

#[test]
fn test_cursor_skip_newline_lf() {
    let s = "\nrest";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_newline(s, &mut cur);
    assert_eq!(skipped, true);
    assert_eq!(cur, 1);
}

#[test]
fn test_cursor_skip_newline_crlf() {
    let s = "\r\nrest";
    let mut cur = 0;
    let skipped = cursor::pgn_cursor_skip_newline(s, &mut cur);
    assert_eq!(skipped, true);
    assert_eq!(cur, 2);
}

#[test]
fn test_export_noop() {
    __pgn_export(); // should not panic
}

fn main() {}
