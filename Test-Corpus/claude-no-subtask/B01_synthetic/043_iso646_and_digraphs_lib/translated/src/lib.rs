use std::ffi::c_int;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result: c_int = x | !y;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    // printf("%d", result);
    let _ = write!(handle, "{}", result);
    // puts("") writes an empty string followed by a newline.
    let _ = writeln!(handle);
    let _ = handle.flush();
}
