//! Test helper: calls the `main` symbol exported by a shared object.
//!
//! `tests/driver_cli.rs` uses it to drive the `main` export of both the C and
//! the Rust shared object exactly like an external caller would: the library is
//! `dlopen`ed and its `main` is called with this process's stdin/stdout.
//!
//! usage: so_main_runner <path to .so>
//!
//! Only `dlopen`/`dlsym` from libc are used, so this helper needs no
//! dependencies of its own.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

const RTLD_NOW: c_int = 2;

extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        // `cargo test` also builds and may run the helper without arguments.
        None => {
            eprintln!("usage: so_main_runner <path to .so>");
            return;
        }
    };
    let cpath = CString::new(path.clone()).expect("path");
    let code = unsafe {
        let handle = dlopen(cpath.as_ptr(), RTLD_NOW);
        if handle.is_null() {
            let err = dlerror();
            let msg = if err.is_null() {
                "unknown error".to_string()
            } else {
                std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            panic!("dlopen({path}) failed: {msg}");
        }
        let name = CString::new("main").unwrap();
        let sym = dlsym(handle, name.as_ptr());
        if sym.is_null() {
            panic!("{path} exports no `main`");
        }
        let entry: extern "C" fn() -> c_int = std::mem::transmute(sym);
        entry()
    };
    // Exiting through libc flushes the C library's stdio buffers as well.
    std::process::exit(code);
}
