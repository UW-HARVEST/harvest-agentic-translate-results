//! Out-of-process driver used by the differential tests.
//!
//! Usage: `ffi_runner <path-to-shared-object> <call-count>`
//!
//! Loads the given shared object with `libloading`, resolves the exported
//! `helloworld` symbol, and calls it `call-count` times. Standard output carries
//! *only* what the library itself writes, so the test can compare it
//! byte-for-byte; the return values are reported on standard error.
//!
//! Running the library in a child process (rather than redirecting the test
//! process's own file descriptor 1) keeps the C `stdio` buffers and the Rust
//! `std::io::stdout` buffer completely isolated from the test harness, which
//! writes to stdout concurrently.

use std::ffi::{c_int, c_void};
use std::io::Write;

unsafe extern "C" {
    /// `fflush(NULL)` flushes every open C output stream.
    fn fflush(stream: *mut c_void) -> c_int;
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let lib_path = args.next().expect("usage: ffi_runner <lib.so> <calls>");
    let calls: usize = args
        .next()
        .expect("usage: ffi_runner <lib.so> <calls>")
        .to_string_lossy()
        .parse()
        .expect("call count must be an integer");

    // SAFETY: both libraries are plain C-ABI shared objects.
    let lib = unsafe { libloading::Library::new(&lib_path) }
        .unwrap_or_else(|e| panic!("failed to load {lib_path:?}: {e}"));
    let helloworld: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
        unsafe { lib.get(b"helloworld\0") }
            .unwrap_or_else(|e| panic!("`helloworld` missing from {lib_path:?}: {e}"));

    let mut rets = Vec::with_capacity(calls);
    for _ in 0..calls {
        rets.push(unsafe { helloworld() });
    }

    // Flush both runtimes' buffers before reporting, so stdout is complete.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let rendered: Vec<String> = rets.iter().map(|r| r.to_string()).collect();
    eprintln!("RETS:{}", rendered.join(","));
}
