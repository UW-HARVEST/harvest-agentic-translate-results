pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";
pub fn panic(message: &str) {
    // Clear screen and move cursor to home (mirrors C panic())
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[2J");
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[H");
    // Re-enable cursor
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[?25h");
    eprintln!("{}", message);
    std::process::exit(1);
}
