use std::ffi::c_int;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result: c_int = x | !y;
    let _ = write!(std::io::stdout(), "{}", result);
    let _ = writeln!(std::io::stdout());
}
