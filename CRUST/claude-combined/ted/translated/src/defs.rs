pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";

/// Print a panic-style message to stderr, clear the screen and exit. Mirrors C's panic().
pub fn panic(message: &str) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(b"\x1b[2J");
    let _ = handle.write_all(b"\x1b[H");
    let _ = handle.write_all(b"\x1b[?25h");
    let _ = handle.flush();
    eprintln!("{}", message);
    std::process::exit(1);
}
