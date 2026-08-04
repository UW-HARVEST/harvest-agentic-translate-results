use ted::defs::{ESC, INVERT_COLOUR, INVERT_COLOUR_SIZE, RESET_STYLE_COLOUR};

#[test]
fn test_esc_constant() {
    assert_eq!(ESC, '\x1b');
    assert_eq!(ESC as u32, 27);
}

#[test]
fn test_invert_colour() {
    assert_eq!(INVERT_COLOUR, "\x1b[7m");
    // It's 4 bytes (ESC, [, 7, m)
    assert_eq!(INVERT_COLOUR.len(), 4);
}

#[test]
fn test_invert_colour_size() {
    assert_eq!(INVERT_COLOUR_SIZE, 4);
    assert_eq!(INVERT_COLOUR.len(), INVERT_COLOUR_SIZE);
}

#[test]
fn test_reset_style_colour() {
    assert_eq!(RESET_STYLE_COLOUR, "\x1b[0m");
    assert_eq!(RESET_STYLE_COLOUR.len(), 4);
}

fn main() {}
