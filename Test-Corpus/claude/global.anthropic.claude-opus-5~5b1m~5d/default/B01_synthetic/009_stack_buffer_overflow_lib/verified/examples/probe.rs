//! Out-of-process differential probe.
//!
//! Usage: `probe <path-to-shared-object>` with a newline-separated op script on
//! stdin.  The probe `dlopen`s exactly ONE shared object (so the identically
//! named `driver` symbols of the C and the Rust build can never interpose on
//! each other) and replays the script through it.  Everything the library
//! prints goes to the probe's stdout verbatim; the probe itself only ever
//! writes to stderr.
//!
//! Script grammar (one op per line, blank lines ignored):
//!
//! ```text
//!   L <hex>          printLine(cstr(hex bytes))
//!   N                printLine(NULL)
//!   I <int>          printIntLine(int)
//!   B <int>          bad(int)
//!   G <int>          good(int)
//!   D <int> <int>    driver(goodData, badData)
//! ```
//!
//! Exit code 0 on completion; the caller compares stdout bytes *and* exit
//! status between the C and the Rust run.
//!
//! An optional second argument `flush` makes the probe `fflush(NULL)` after
//! *every* op. That matters for the out-of-bounds `bad()` indices: `stdout` is
//! block-buffered when it is a pipe, so a call that prints ten lines and *then*
//! dies on `ret` loses all ten to the un-flushed buffer. Flushing per op
//! separates "what did the library print" from "did the process survive".

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;

extern "C" {
    /// `fflush(NULL)` flushes every open output stream, including the `stdout`
    /// the freshly `dlopen`ed library's `printf` calls wrote into.
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    /// glibc exports `stdout` as a data symbol; the `dlopen`ed library's
    /// `printf` writes into this very same `FILE`.
    static mut stdout: *mut c_void;
}

/// `_IONBF` on glibc.
const IONBF: c_int = 2;

type FnPtr = unsafe extern "C" fn(*const c_char);
type FnInt = unsafe extern "C" fn(c_int);
type FnIntInt = unsafe extern "C" fn(c_int, c_int);

fn unhex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    assert!(b.len() % 2 == 0, "odd hex length");
    (0..b.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("bad hex"))
        .collect()
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let lib_path = args.next().expect("usage: probe <lib.so> [flush]");
    let flush_each = args.next().map(|a| a == "flush").unwrap_or(false);

    let mut script = String::new();
    std::io::stdin()
        .read_to_string(&mut script)
        .expect("read script");

    unsafe {
        if flush_each {
            // Make stdout unbuffered *before* any library call, so that output
            // produced inside a call that later dies on `ret` is still
            // observable. Without this, a block-buffered pipe silently loses
            // everything the doomed call printed.
            setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
        }

        let lib = libloading::Library::new(&lib_path)
            .unwrap_or_else(|e| panic!("dlopen {lib_path:?}: {e}"));

        let print_line: libloading::Symbol<FnPtr> = lib.get(b"printLine\0").expect("printLine");
        let print_int_line: libloading::Symbol<FnInt> =
            lib.get(b"printIntLine\0").expect("printIntLine");
        let bad: libloading::Symbol<FnInt> = lib.get(b"bad\0").expect("bad");
        let good: libloading::Symbol<FnInt> = lib.get(b"good\0").expect("good");
        let driver: libloading::Symbol<FnIntInt> = lib.get(b"driver\0").expect("driver");

        for line in script.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            let op = it.next().unwrap();
            match op {
                "L" => {
                    let mut bytes = unhex(it.next().unwrap_or(""));
                    bytes.push(0); // NUL terminate
                    print_line(bytes.as_ptr() as *const c_char);
                }
                "N" => print_line(std::ptr::null()),
                "I" => print_int_line(it.next().unwrap().parse::<i32>().unwrap() as c_int),
                "B" => bad(it.next().unwrap().parse::<i32>().unwrap() as c_int),
                "G" => good(it.next().unwrap().parse::<i32>().unwrap() as c_int),
                "D" => {
                    let a = it.next().unwrap().parse::<i32>().unwrap() as c_int;
                    let b = it.next().unwrap().parse::<i32>().unwrap() as c_int;
                    driver(a, b);
                }
                other => panic!("unknown op {other:?}"),
            }
            if flush_each {
                fflush(std::ptr::null_mut());
            }
        }

        fflush(std::ptr::null_mut());
    }
}
