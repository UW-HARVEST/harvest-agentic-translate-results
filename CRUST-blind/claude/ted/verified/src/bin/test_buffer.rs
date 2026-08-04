use ted::buffer::{TextBuffer, DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP};

#[test]
fn test_create_basic() {
    let tb = TextBuffer::create(10, 20).expect("should create");
    assert_eq!(tb.lines_capacity, 10);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.last_line_loc, 0);
    // First line should be allocated
    assert!(tb.lines[0].is_some());
    assert_eq!(tb.lines[0].as_ref().unwrap().str_len, 0);
    assert_eq!(tb.lines[0].as_ref().unwrap().gap_len, 20);
    // Other lines should be None
    for i in 1..10 {
        assert!(tb.lines[i].is_none(), "line {} should be None", i);
    }
}

#[test]
fn test_get_line_empty_first() {
    let tb = TextBuffer::create(10, 20).unwrap();
    let line = tb.get_line(0);
    assert!(line.is_some());
    assert_eq!(line.unwrap(), "");
}

#[test]
fn test_get_line_out_of_bounds() {
    let tb = TextBuffer::create(10, 20).unwrap();
    // last_line_loc=0, so any row > 0 is out of bounds
    assert!(tb.get_line(1).is_none());
    assert!(tb.get_line(5).is_none());
}

#[test]
fn test_insert_one_char() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    let err = tb.insert('a');
    assert_eq!(err, 0);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 1);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.last_line_loc, 0);
    assert_eq!(tb.get_line(0).unwrap(), "a");
}

#[test]
fn test_insert_multiple_chars() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..11 {
        let err = tb.insert('a');
        assert_eq!(err, 0);
    }
    assert_eq!(tb.cursor_col, 11);
    assert_eq!(tb.get_line(0).unwrap(), "aaaaaaaaaaa");
}

#[test]
fn test_backspace_after_insert() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..11 {
        tb.insert('a');
    }
    let err = tb.backspace();
    assert_eq!(err, 0);
    assert_eq!(tb.cursor_col, 10);
    assert_eq!(tb.get_line(0).unwrap(), "aaaaaaaaaa");
}

#[test]
fn test_move_cursor_clamp_row_above() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('x');
    }
    // row=100 but last_line_loc=0 => row should be clamped to 0.
    // col=100 but str_len=5 => col clamped to 5
    tb.move_cursor(100, 100);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 5);
    // cursor_col was 5 already, then we set to 5 -> no change so cursor_col_moved stays false
    assert_eq!(tb.cursor_col_moved, false);
}

#[test]
fn test_move_cursor_changes_col() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 {
        tb.insert('x');
    }
    // move col from 5 to 3
    tb.move_cursor(0, 3);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 3);
    assert_eq!(tb.cursor_col_moved, true);
}

#[test]
fn test_newline_basic() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..11 {
        tb.insert('a');
    }
    tb.backspace();
    // now line is "aaaaaaaaaa" cursor at 10
    tb.move_cursor(0, 9);  // move cursor back by 1
    let err = tb.new_line();
    assert_eq!(err, 0);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.last_line_loc, 1);
    assert_eq!(tb.get_line(0).unwrap(), "aaaaaaaaa");
    assert_eq!(tb.get_line(1).unwrap(), "a");
}

#[test]
fn test_newline_grows_capacity() {
    let mut tb = TextBuffer::create(2, 20).unwrap();
    assert_eq!(tb.lines_capacity, 2);
    tb.insert('a');
    let err = tb.new_line();
    assert_eq!(err, 0);
    // After 1 newline, last_line_loc=1, capacity=2 (last_line == capacity-1)
    assert_eq!(tb.last_line_loc, 1);
    assert_eq!(tb.lines_capacity, 2);

    tb.insert('b');
    let err = tb.new_line();
    assert_eq!(err, 0);
    // Now last_line_loc was 1, capacity-1 == 1, so the buffer was grown to 4
    assert_eq!(tb.last_line_loc, 2);
    assert_eq!(tb.lines_capacity, 4);
    assert_eq!(tb.get_line(0).unwrap(), "a");
    assert_eq!(tb.get_line(1).unwrap(), "b");
    assert_eq!(tb.get_line(2).unwrap(), "");
    assert_eq!(tb.cursor_row, 2);
    assert_eq!(tb.cursor_col, 0);
}

