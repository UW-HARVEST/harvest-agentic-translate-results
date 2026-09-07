//! Subprocess worker for the Phase C rows whose "expected C result" is a
//! FAULT rather than a return value (`ERRORS.md` rows 10, 24, 25).
//!
//! Usage: `crash_worker <path-to-.so> <case>`
//!
//! Loads the given shared library with `libloading` and performs the one
//! offending call. The parent test spawns this twice — once per library — and
//! asserts both processes terminate with the *same* signal / exit status.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: crash_worker <lib.so> <case>");
        std::process::exit(64);
    }
    let (path, case) = (&args[1], args[2].as_str());

    unsafe {
        let lib = Library::new(path).expect("dlopen failed");

        match case {
            // ERRORS row 10: process_string(NULL) -> unconditional `*str`
            "process_string_null" => {
                let f: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
                    lib.get(b"process_string").unwrap();
                let v = f(std::ptr::null());
                println!("returned {v}");
            }
            // ERRORS row 24: init_matrix(NULL) -> unconditional store
            "init_matrix_null" => {
                let f: Symbol<unsafe extern "C" fn(*mut c_int)> = lib.get(b"init_matrix").unwrap();
                f(std::ptr::null_mut());
                println!("returned");
            }
            // ERRORS row 25: shift_array(NULL, 4, 1) -> guard PASSES, memmove(NULL)
            "shift_array_null_active" => {
                let f: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
                    lib.get(b"shift_array").unwrap();
                f(std::ptr::null_mut(), 4, 1);
                println!("returned");
            }
            // ERRORS row 26: shift_array(NULL, 4, 0) -> guard FAILS, safe
            "shift_array_null_guarded" => {
                let f: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
                    lib.get(b"shift_array").unwrap();
                f(std::ptr::null_mut(), 4, 0);
                f(std::ptr::null_mut(), 4, -1);
                f(std::ptr::null_mut(), 4, 4);
                f(std::ptr::null_mut(), 4, 99);
                f(std::ptr::null_mut(), 0, 1);
                f(std::ptr::null_mut(), -1, 1);
                println!("all guarded calls returned safely");
            }
            // ERRORS row 3: arity(len<2, NULL) -> guard runs before any deref
            "arity_null_guarded" => {
                let f: Symbol<unsafe extern "C" fn(c_int, *const c_int) -> c_int> =
                    lib.get(b"arity").unwrap();
                for len in [0i32, 1, 256, 257, 512, -256, -255] {
                    let v = f(len, std::ptr::null());
                    println!("arity({len}, NULL) = {v}");
                }
            }
            // arity with a dispatching len and NULL params -> deref of NULL
            "arity_null_active" => {
                let f: Symbol<unsafe extern "C" fn(c_int, *const c_int) -> c_int> =
                    lib.get(b"arity").unwrap();
                let v = f(4, std::ptr::null());
                println!("returned {v}");
            }
            other => {
                eprintln!("unknown case {other}");
                std::process::exit(64);
            }
        }
    }
}
