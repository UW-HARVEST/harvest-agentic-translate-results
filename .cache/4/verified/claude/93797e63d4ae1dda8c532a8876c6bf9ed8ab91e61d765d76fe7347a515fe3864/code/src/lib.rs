// C-ABI surface of the translation.
//
// `c_src/src/main.c` is a translation unit that, when compiled as a shared
// object, exports exactly two dynamic symbols: `run` and `main`.  This cdylib
// exports the same two symbols with identical signatures and behaviour so that
// the two shared objects can be compared through the FFI boundary.

use std::io::Write;
use std::os::raw::c_int;

#[path = "imp.rs"]
mod imp;

pub use imp::House;

/// `void run(house_t *the_house, int extra_bedrooms)`
///
/// # Safety
/// `the_house` must point to a valid `house_t`, exactly as required by the C
/// implementation (which dereferences it unconditionally).
#[no_mangle]
pub unsafe extern "C" fn run(the_house: *mut House, extra_bedrooms: c_int) {
    // The C code dereferences `the_house` with no validation whatsoever, so
    // this must too: `ptr::read`/`ptr::write` reproduce a plain C load/store
    // (a NULL argument faults exactly like the C does, instead of tripping a
    // Rust-side null check).
    let mut house: House = std::ptr::read(the_house);

    // Match printf()'s stream semantics: everything goes to stdout and is
    // flushed before returning to the caller.
    let mut buf: Vec<u8> = Vec::new();
    imp::run(&mut buf, &mut house, extra_bedrooms);
    std::ptr::write(the_house, house);

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&buf);
    let _ = lock.flush();
}

/// `int main(void)`
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::program_main();
    0
}
