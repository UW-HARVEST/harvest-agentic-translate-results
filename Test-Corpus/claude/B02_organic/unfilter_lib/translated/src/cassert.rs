//! Faithful emulation of C's `assert()` (asserts are *enabled* in the C build:
//! the CMake project sets no build type, so `NDEBUG` is never defined).
//!
//! glibc's `__assert_fail` writes
//!   `<progname>: <file>:<line>: <func>: Assertion `<expr>' failed.\n`
//! to stderr and then raises `SIGABRT` via `abort()`.

use std::io::Write;

/// The `__FILE__` string as seen by the original translation unit.
const C_FILE: &str = "c_src/src/lib.c";

#[cold]
#[inline(never)]
pub fn assert_fail(expr: &str, func: &str, line: u32) -> ! {
    let prog = std::env::args_os()
        .next()
        .and_then(|a| {
            std::path::Path::new(&a)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let msg = format!(
        "{}: {}:{}: {}: Assertion `{}' failed.\n",
        prog, C_FILE, line, func, expr
    );
    let _ = std::io::stderr().write_all(msg.as_bytes());
    let _ = std::io::stderr().flush();
    std::process::abort()
}

/// `assert(cond)` where `expr_text` is the stringified expression, `func` is
/// the enclosing function name and `line` the source line in `lib.c`.
macro_rules! c_assert {
    ($cond:expr, $expr_text:expr, $func:expr, $line:expr) => {
        if !($cond) {
            crate::cassert::assert_fail($expr_text, $func, $line);
        }
    };
}
