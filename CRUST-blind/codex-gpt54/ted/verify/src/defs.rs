pub const ESC: char = '\x1b';
pub const INVERT_COLOUR: &str = "\x1b[7m";
pub const INVERT_COLOUR_SIZE: usize = 4;
pub const RESET_STYLE_COLOUR: &str = "\x1b[0m";
pub fn panic(message: &str) {
    use std::io::{self, Write};

    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(b"\x1b[2J");
    let _ = stdout.write_all(b"\x1b[H");
    let _ = stdout.write_all(b"\x1b[?25h");
    let _ = stdout.flush();

    let err = io::Error::last_os_error();
    eprintln!("{message}: {err}");
    std::process::exit(1);
}
