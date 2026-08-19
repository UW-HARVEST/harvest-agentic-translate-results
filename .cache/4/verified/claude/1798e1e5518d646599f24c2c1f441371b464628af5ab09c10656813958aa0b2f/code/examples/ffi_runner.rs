//! Loads a driver shared object with `libloading` and calls one exported symbol.
//!
//! Used by the differential tests for the `main` export, which consumes stdin
//! and therefore needs a fresh process (and a fresh stdio state) per input.
//! Both the C `.so` and the Rust `.so` are driven through this exact same path,
//! so the comparison is symmetric.
//!
//! Usage: `ffi_runner <path/to/lib.so> <symbol>`

use std::io::Write;
use std::os::raw::c_int;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let lib_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: ffi_runner <lib.so> <symbol>");
            std::process::exit(2);
        }
    };
    let symbol = match args.next() {
        Some(s) => s.into_string().expect("symbol must be UTF-8"),
        None => {
            eprintln!("usage: ffi_runner <lib.so> <symbol>");
            std::process::exit(2);
        }
    };

    let rc: c_int = unsafe {
        let lib = libloading::Library::new(&lib_path)
            .unwrap_or_else(|e| panic!("dlopen {lib_path:?}: {e}"));
        match symbol.as_str() {
            // int (*)(void)
            "main" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> = lib
                    .get(b"main")
                    .unwrap_or_else(|e| panic!("dlsym main: {e}"));
                f()
            }
            // void (*)(void)
            "bad" | "good" => {
                let f: libloading::Symbol<unsafe extern "C" fn()> = lib
                    .get(symbol.as_bytes())
                    .unwrap_or_else(|e| panic!("dlsym {symbol}: {e}"));
                f();
                0
            }
            other => panic!("ffi_runner does not know symbol {other:?}"),
        }
    };

    // Flush the callee's C `FILE*` buffers and our own Rust stdout before the
    // process image goes away, so nothing is lost or reordered.
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();
    std::process::exit(rc);
}
