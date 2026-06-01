use ted::buffer::TextBuffer;
use ted::gap::GapBuffer;
use ted::visual::{Cursor, VirtualScreen};

fn make_screen(width: usize, height: usize) -> VirtualScreen {
    VirtualScreen {
        buffer: vec!['\0'; 4096],
        buf_pos: 0,
        len: 4096,
        cursor: Cursor { x: 0, y: 0 },
        width,
        height,
        render_start_line: 0,
    }
}

#[test]
fn test_required_screen_rows_zero() {
    assert_eq!(VirtualScreen::required_screen_rows(0, 80), 1);
}

#[test]
fn test_required_screen_rows_one() {
    assert_eq!(VirtualScreen::required_screen_rows(1, 80), 1);
}

#[test]
fn test_required_screen_rows_exact_width() {
    assert_eq!(VirtualScreen::required_screen_rows(80, 80), 1);
}

#[test]
fn test_required_screen_rows_just_over() {
    assert_eq!(VirtualScreen::required_screen_rows(81, 80), 2);
}

#[test]
fn test_required_screen_rows_double_exact() {
    assert_eq!(VirtualScreen::required_screen_rows(160, 80), 2);
}

#[test]
fn test_required_screen_rows_one_over_double() {
    assert_eq!(VirtualScreen::required_screen_rows(161, 80), 3);
}

#[test]
fn test_required_screen_rows_misc() {
    assert_eq!(VirtualScreen::required_screen_rows(40, 80), 1);
    assert_eq!(VirtualScreen::required_screen_rows(200, 80), 3);
    assert_eq!(VirtualScreen::required_screen_rows(5, 1), 5);
}

#[test]
fn test_screen_append_basic() {
    let mut screen = make_screen(80, 24);
    screen.screen_append("hello", 5);
    assert_eq!(screen.buf_pos, 5);
    let s: String = screen.buffer[..5].iter().collect();
    assert_eq!(s, "hello");
}

#[test]
fn test_screen_append_no_room() {
    // Buffer exactly equal-or-less in remaining space than 'size' triggers a no-op (matches C: > size).
    let mut screen = VirtualScreen {
        buffer: vec!['\0'; 4],
        buf_pos: 0,
        len: 4,
        cursor: Cursor { x: 0, y: 0 },
        width: 80,
        height: 24,
        render_start_line: 0,
    };
    // size=4 but len-buf_pos=4; the check is strict >, so no append.
    screen.screen_append("abcd", 4);
    assert_eq!(screen.buf_pos, 0);
    assert_eq!(screen.buffer, vec!['\0', '\0', '\0', '\0']);
}

#[test]
fn test_set_virtual_cursor_position_first_line() {
    // A buffer with one line of length 5, cursor at (0, 3), screen width 80
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('h');
    tb.insert('e');
    tb.insert('l');
    tb.insert('l');
    tb.insert('o');
    // cursor_col is 5 after inserts. Move cursor to col 3.
    tb.move_cursor(0, 3);

    let mut screen = make_screen(80, 24);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut screen);
    // virtual_cursor_row should be 1 (first row, no wrap since line < width)
    assert_eq!(screen.cursor.x, 1);
    // y = (3 % 80) + 1 = 4
    assert_eq!(screen.cursor.y, 4);
}

#[test]
fn test_set_virtual_cursor_position_wraps() {
    // Build buffer with one long line that wraps
    let mut tb = TextBuffer::create(10, 200).unwrap();
    for _ in 0..150 {
        tb.insert('a');
    }
    // cursor_col is 150. Width=80 so col%width = 70, col/width = 1.
    let mut screen = make_screen(80, 24);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut screen);
    // virtual_cursor_row = 1 + 1 = 2
    assert_eq!(screen.cursor.x, 2);
    assert_eq!(screen.cursor.y, 71);
}

#[test]
fn test_set_virtual_cursor_position_second_line() {
    // Two lines, second line cursor.
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('h');
    tb.insert('i');
    tb.new_line();
    tb.insert('a');
    tb.insert('b');
    // cursor_row=1, cursor_col=2, width=80 so virtual_cursor_row = 1 + rsr(2,80) = 1+1 = 2
    let mut screen = make_screen(80, 24);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut screen);
    assert_eq!(screen.cursor.x, 2);
    assert_eq!(screen.cursor.y, 3);
}

#[test]
fn test_move_cursor_in_view_no_change() {
    // Buffer with 2 short lines, cursor on line 0, render_start_line 0.
    let mut tb = TextBuffer::create(10, 20).unwrap();
    tb.insert('a');
    tb.new_line();
    tb.insert('b');
    tb.move_cursor(0, 0);

    let mut screen = make_screen(80, 24);
    VirtualScreen::move_cursor_in_view(&tb, &mut screen);
    assert_eq!(screen.render_start_line, 0);
}

#[test]
fn test_move_cursor_in_view_above() {
    // Cursor on line 0 but render_start_line on line 2 -> should move up.
    let mut tb = TextBuffer::create(10, 20).unwrap();
    // We need at least 3 lines, so cheat: hack lines/last_line_loc.
    tb.insert('a');
    tb.new_line();
    tb.insert('b');
    tb.new_line();
    tb.insert('c');
    tb.move_cursor(0, 0);

    let mut screen = make_screen(80, 24);
    screen.render_start_line = 2;
    VirtualScreen::move_cursor_in_view(&tb, &mut screen);
    assert_eq!(screen.render_start_line, 0);
}

#[test]
fn test_destroy_consume() {
    // Just ensure destroy can be called.
    let gb = GapBuffer::create(10);
    gb.destroy();
}

fn main() {}
