use ted::defs;

#[test]
fn test_esc() {
    assert_eq!(defs::ESC, '\x1b');
}

#[test]
fn test_invert_colour() {
    assert_eq!(defs::INVERT_COLOUR, "\x1b[7m");
}

#[test]
fn test_invert_colour_size() {
    assert_eq!(defs::INVERT_COLOUR_SIZE, 4);
}

#[test]
fn test_reset_style_colour() {
    assert_eq!(defs::RESET_STYLE_COLOUR, "\x1b[0m");
}

fn main() {}