#[test]
fn test_get_line_after_newline() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for c in "hello".chars() { tb.insert(c); }
    tb.new_line();
    for c in "world".chars() { tb.insert(c); }
    assert_eq!(tb.last_line_loc, 1);
    assert_eq!(tb.get_line(0).unwrap(), "hello");
    assert_eq!(tb.get_line(1).unwrap(), "world");
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 5);
}

#[test]
fn test_create_from_file() {
    use std::fs::File;
    use std::io::Write;
    use std::env::temp_dir;

    let path = temp_dir().join("ted_test_input1.txt");
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(b"aaaaaaaaaaa\naaaaaaaaaa\naaaaaaaaa\n").unwrap();
    }
    let f = File::open(&path).unwrap();
    let tb = TextBuffer::create_from_file(&f).unwrap();
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.last_line_loc, 2);
    assert_eq!(tb.get_line(0).unwrap(), "aaaaaaaaaaa");
    assert_eq!(tb.get_line(1).unwrap(), "aaaaaaaaaa");
    assert_eq!(tb.get_line(2).unwrap(), "aaaaaaaaa");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_create_from_empty_file() {
    use std::fs::File;
    use std::env::temp_dir;

    let path = temp_dir().join("ted_test_empty.txt");
    {
        let _ = File::create(&path).unwrap();
    }
    let f = File::open(&path).unwrap();
    let tb = TextBuffer::create_from_file(&f).unwrap();
    assert_eq!(tb.last_line_loc, 0);
    assert_eq!(tb.get_line(0).unwrap(), "");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_default_constants() {
    assert_eq!(DEFAULT_CAPACITY, 100);
    assert_eq!(DEFAULT_GAP_BUF_CAP, 100);
}

#[test]
fn test_official_runtests_textbuffer_sequence() {
    // Mirror the C runtests TestTextBuffer
    let mut tb = TextBuffer::create(10, 20).expect("create");

    // Test 1, empty string
    assert_eq!(tb.get_line(tb.cursor_row).unwrap(), "");

    // Test 2 insert char
    let err = tb.insert('a');
    assert_eq!(err, 0);
    assert_eq!(tb.get_line(tb.cursor_row).unwrap(), "a");

    // Test 2.1 insert 10 more 'a' (total 11)
    for _ in 0..10 {
        let err = tb.insert('a');
        assert_eq!(err, 0);
    }
    assert_eq!(tb.get_line(tb.cursor_row).unwrap(), "aaaaaaaaaaa");

    // Test 3 backspace -> 10 'a'
    let err = tb.backspace();
    assert_eq!(err, 0);
    assert_eq!(tb.get_line(tb.cursor_row).unwrap(), "aaaaaaaaaa");

    // Test 4 MoveCursor, Newline
    tb.move_cursor(tb.cursor_row, tb.cursor_col - 1);
    let err = tb.new_line();
    assert_eq!(err, 0);
    // The new (current) line is "a"; the prior line (cursor_row-1) is "aaaaaaaaa"
    assert_eq!(tb.get_line(tb.cursor_row).unwrap(), "a");
    assert_eq!(tb.get_line(tb.cursor_row - 1).unwrap(), "aaaaaaaaa");
}

#[test]
fn test_create_zero_lines() {
    let tb = TextBuffer::create(0, 20);
    assert!(tb.is_none());
}

#[test]
fn test_move_cursor_within_bounds() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('x'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('y'); }
    // last_line_loc=1, cursor at (1,3)
    tb.move_cursor(0, 2);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 2);
    assert_eq!(tb.cursor_col_moved, true);
    // Insert at this location
    let err = tb.insert('Z');
    assert_eq!(err, 0);
    assert_eq!(tb.cursor_col, 3);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.get_line(0).unwrap(), "xxZxxx");
    assert_eq!(tb.get_line(1).unwrap(), "yyy");
}

fn main() {}
