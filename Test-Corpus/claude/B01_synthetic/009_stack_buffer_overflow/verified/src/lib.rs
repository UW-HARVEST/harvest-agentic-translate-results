//! `extern "C"` export layer for the translation of `c_src/src/main.c`.
//!
//! The C translation unit gives external linkage to `printLine`,
//! `printIntLine`, `bad`, `good` and `main` (`goodG2B` and `goodB2G` are
//! `static`, so they are deliberately *not* exported here either). Building
//! `main.c` with `gcc -shared -fPIC` therefore yields exactly those five
//! dynamic symbols, and this cdylib exports the same five under the same names
//! so an external caller cannot tell the two shared objects apart.
//!
//! Note what these wrappers do *not* do: they do not flush. The C functions are
//! plain `printf` calls into the process's `stdout` FILE, leaving the output
//! buffered for the caller to flush (or to lose, if `bad()`'s out-of-bounds
//! write kills the process first). Because `imp` writes through that very same
//! FILE object, doing nothing here is what makes the two libraries
//! indistinguishable.

mod imp;

use imp::{Caller, Io};
use std::os::raw::{c_char, c_int};

/// `void printLine(const char * line)`
///
/// Reproduces the `line != NULL` guard: a NULL argument prints nothing at all.
#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        return;
    }
    Io::new().print_cstr(line);
}

/// `void printIntLine(int intNumber)`
#[no_mangle]
pub extern "C" fn printIntLine(int_number: c_int) {
    Io::new().print_int_line(int_number);
}

/// `void bad()`
///
/// `Caller::Unknown`: the frame above this one belongs to whoever loaded the
/// shared object, so the out-of-bounds store's effect beyond index 19 is a
/// property of that caller and cannot be modelled from in here. See the comment
/// block in `imp.rs`.
#[no_mangle]
pub extern "C" fn bad() {
    let mut io = Io::new();
    // With `Caller::Unknown` the only fault `imp::bad` can arm is at its own `ret`
    // (indices 18..19), which it raises itself, so nothing is left to do here.
    // Indices 16..17 clobber only the *saved rbp*, and whether that is fatal
    // depends on whether the consumer reloads `rbp` -- a `gcc -O0` consumer dies,
    // an optimized one does not. Both were observed with the same C `.so`, so the
    // export declines to guess rather than fabricating a fault. See `imp.rs`.
    let _ = imp::bad(&mut io, Caller::Unknown);
}

/// `void good()`
#[no_mangle]
pub extern "C" fn good() {
    let mut io = Io::new();
    imp::good(&mut io);
}

/// `int main(int argc, char * argv[])`
///
/// `main` has external linkage in the C translation unit and so appears in the
/// shared object's dynamic symbol table; it is exported here for parity. `argc`
/// and `argv` are unused by the C body.
///
/// `Caller::CMain` is correct here even though this is the library: within this
/// call `bad()`'s caller really is this `main`, laid out as gcc lays it out, so
/// indices 26..27 do hit its return address.
#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let mut io = Io::new();
    imp::run_main(&mut io, Caller::CMain)
}
