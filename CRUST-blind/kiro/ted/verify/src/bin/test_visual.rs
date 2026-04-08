use ted::visual::VirtualScreen;

#[test]
fn test_required_screen_rows_zero_length() {
    assert_eq!(VirtualScreen::required_screen_rows(0, 80), 1);
}

#[test]
fn test_required_screen_rows_exact_fit() {
    assert_eq!(VirtualScreen::required_screen_rows(80, 80), 1);
}

#[test]
fn test_required_screen_rows_one_char() {
    assert_eq!(VirtualScreen::required_screen_rows(1, 80), 1);
}

#[test]
fn test_required_screen_rows_wraps() {
    assert_eq!(VirtualScreen::required_screen_rows(81, 80), 2);
    assert_eq!(VirtualScreen::required_screen_rows(160, 80), 2);
    assert_eq!(VirtualScreen::required_screen_rows(161, 80), 3);
}

#[test]
fn test_required_screen_rows_small_width() {
    assert_eq!(VirtualScreen::required_screen_rows(10, 3), 4); // 3+3+3+1
    assert_eq!(VirtualScreen::required_screen_rows(9, 3), 3);
    assert_eq!(VirtualScreen::required_screen_rows(1, 1), 1);
    assert_eq!(VirtualScreen::required_screen_rows(5, 1), 5);
}

fn main() {}
