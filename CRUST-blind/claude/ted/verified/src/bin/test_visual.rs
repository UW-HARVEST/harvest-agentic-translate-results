use ted::buffer::TextBuffer;
use ted::visual::{Cursor, VirtualScreen};

fn make_screen(width: usize, height: usize) -> VirtualScreen {
    VirtualScreen {
        buffer: vec!['\0'; 1000],
        buf_pos: 0,
        len: 1000,
        cursor: Cursor { x: 0, y: 0 },
        width,
        height,
        render_start_line: 0,
    }
}

#[test]
fn test_required_screen_rows_zero_line() {
    assert_eq!(VirtualScreen::required_screen_rows(0, 80), 1);
}

#[test]
fn test_required_screen_rows_small_line() {
    assert_eq!(VirtualScreen::required_screen_rows(1, 80), 1);
}

#[test]
fn test_required_screen_rows_full_line() {
    // 80/80 = 1, no remainder -> 1
    assert_eq!(VirtualScreen::required_screen_rows(80, 80), 1);
}

#[test]
fn test_required_screen_rows_overflow_line() {
    // 81/80 = 1, remainder 1 -> 2
    assert_eq!(VirtualScreen::required_screen_rows(81, 80), 2);
}

#[test]
fn test_required_screen_rows_double_overflow() {
    assert_eq!(VirtualScreen::required_screen_rows(160, 80), 2);
}

#[test]
fn test_required_screen_rows_more_cases() {
    assert_eq!(VirtualScreen::required_screen_rows(20, 5), 4);
    assert_eq!(VirtualScreen::required_screen_rows(21, 5), 5);
    assert_eq!(VirtualScreen::required_screen_rows(1, 5), 1);
    assert_eq!(VirtualScreen::required_screen_rows(50, 25), 2);
}

#[test]
fn test_required_screen_rows_zero_width_zero_line() {
    // Line length zero short-circuits before width
    assert_eq!(VirtualScreen::required_screen_rows(0, 0), 1);
}

#[test]
fn test_screen_append_basic() {
    let mut s = VirtualScreen {
        buffer: vec!['\0'; 100],
        buf_pos: 0,
        len: 100,
        cursor: Cursor { x: 0, y: 0 },
        width: 80,
        height: 24,
        render_start_line: 0,
    };
    s.screen_append("hello", 5);
    assert_eq!(s.buf_pos, 5);
    assert_eq!(s.buffer[0], 'h');
    assert_eq!(s.buffer[1], 'e');
    assert_eq!(s.buffer[2], 'l');
    assert_eq!(s.buffer[3], 'l');
    assert_eq!(s.buffer[4], 'o');

    s.screen_append("world", 5);
    assert_eq!(s.buf_pos, 10);
    assert_eq!(s.buffer[5], 'w');
    assert_eq!(s.buffer[9], 'd');
}

#[test]
fn test_screen_append_no_room() {
    let mut s = VirtualScreen {
        buffer: vec!['\0'; 5],
        buf_pos: 0,
        len: 5,
        cursor: Cursor { x: 0, y: 0 },
        width: 80,
        height: 24,
        render_start_line: 0,
    };
    // len(5) - buf_pos(0) = 5; check is `> size` so > 5 is false.
    s.screen_append("hello", 5);
    assert_eq!(s.buf_pos, 0);
}

#[test]
fn test_screen_append_just_fits() {
    let mut s = VirtualScreen {
        buffer: vec!['\0'; 5],
        buf_pos: 0,
        len: 5,
        cursor: Cursor { x: 0, y: 0 },
        width: 80,
        height: 24,
        render_start_line: 0,
    };
    // len(5) - buf_pos(0) = 5; > 4 is true
    s.screen_append("abcd", 4);
    assert_eq!(s.buf_pos, 4);
    assert_eq!(s.buffer[0], 'a');
    assert_eq!(s.buffer[3], 'd');
}

#[test]
fn test_set_virtual_cursor_position_first_line() {
    let mut tb = TextBuffer::create(10, 100).unwrap();
    for c in "hello".chars() { tb.insert(c); }
    // cursor at (0, 5)
    let mut s = make_screen(20, 10);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut s);
    // First line, render_start_line = 0; cursor_row=0, cursor_col=5
    // virtual_cursor_row = 1; col_wrap = 5/20 = 0 -> x=1
    // y = 5%20 + 1 = 6
    assert_eq!(s.cursor.x, 1);
    assert_eq!(s.cursor.y, 6);
}

