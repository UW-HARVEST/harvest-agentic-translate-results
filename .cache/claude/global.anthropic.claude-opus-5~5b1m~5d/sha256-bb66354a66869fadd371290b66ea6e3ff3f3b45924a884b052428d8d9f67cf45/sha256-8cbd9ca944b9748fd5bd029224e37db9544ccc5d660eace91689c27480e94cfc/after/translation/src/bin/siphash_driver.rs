//! Test driver: `siphash_driver <shared-object> <init>`
//!
//! `dlopen`s the given shared object (either the C `.so` or the Rust `.so`),
//! looks up the exported `siphash` symbol, and calls it with `init`.  Nothing
//! else is written to stdout, so the process's stdout is exactly the bytes the
//! library's `printf` calls produced — which makes a byte-for-byte comparison
//! between the C and Rust libraries possible.
//!
//! Uses raw `dlopen`/`dlsym` so the driver needs no dependencies at all.

use std::ffi::{CString, c_char, c_int, c_void};

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *mut c_char;
    fn fflush(stream: *mut c_void) -> c_int;
}

const RTLD_NOW: c_int = 2;

fn last_error() -> String {
    unsafe {
        let e = dlerror();
        if e.is_null() {
            "<no dlerror>".to_string()
        } else {
            std::ffi::CStr::from_ptr(e).to_string_lossy().into_owned()
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <shared-object> <init>", args[0]);
        std::process::exit(2);
    }
    let so = CString::new(args[1].clone()).unwrap();
    let init: c_int = args[2].parse().expect("init must be an i32");

    unsafe {
        let handle = dlopen(so.as_ptr(), RTLD_NOW);
        if handle.is_null() {
            eprintln!("dlopen({}) failed: {}", args[1], last_error());
            std::process::exit(3);
        }
        let name = CString::new("siphash").unwrap();
        let sym = dlsym(handle, name.as_ptr());
        if sym.is_null() {
            eprintln!("dlsym(siphash) failed: {}", last_error());
            std::process::exit(4);
        }
        let f: unsafe extern "C" fn(c_int) = std::mem::transmute(sym);
        f(init);
        // Flush the libc stream the library printed through before exiting.
        fflush(std::ptr::null_mut());
    }
}
