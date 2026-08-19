// C-ABI surface of the translation, mirroring the symbols that
// c_src/src/main.c exports when it is compiled as a shared library
// (`driver` and `main`; `print_hex` is `static` in the C source and therefore
// not exported).
//
// The implementation lives in src/scan.rs, which is shared verbatim with the
// `driver` executable target (src/main.rs).

#[allow(dead_code)]
mod scan;

use std::io::{self, Write};
#[cfg(not(test))]
use std::os::raw::c_int;

/// `void driver(float x);`
///
/// Prints the raw bytes of `x` in memory order as lowercase two-digit hex,
/// followed by a newline, on stdout.
#[no_mangle]
pub extern "C" fn driver(x: f32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    scan::driver(x, &mut out);
    let _ = out.flush();
}

/// `int main(void);`
///
/// Reads one float from stdin with C `scanf("%f", &x)` semantics (leaving the
/// value at +0.0f when the conversion fails) and passes it to `driver`.
///
/// `cfg(not(test))` because `cargo test` compiles this same file with `--test`,
/// which generates its own `main` symbol.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let rc = scan::run(&mut input, &mut out);
    let _ = out.flush();
    rc as c_int
}
