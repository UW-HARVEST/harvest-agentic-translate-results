pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";

/// CTRL_KEY combines a key with the CTRL modifier (ASCII).
pub fn ctrl_key(k: u8) -> u8 {
    k & 0x1f
}

pub fn panic(message: &str) {
    // Match the C panic which clears the screen and exits.
    // In Rust we just print a message to stderr and exit.
    // Avoid clearing the actual terminal in tests.
    eprintln!("panic: {}", message);
    std::process::exit(1);
}
