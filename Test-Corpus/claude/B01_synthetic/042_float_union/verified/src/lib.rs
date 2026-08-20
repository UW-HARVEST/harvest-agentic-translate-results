// C ABI surface of the translation of `c_src/src/main.c`.
//
// `c_src/src/main.c` compiled as a shared object exports exactly two symbols,
// `driver` and `main` (see `SYMBOLS.md`).  This crate re-exports the same two
// symbols with the same signatures so that a caller loading either shared
// object with `dlopen`/`dlsym` cannot tell them apart.
//
// The implementation itself lives in `imp.rs`, which is shared verbatim with the
// `driver` executable (`main.rs` includes the very same file).

mod imp;

use std::os::raw::c_int;

/// `void driver(double f)`
#[no_mangle]
pub extern "C" fn driver(f: f64) {
    imp::driver(f);
}

/// `int main()` — reads one `double` from standard input with the semantics of
/// `scanf("%lf", &f)` and calls `driver`.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::run();
    0
}
