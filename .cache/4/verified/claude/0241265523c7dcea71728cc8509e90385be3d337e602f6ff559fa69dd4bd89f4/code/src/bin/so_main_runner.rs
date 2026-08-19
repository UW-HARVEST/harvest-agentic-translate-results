//! Test helper: `dlopen()` a shared object and invoke one of its exported
//! C-ABI entry points in a *fresh* process.
//!
//! The differential tests use this for the entry points that consume `stdin`
//! (i.e. `main`), because glibc's `FILE *stdin` and Rust's `std::io::stdin()`
//! both keep a process-wide read-ahead buffer.  Running every invocation in
//! its own process guarantees that the C object and the Rust object each see
//! exactly the same untouched byte stream, which is the only way to compare
//! them fairly.
//!
//! Usage: `so_main_runner <path-to-.so> <request>`
//!
//! | request                | dlsym'd signature            | behavior |
//! |------------------------|------------------------------|----------|
//! | `main`                 | `int  (*)(void)`             | exits with the returned value |
//! | `bad`                  | `void (*)(void)`             | exits 0 |
//! | `good`                 | `void (*)(void)`             | exits 0 |
//! | `printLine:@null`      | `void (*)(const char *)`     | passes `NULL` |
//! | `printLine:@file:PATH` | `void (*)(const char *)`     | passes the raw bytes of `PATH` as a NUL-terminated string |

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <path-to-.so> <request>", args[0]);
        std::process::exit(2);
    }
    let lib_path = &args[1];
    let request = &args[2];

    unsafe {
        let lib = match libloading::Library::new(lib_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("dlopen({lib_path}) failed: {e}");
                std::process::exit(3);
            }
        };

        if let Some(rest) = request.strip_prefix("printLine:") {
            let f: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"printLine\0").expect("printLine not exported");
            if rest == "@null" {
                f(std::ptr::null());
            } else if let Some(path) = rest.strip_prefix("@file:") {
                let bytes = std::fs::read(path).expect("read arg file");
                let cstr = CString::new(bytes).expect("interior NUL in arg file");
                f(cstr.as_ptr());
            } else {
                eprintln!("unknown printLine request: {rest}");
                std::process::exit(2);
            }
            flush();
            std::process::exit(0);
        }

        match request.as_str() {
            "main" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("main not exported");
                let rc = f();
                flush();
                std::process::exit(rc);
            }
            "bad" | "good" => {
                let name = format!("{request}\0");
                let f: libloading::Symbol<unsafe extern "C" fn()> =
                    lib.get(name.as_bytes()).expect("symbol not exported");
                f();
                flush();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown request: {other}");
                std::process::exit(2);
            }
        }
    }
}

extern "C" {
    /// `fflush(NULL)` flushes every open C stream, which is how the C object's
    /// `puts()` output is forced out before this process exits.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Flush both the C `stdout` (used by the C object under test) and this
/// process's Rust `stdout`.  The Rust object under test flushes its own
/// `std::io::stdout()` from inside its export wrappers, since a `dlopen()`ed
/// `cdylib` carries its own copy of `std` with its own buffer.
fn flush() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}
