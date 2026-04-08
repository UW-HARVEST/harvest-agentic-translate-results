use ted::buffer::TextBuffer;
use ted::visual::VirtualScreen;
use ted::visual::Cursor;

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
fn test_required_screen_rows_zero() {
    assert_eq!(VirtualScreen::required_screen_rows(0, 80), 1);
}

#[test]
fn test_required_screen_rows_one() {
    assert_eq!(VirtualScreen::required_screen_rows(1, 80), 1);
}

#[test]
fn test_required_screen_rows_exact() {
    assert_eq!(VirtualScreen::required_screen_rows(80, 80), 1);
}

#[test]
fn test_required_screen_rows_overflow() {
    assert_eq!(VirtualScreen::required_screen_rows(81, 80), 2);
}

#[test]
fn test_required_screen_rows_double() {
    assert_eq!(VirtualScreen::required_screen_rows(160, 80), 2);
}

#[test]
fn test_required_screen_rows_double_plus_one() {
    assert_eq!(VirtualScreen::required_screen_rows(161, 80), 3);
}

#[test]
fn test_required_screen_rows_under() {
    assert_eq!(VirtualScreen::required_screen_rows(79, 80), 1);
}

#[test]
fn test_required_screen_rows_width_one() {
    assert_eq!(VirtualScreen::required_screen_rows(1, 1), 1);
}

#[test]
fn test_required_screen_rows_remainder() {
    assert_eq!(VirtualScreen::required_screen_rows(5, 3), 2);
    assert_eq!(VirtualScreen::required_screen_rows(6, 3), 2);
}

#[test]
fn test_required_screen_rows_ten() {
    assert_eq!(VirtualScreen::required_screen_rows(100, 10), 10);
    assert_eq!(VirtualScreen::required_screen_rows(101, 10), 11);
    assert_eq!(VirtualScreen::required_screen_rows(10, 10), 1);
    assert_eq!(VirtualScreen::required_screen_rows(11, 10), 2);
}

#[test]
fn test_set_virtual_cursor_position() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('a'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('b'); }

    let mut vs = make_screen(80, 24);
    // cursor at row=1, col=3
    assert_eq!(tb.cursor_row, 1);
    assert_eq!(tb.cursor_col, 3);

    VirtualScreen::set_virtual_cursor_position(&tb, &mut vs);
    assert_eq!(vs.cursor.x, 2);
    assert_eq!(vs.cursor.y, 4);
}

#[test]
fn test_set_virtual_cursor_position_first_line() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('a'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('b'); }
    tb.move_cursor(0, 2);

    let mut vs = make_screen(80, 24);
    VirtualScreen::set_virtual_cursor_position(&tb, &mut vs);
    assert_eq!(vs.cursor.x, 1);
    assert_eq!(vs.cursor.y, 3);
}

#[test]
fn test_move_cursor_in_view_already_visible() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('a'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('b'); }
    tb.move_cursor(0, 2);

    let mut vs = make_screen(80, 24);
    VirtualScreen::move_cursor_in_view(&tb, &mut vs);
    assert_eq!(vs.render_start_line, 0);
}

#[test]
fn test_move_cursor_in_view_scroll_down() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('a'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('b'); }
    tb.move_cursor(1, 0);

    let mut vs = make_screen(80, 2);
    VirtualScreen::move_cursor_in_view(&tb, &mut vs);
    assert_eq!(vs.render_start_line, 2);
}

#[test]
fn test_move_cursor_in_view_scroll_up() {
    let mut tb = TextBuffer::create(10, 20).unwrap();
    for _ in 0..5 { tb.insert('a'); }
    tb.new_line();
    for _ in 0..3 { tb.insert('b'); }
    tb.move_cursor(0, 0);

    let mut vs = make_screen(80, 24);
    vs.render_start_line = 1;
    VirtualScreen::move_cursor_in_view(&tb, &mut vs);
    assert_eq!(vs.render_start_line, 0);
}

fn main() {}
