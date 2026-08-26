//! Differential-test helper: `dlopen`s a shared library and calls one of its
//! exported C symbols in a **fresh process**.
//!
//! Both the C shared library (built from `c_src/src/main.c` with
//! `gcc -shared -fPIC`) and the Rust `cdylib` (`libdriver.so`) are driven
//! through this very same program, so the only difference between the two runs
//! is the library that gets loaded.
//!
//! Usage:
//!
//! ```text
//! so_runner <library> main                     # calls `int main()`
//! so_runner <library> printHexCharLine <int>   # calls `void printHexCharLine(char)`
//! ```
//!
//! The process exit code is the value returned by `main` (or 0 for
//! `printHexCharLine`). Standard input/output are inherited untouched, so the
//! caller controls them exactly as it would for the real executable.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let argv0 = args.next().unwrap_or_else(|| "so_runner".to_string());
    let lib_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: {argv0} <library.so> <symbol> [int-arg]");
            return ExitCode::from(64);
        }
    };
    let symbol = args.next().unwrap_or_else(|| "main".to_string());

    unsafe {
        let lib = match libloading::Library::new(&lib_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("dlopen({lib_path}) failed: {e}");
                return ExitCode::from(65);
            }
        };

        match symbol.as_str() {
            "main" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> std::os::raw::c_int> =
                    match lib.get(b"main\0") {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("dlsym(main) failed: {e}");
                            return ExitCode::from(66);
                        }
                    };
                let rc = f();
                // Leak the library: unloading it could run destructors that
                // interfere with the observable output.
                std::mem::forget(lib);
                ExitCode::from((rc & 0xff) as u8)
            }
            "printHexCharLine" => {
                let value: i32 = args
                    .next()
                    .unwrap_or_else(|| "0".to_string())
                    .parse()
                    .expect("int-arg must parse as i32");
                let f: libloading::Symbol<unsafe extern "C" fn(std::os::raw::c_int)> =
                    match lib.get(b"printHexCharLine\0") {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("dlsym(printHexCharLine) failed: {e}");
                            return ExitCode::from(66);
                        }
                    };
                f(value);
                std::mem::forget(lib);
                ExitCode::from(0)
            }
            other => {
                eprintln!("unknown symbol {other}");
                ExitCode::from(67)
            }
        }
    }
}
