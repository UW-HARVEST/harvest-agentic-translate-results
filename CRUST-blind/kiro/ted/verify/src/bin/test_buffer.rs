use ted::buffer::TextBuffer;

#[test]
fn test_create() {
    let tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.last_line_loc, 0);
    assert_eq!(tb.lines_capacity, 10);
    assert_eq!(tb.get_line(0), Some(String::new()));
}

#[test]
fn test_insert_char() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.insert('a'), 0);
    assert_eq!(tb.cursor_col, 1);
    assert_eq!(tb.get_line(0), Some("a".to_string()));
}

#[test]
fn test_insert_11_chars() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..11 {
        assert_eq!(tb.insert('a'), 0);
    }
    assert_eq!(tb.get_line(0), Some("aaaaaaaaaaa".to_string()));
}

#[test]
fn test_backspace() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..11 {
        tb.insert('a');
    }
    tb.backspace();
    assert_eq!(tb.get_line(0), Some("aaaaaaaaaa".to_string()));
}

#[test]
fn test_move_cursor_and_newline() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..10 {
        tb.insert('a');
    }
    tb.move_cursor(tb.cursor_row, tb.cursor_col - 1);
    assert_eq!(tb.new_line(), 0);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.get_line(1), Some("a".to_string()));
    assert_eq!(tb.get_line(0), Some("aaaaaaaaa".to_string()));
}

#[test]
fn test_get_line_out_of_bounds() {
    let tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.get_line(5), None);
}

#[test]
fn test_move_cursor_clamp_row() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('a');
    tb.move_cursor(100, 0);
    assert_eq!(tb.cursor_row, 0);
}

#[test]
fn test_move_cursor_clamp_col() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('a');
    tb.move_cursor(0, 100);
    assert_eq!(tb.cursor_col, 1);
}

#[test]
fn test_newline_splits_correctly() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('a');
    }
    tb.new_line();
    for _ in 0..3 {
        tb.insert('b');
    }
    assert_eq!(tb.last_line_loc, 1);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 3);
    assert_eq!(tb.get_line(0), Some("aaaaa".to_string()));
    assert_eq!(tb.get_line(1), Some("bbb".to_string()));
}

fn main() {}