#[test]
fn test_set_virtual_cursor_position_with_wrap() {
    let mut tb = TextBuffer::create(10, 100).unwrap();
    // single line of 25 chars, cursor at the end: width=10 -> wraps
    for _ in 0..25 { tb.insert('a'); }
    // cursor at (0, 25)
    let mut s = make_screen(10, 10);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut s);
    // virtual_cursor_row = 1 (no other lines before)
    // virtual_cursor_row += 25/10 = 2 -> 3
    // y = 25 % 10 + 1 = 6
    assert_eq!(s.cursor.x, 3);
    assert_eq!(s.cursor.y, 6);
}

#[test]
fn test_set_virtual_cursor_position_multiline() {
    let mut tb = TextBuffer::create(10, 100).unwrap();
    // 5 lines: "hello", "world how are you" (17), "x" (1), "this is line four" (17), "last" (4)
    let lines = ["hello", "world how are you", "x", "this is line four", "last"];
    for (i, line) in lines.iter().enumerate() {
        for c in line.chars() { tb.insert(c); }
        if i < 4 { tb.new_line(); }
    }
    // cursor at row=4, col=4 after all that. We move it to (4, 4)
    tb.move_cursor(4, 4);
    let mut s = make_screen(20, 10);
    s.render_start_line = 0;
    VirtualScreen::set_virtual_cursor_position(&tb, &mut s);
    // From C verify: cursor.x=5, cursor.y=5
    assert_eq!(s.cursor.x, 5);
    assert_eq!(s.cursor.y, 5);
}

#[test]
fn test_set_virtual_cursor_position_with_wrapping_lines() {
    let mut tb = TextBuffer::create(10, 100).unwrap();
    let lines = ["hello", "world how are you", "x", "this is line four", "last"];
    for (i, line) in lines.iter().enumerate() {
        for c in line.chars() { tb.insert(c); }
        if i < 4 { tb.new_line(); }
    }
    // width=10, cursor at row=3, col=12 (line "this is line four" = 17 chars -> wraps at width 10)
    tb.move_cursor(3, 12);
    let mut s = make_screen(10, 10);
    s.render_start_line = 0;
    VirtualScreen::set_virtual_cursor_position(&tb, &mut s);
    // From C verify: cursor.x=6, cursor.y=3
    assert_eq!(s.cursor.x, 6);
    assert_eq!(s.cursor.y, 3);
}

#[test]
fn test_move_cursor_in_view_cursor_above_render_start() {
    // Cursor row < render_start_line -> render_start_line should snap to cursor row
    let mut tb = TextBuffer::create(10, 100).unwrap();
    // Build 5 lines
    let lines = ["a", "b", "c", "d", "e"];
    for (i, line) in lines.iter().enumerate() {
        for c in line.chars() { tb.insert(c); }
        if i < 4 { tb.new_line(); }
    }
    // Cursor at line 0 (we'll move it there)
    tb.move_cursor(0, 0);
    let mut s = make_screen(20, 10);
    s.render_start_line = 2;
    VirtualScreen::move_cursor_in_view(&tb, &mut s);
    assert_eq!(s.render_start_line, 0);
}

#[test]
fn test_move_cursor_in_view_within_view() {
    // Cursor in already-visible range -> render_start_line unchanged
    let mut tb = TextBuffer::create(10, 100).unwrap();
    let lines = ["a", "b", "c", "d", "e"];
    for (i, line) in lines.iter().enumerate() {
        for c in line.chars() { tb.insert(c); }
        if i < 4 { tb.new_line(); }
    }
    tb.move_cursor(2, 0);  // visible
    let mut s = make_screen(20, 10);
    s.render_start_line = 0;
    VirtualScreen::move_cursor_in_view(&tb, &mut s);
    // Cursor is well within the screen; render_start_line stays at 0.
    assert_eq!(s.render_start_line, 0);
}

#[test]
fn test_draw_editor_window_basic() {
    // Simply ensure it doesn't panic and produces output
    let mut tb = TextBuffer::create(10, 100).unwrap();
    for c in "hello".chars() { tb.insert(c); }
    tb.new_line();
    for c in "world".chars() { tb.insert(c); }

    let mut s = make_screen(20, 10);
    VirtualScreen::draw_editor_window(&tb, &mut s);
    // Buffer should have "hello\r\nworld\r\n" then blank rows
    // Find what was written by checking buf_pos
    assert!(s.buf_pos > 0);
}

fn main() {}
