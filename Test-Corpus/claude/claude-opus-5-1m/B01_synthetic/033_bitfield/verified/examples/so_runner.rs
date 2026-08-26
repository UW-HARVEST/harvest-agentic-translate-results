//! Out-of-process driver for the differential tests.
//!
//! `dlopen`s a shared object (either the C one built from `c_src/src/main.c` or
//! the Rust `cdylib`) with `libloading` and calls one of its exported C
//! symbols.  A fresh process per call is what makes the `main` comparison
//! meaningful (`main` consumes the process' `stdin`, and both libc's `stdin`
//! FILE and Rust's `io::stdin()` are process-global, buffered objects), and it
//! is also the only way to observe a fatal signal (e.g. `print_foo(NULL)`).
//!
//! Usage:
//!   so_runner <so> main
//!   so_runner <so> driver <x:u32> <y:u32> <b:u8> <z:i32>
//!   so_runner <so> print_foo <bits:u8> <pad0:u8> <pad1:u8> <pad2:u8> <z:i32>
//!   so_runner <so> print_foo_null

use std::os::raw::{c_int, c_uint};

type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
type PrintFooFn = unsafe extern "C" fn(*const u8);
type MainFn = unsafe extern "C" fn() -> c_int;

#[repr(C, align(4))]
struct RawFoo([u8; 8]);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: so_runner <so> <main|driver|print_foo|print_foo_null> [args...]");
        std::process::exit(2);
    }
    let so = &args[1];
    let cmd = &args[2];

    let lib = unsafe { libloading::Library::new(so) }
        .unwrap_or_else(|e| panic!("dlopen({so}) failed: {e}"));

    let rc = match cmd.as_str() {
        "main" => unsafe {
            let f = lib.get::<MainFn>(b"main\0").expect("no `main` symbol");
            f()
        },
        "driver" => unsafe {
            assert_eq!(args.len(), 7, "driver needs x y b z");
            let x: c_uint = args[3].parse().unwrap();
            let y: c_uint = args[4].parse().unwrap();
            let b: u8 = args[5].parse().unwrap();
            let z: c_int = args[6].parse().unwrap();
            let f = lib.get::<DriverFn>(b"driver\0").expect("no `driver` symbol");
            f(x, y, b, z);
            0
        },
        "print_foo" => unsafe {
            assert_eq!(args.len(), 8, "print_foo needs bits pad0 pad1 pad2 z");
            let bits: u8 = args[3].parse().unwrap();
            let pad: [u8; 3] = [
                args[4].parse().unwrap(),
                args[5].parse().unwrap(),
                args[6].parse().unwrap(),
            ];
            let z: c_int = args[7].parse().unwrap();
            let mut raw = RawFoo([0u8; 8]);
            raw.0[0] = bits;
            raw.0[1..4].copy_from_slice(&pad);
            raw.0[4..8].copy_from_slice(&z.to_ne_bytes());
            let f = lib
                .get::<PrintFooFn>(b"print_foo\0")
                .expect("no `print_foo` symbol");
            f(raw.0.as_ptr());
            0
        },
        "print_foo_null" => unsafe {
            let f = lib
                .get::<PrintFooFn>(b"print_foo\0")
                .expect("no `print_foo` symbol");
            f(std::ptr::null());
            0
        },
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(2);
        }
    };

    // Flush both worlds: Rust's `io::stdout()` explicitly, and glibc's
    // `stdout` FILE through `exit()`'s `_IO_cleanup` (exactly like the real
    // program does when its `main` returns).
    let _ = std::io::Write::flush(&mut std::io::stdout());
    std::process::exit(rc);
}
