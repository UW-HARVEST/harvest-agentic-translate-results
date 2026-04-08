use ted::buffer::TextBuffer;

#[test]
fn test_create() {
    let tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.last_line_loc, 0);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.get_line(0), Some(String::new()));
}

#[test]
fn test_insert_single() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.insert('a'), 0);
    assert_eq!(tb.get_line(0), Some("a".to_string()));
}

#[test]
fn test_insert_multiple() {
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
    assert_eq!(tb.backspace(), 0);
    assert_eq!(tb.get_line(0), Some("aaaaaaaaaa".to_string()));
}

#[test]
fn test_move_cursor_and_newline() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    // Insert 10 'a's
    for _ in 0..10 {
        tb.insert('a');
    }
    // Backspace once -> 9 'a's, cursor at 9
    tb.backspace();
    assert_eq!(tb.get_line(0), Some("aaaaaaaaa".to_string()));

    // Move cursor back one position (col-1 = 8)
    tb.move_cursor(tb.cursor_row, tb.cursor_col - 1);

    // New line splits at cursor position 8
    assert_eq!(tb.new_line(), 0);

    // After newline: cursor should be on row 1
    assert_eq!(tb.cursor_row, 1);

    // Row 1 should have the last char "a"
    assert_eq!(tb.get_line(1), Some("a".to_string()));
    // Row 0 should have first 8 chars
    assert_eq!(tb.get_line(0), Some("aaaaaaaa".to_string()));
}

#[test]
fn test_newline_at_end() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('a');
    }
    // Newline at end of line
    assert_eq!(tb.new_line(), 0);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.get_line(0), Some("aaaaa".to_string()));
    assert_eq!(tb.get_line(1), Some(String::new()));
    assert_eq!(tb.last_line_loc, 1);
}

#[test]
fn test_newline_at_beginning() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('a');
    }
    tb.move_cursor(0, 0);
    assert_eq!(tb.new_line(), 0);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.get_line(0), Some(String::new()));
    assert_eq!(tb.get_line(1), Some("aaaaa".to_string()));
}

#[test]
fn test_get_line_out_of_bounds() {
    let tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.get_line(1), None);
    assert_eq!(tb.get_line(100), None);
}

#[test]
fn test_create_from_file() {
    use std::fs::File;
    let fp = File::open("c_src/tests/runtests.txt").unwrap();
    let tb = TextBuffer::create_from_file(&fp).unwrap();

    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.last_line_loc, 2);

    // Line 0: 11 a's
    assert_eq!(tb.get_line(0), Some("aaaaaaaaaaa".to_string()));
    // Line 1: 10 a's
    assert_eq!(tb.get_line(1), Some("aaaaaaaaaa".to_string()));
    // Line 2: 9 a's
    assert_eq!(tb.get_line(2), Some("aaaaaaaaa".to_string()));
}

#[test]
fn test_move_cursor_clamps_row() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.move_cursor(100, 0);
    assert_eq!(tb.cursor_row, 0); // clamped to last_line_loc
}

#[test]
fn test_move_cursor_clamps_col() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('a');
    }
    tb.move_cursor(0, 100);
    assert_eq!(tb.cursor_col, 5); // clamped to str_len
}

#[test]
fn test_multiple_newlines() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..3 {
        tb.insert('a');
    }
    // Split: "aaa" -> newline at end
    tb.new_line();
    for _ in 0..2 {
        tb.insert('b');
    }
    tb.new_line();
    for _ in 0..1 {
        tb.insert('c');
    }

    assert_eq!(tb.last_line_loc, 2);
    assert_eq!(tb.get_line(0), Some("aaa".to_string()));
    assert_eq!(tb.get_line(1), Some("bb".to_string()));
    assert_eq!(tb.get_line(2), Some("c".to_string()));
}

fn main() {}
