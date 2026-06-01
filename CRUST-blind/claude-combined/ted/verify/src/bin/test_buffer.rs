use ted::buffer::{TextBuffer, DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP};

#[test]
fn test_constants() {
    assert_eq!(DEFAULT_CAPACITY, 100);
    assert_eq!(DEFAULT_GAP_BUF_CAP, 100);
}

#[test]
fn test_create() {
    let tb = TextBuffer::create(10, 20).expect("expected text buffer");
    assert_eq!(tb.lines_capacity, 10);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.cursor_col_moved, false);
    assert_eq!(tb.last_line_loc, 0);
    // First line exists with str_len=0, gap_len=20, gap_loc=0.
    let line = tb.lines[0].as_ref().expect("line 0 must be present");
    assert_eq!(line.str_len, 0);
    assert_eq!(line.gap_len, 20);
    assert_eq!(line.gap_loc, 0);
    // Other lines are NULL/None.
    for i in 1..10 {
        assert!(tb.lines[i].is_none(), "line {i} must be None");
    }
}

#[test]
fn test_insert_chars() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.insert('h'), 0);
    assert_eq!(tb.insert('i'), 0);
    assert_eq!(tb.cursor_col, 2);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.get_line(0), Some("hi".to_string()));
}

#[test]
fn test_new_line_then_insert() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('h');
    tb.insert('i');
    assert_eq!(tb.new_line(), 0);
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 0);
    assert_eq!(tb.last_line_loc, 1);
    tb.insert('a');
    tb.insert('b');
    assert_eq!(tb.get_line(0), Some("hi".to_string()));
    assert_eq!(tb.get_line(1), Some("ab".to_string()));
}

#[test]
fn test_move_cursor_and_insert() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('h');
    tb.insert('i');
    tb.move_cursor(0, 1);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 1);
    assert_eq!(tb.cursor_col_moved, true);
    tb.insert('X');
    assert_eq!(tb.get_line(0), Some("hXi".to_string()));
    // After insert, cursor_col_moved must be reset.
    assert_eq!(tb.cursor_col_moved, false);
}

#[test]
fn test_backspace() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('h');
    tb.insert('X');
    tb.insert('i');
    // current line: "hXi", cursor_col=3
    assert_eq!(tb.backspace(), 0);
    // backspace should delete 'i'
    assert_eq!(tb.get_line(0), Some("hX".to_string()));
    assert_eq!(tb.cursor_col, 2);
}

#[test]
fn test_get_line_oob_returns_none() {
    let tb = TextBuffer::create(10, 20).unwrap();
    assert_eq!(tb.get_line(5), None);
    assert_eq!(tb.get_line(100), None);
}

#[test]
fn test_move_cursor_clamps_row_and_col() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('a');
    tb.insert('b');
    // out-of-bounds row clamps to last_line_loc (0)
    tb.move_cursor(99, 1);
    assert_eq!(tb.cursor_row, 0);
    assert_eq!(tb.cursor_col, 1);

    // col clamps to str_len
    tb.move_cursor(0, 99);
    assert_eq!(tb.cursor_col, 2);
}

#[test]
fn test_newline_shifting() {
    // Build up a 3-line buffer A, B, C
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('A');
    tb.new_line();
    tb.insert('B');
    tb.new_line();
    tb.insert('C');
    assert_eq!(tb.last_line_loc, 2);
    assert_eq!(tb.get_line(0), Some("A".to_string()));
    assert_eq!(tb.get_line(1), Some("B".to_string()));
    assert_eq!(tb.get_line(2), Some("C".to_string()));

    // move cursor to end of line 0 ("A") and insert newline
    tb.move_cursor(0, 1);
    assert_eq!(tb.new_line(), 0);
    assert_eq!(tb.last_line_loc, 3);
    assert_eq!(tb.cursor_row, 1);
    // C output: "A" "" "B" "C"
    assert_eq!(tb.get_line(0), Some("A".to_string()));
    assert_eq!(tb.get_line(1), Some("".to_string()));
    assert_eq!(tb.get_line(2), Some("B".to_string()));
    assert_eq!(tb.get_line(3), Some("C".to_string()));
}

#[test]
fn test_create_from_file() {
    use std::io::Write;
    let path = std::env::temp_dir().join("ted_test_input.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "line one").unwrap();
        writeln!(f, "line two").unwrap();
        writeln!(f, "line three").unwrap();
    }
    let f = std::fs::File::open(&path).unwrap();
    let tb = TextBuffer::create_from_file(&f).expect("create_from_file failed");
    assert_eq!(tb.last_line_loc, 2);
    assert_eq!(tb.lines_capacity, 100);
    assert_eq!(tb.get_line(0), Some("line one".to_string()));
    assert_eq!(tb.get_line(1), Some("line two".to_string()));
    assert_eq!(tb.get_line(2), Some("line three".to_string()));

    // line lengths should match read length
    let l0 = tb.lines[0].as_ref().unwrap();
    assert_eq!(l0.str_len, 8);
    assert_eq!(l0.gap_loc, 8);
    assert_eq!(l0.gap_len, 100);

    let l2 = tb.lines[2].as_ref().unwrap();
    assert_eq!(l2.str_len, 10);
    assert_eq!(l2.gap_loc, 10);
    assert_eq!(l2.gap_len, 100);
}

fn main() {}
