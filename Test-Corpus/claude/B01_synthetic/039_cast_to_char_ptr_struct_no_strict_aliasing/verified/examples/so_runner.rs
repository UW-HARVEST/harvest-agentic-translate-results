//! Differential-test helper: loads a shared library with `libloading` and calls
//! its exported C-ABI symbols, exactly the way an external consumer would.
//!
//! It is used for BOTH shared libraries under test:
//!   * the C build of `c_src/src/main.c` (`gcc -shared`)
//!   * the Rust `cdylib` (`target/<profile>/libdriver.so`)
//!
//! Running each call in a fresh process gives every test case pristine
//! stdin/stdout stream state (the C program calls `main` exactly once per
//! process, so that is the faithful comparison).
//!
//! Usage:
//!   so_runner <lib.so> main                # call `int main()`, exit with its result
//!   so_runner <lib.so> driver <int>        # call `void driver(int)`
//!   so_runner <lib.so> driver-batch        # read ints from stdin, one `driver` call each
//!   so_runner <lib.so> mixed <a> <b>       # driver(a), then main(), then driver(b)
//!   so_runner <lib.so> symbols <name>...   # exit 0 iff every name resolves

use std::io::{Read, Write};
use std::os::raw::{c_int, c_void};

// Same libc instance the dlopen'ed C library uses: `fflush(NULL)` flushes every
// open C stream, which is what the C program's `exit()` would have done.
extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

fn flush_all() {
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

fn die(msg: &str) -> ! {
    eprintln!("so_runner: {msg}");
    std::process::exit(97);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        die("usage: so_runner <lib.so> <main|driver <int>|driver-batch|mixed <int> <int>|symbols ...>");
    }
    let lib_path = &args[1];
    let mode = &args[2];

    // SAFETY: loading a library runs its initializers; both libraries under
    // test are ordinary C-ABI shared objects.
    let lib = unsafe { libloading::Library::new(lib_path) }
        .unwrap_or_else(|e| die(&format!("dlopen {lib_path}: {e}")));

    match mode.as_str() {
        "main" => {
            let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { lib.get(b"main\0") }.unwrap_or_else(|e| die(&format!("dlsym main: {e}")));
            let rc = unsafe { f() };
            flush_all();
            std::process::exit(rc);
        }
        "driver" => {
            if args.len() < 4 {
                die("driver needs an integer argument");
            }
            let v = parse_int(&args[3]);
            let f: libloading::Symbol<unsafe extern "C" fn(c_int)> = unsafe { lib.get(b"driver\0") }
                .unwrap_or_else(|e| die(&format!("dlsym driver: {e}")));
            unsafe { f(v) };
            flush_all();
        }
        "driver-batch" => {
            let f: libloading::Symbol<unsafe extern "C" fn(c_int)> = unsafe { lib.get(b"driver\0") }
                .unwrap_or_else(|e| die(&format!("dlsym driver: {e}")));
            let mut input = String::new();
            let _ = std::io::stdin().read_to_string(&mut input);
            for tok in input.split_whitespace() {
                let v = parse_int(tok);
                unsafe { f(v) };
            }
            flush_all();
        }
        // Call `driver`, then `main`, then `driver` again in ONE process: checks
        // that the two entry points do not interfere through the shared stdout
        // stream state.
        "mixed" => {
            if args.len() < 5 {
                die("mixed needs two integer arguments");
            }
            let a = parse_int(&args[3]);
            let b = parse_int(&args[4]);
            let d: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { lib.get(b"driver\0") }
                    .unwrap_or_else(|e| die(&format!("dlsym driver: {e}")));
            let m: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { lib.get(b"main\0") }.unwrap_or_else(|e| die(&format!("dlsym main: {e}")));
            unsafe { d(a) };
            let rc = unsafe { m() };
            unsafe { d(b) };
            flush_all();
            std::process::exit(rc);
        }
        "symbols" => {
            for name in &args[3..] {
                let mut sym = name.clone().into_bytes();
                sym.push(0);
                let found = unsafe { lib.get::<*const c_void>(&sym) }.is_ok();
                println!("{name} {}", if found { "FOUND" } else { "MISSING" });
                if !found {
                    flush_all();
                    std::process::exit(1);
                }
            }
            flush_all();
        }
        other => die(&format!("unknown mode {other}")),
    }
}

/// Accepts decimal (possibly negative) and `0x`-prefixed hex bit patterns so
/// that tests can name exact 32-bit images.
fn parse_int(s: &str) -> c_int {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16)
            .unwrap_or_else(|e| die(&format!("bad hex {s}: {e}"))) as i32;
    }
    match s.parse::<i64>() {
        Ok(v) => v as i32,
        Err(e) => die(&format!("bad int {s}: {e}")),
    }
}
