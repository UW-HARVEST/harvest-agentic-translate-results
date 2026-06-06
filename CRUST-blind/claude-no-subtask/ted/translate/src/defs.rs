pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";
pub fn panic(message: &str) {
    use std::io::Write;
    // Clear screen
    let _ = std::io::stdout().write_all(b"\x1b[2J");
    // Move cursor to top
    let _ = std::io::stdout().write_all(b"\x1b[H");
    // Enable cursor
    let _ = std::io::stdout().write_all(b"\x1b[?25h");
    eprintln!("{}", message);
    std::process::exit(1);
}
