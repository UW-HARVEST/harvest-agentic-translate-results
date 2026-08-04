use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2 * x;
    y += 300;
    // Match printf("%d\n", y) using stdout (line-buffered like C's stdout to a TTY,
    // but for byte-identical output we just write the same bytes and flush).
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", y);
    let _ = handle.flush();
}
