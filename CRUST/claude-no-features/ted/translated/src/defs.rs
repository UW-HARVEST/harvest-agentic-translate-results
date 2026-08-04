pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";
pub fn panic(message: &str) {
    // Clear screen and move cursor home, similar to the C version, then exit.
    print!("\x1b[2J\x1b[H\x1b[?25h");
    eprintln!("{}", message);
    std::process::exit(1);
}
