use ted::defs::{ctrl_key, ESC, INVERT_COLOUR, INVERT_COLOUR_SIZE, RESET_STYLE_COLOUR};

#[test]
fn test_constants() {
    assert_eq!(ESC, '\x1b');
    assert_eq!(INVERT_COLOUR, "\x1b[7m");
    assert_eq!(INVERT_COLOUR_SIZE, 4);
    assert_eq!(RESET_STYLE_COLOUR, "\x1b[0m");
}

#[test]
fn test_ctrl_key() {
    // CTRL+Q in C: 'q' & 0x1f = 0x71 & 0x1f = 0x11 = 17
    assert_eq!(ctrl_key(b'q'), 17);
    // CTRL+S = 0x73 & 0x1f = 0x13 = 19
    assert_eq!(ctrl_key(b's'), 19);
    // CTRL+L = 0x6c & 0x1f = 0x0c = 12
    assert_eq!(ctrl_key(b'l'), 12);
    // CTRL+H = 0x68 & 0x1f = 0x08 = 8
    assert_eq!(ctrl_key(b'h'), 8);
    // 0x00 -> 0x00
    assert_eq!(ctrl_key(0), 0);
}

fn main() {}
