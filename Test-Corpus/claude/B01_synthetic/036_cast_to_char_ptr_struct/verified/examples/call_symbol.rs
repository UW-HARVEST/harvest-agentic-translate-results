//! Differential-test helper.
//!
//! ```text
//! call_symbol <library.so> main
//! call_symbol <library.so> driver_batch        # values on stdin, one per line
//! call_symbol <library.so> driver_wide_batch   # same, called through fn(i64)
//! ```
//!
//! Loads the given shared library with `dlopen`, resolves the requested
//! exported symbol with `dlsym`, and calls it. Its stdout therefore contains
//! *only* what the loaded library wrote, which is what makes the comparison
//! trustworthy: nothing from the test harness can be mixed in.
//!
//! A separate process is used because
//!  * the exported `main` consumes stdin, and each implementation buffers
//!    stdin internally in a way that cannot be reset from the outside, and
//!  * a test harness writing its own progress output to file descriptor 1
//!    would otherwise interleave with the library's output.
//!
//! The process exits with the value the called function returned, so callers
//! can compare return values as well as stdout.

use std::io::Read;
use std::os::raw::{c_int, c_void};

extern "C" {
    /// `fflush(NULL)` drains every C stdio stream, so the loaded C library's
    /// buffered `printf` output is written before this process exits.
    fn fflush(stream: *mut c_void) -> c_int;
    /// Used by the "host" modes below to read and write through *libc's* own
    /// `stdin`/`stdout`, the way a C program embedding this library would.
    fn scanf(format: *const std::os::raw::c_char, ...) -> c_int;
    fn printf(format: *const std::os::raw::c_char, ...) -> c_int;
    /// Terminates without running stdio cleanup, so anything still sitting in
    /// libc's `stdout` buffer is lost.
    fn _exit(status: c_int) -> !;
}

/// `scanf("%d", &v)` performed by the host process itself.
unsafe fn host_scanf() -> (c_int, c_int) {
    let mut v: c_int = -999;
    let r = scanf(b"%d\0".as_ptr() as *const std::os::raw::c_char, &mut v as *mut c_int);
    (r, v)
}

/// `printf("%s", s)` performed by the host process itself.
unsafe fn host_printf(s: &str) {
    let c = std::ffi::CString::new(s).unwrap();
    printf(b"%s\0".as_ptr() as *const std::os::raw::c_char, c.as_ptr());
}

fn read_values() -> Vec<i64> {
    let mut s = String::new();
    std::io::stdin()
        .read_to_string(&mut s)
        .expect("read values from stdin");
    s.split_whitespace()
        .map(|t| t.parse::<i64>().unwrap_or_else(|e| panic!("bad value {t:?}: {e}")))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <library.so> main|driver_batch|driver_wide_batch", args[0]);
        std::process::exit(2);
    }
    let path = &args[1];
    let mode = args[2].as_str();

    // Values are read before the library is loaded so that stdin is fully
    // consumed and cannot interfere with anything the library does.
    let values = match mode {
        "driver_batch" | "driver_wide_batch" => read_values(),
        _ => Vec::new(),
    };

    let rc = unsafe {
        let lib = match libloading::Library::new(path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("dlopen {path}: {e}");
                std::process::exit(3);
            }
        };
        match mode {
            "main" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    match lib.get(b"main\0") {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("dlsym main: {e}");
                            std::process::exit(4);
                        }
                    };
                f()
            }
            // Calls the exported `main` several times in one process. Each call
            // continues reading the same stdin stream, which exposes how a
            // conversion leaves the stream positioned (glibc pushes the
            // terminating/mismatching character back with `ungetc`).
            //
            // The per-call return values go to stderr so that stdout stays
            // exactly the concatenation of the library's own output.
            "main_n" => {
                let n: usize = args
                    .get(3)
                    .expect("main_n needs a count")
                    .parse()
                    .expect("count must be a number");
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    match lib.get(b"main\0") {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("dlsym main: {e}");
                            std::process::exit(4);
                        }
                    };
                let mut rets = Vec::with_capacity(n);
                for _ in 0..n {
                    rets.push(f().to_string());
                }
                // Drain the C stdio buffer before writing to stderr, so the two
                // streams cannot be reordered relative to each other.
                fflush(std::ptr::null_mut());
                eprint!("{}", rets.join(" "));
                0
            }
            "driver_batch" => {
                let f: libloading::Symbol<unsafe extern "C" fn(c_int)> = match lib.get(b"driver\0")
                {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("dlsym driver: {e}");
                        std::process::exit(4);
                    }
                };
                for v in values {
                    f(v as i32 as c_int);
                }
                0
            }
            // Calls the very same `driver` symbol through a 64-bit parameter
            // type, to observe what each implementation does with the unused
            // upper half of the argument register.
            "driver_wide_batch" => {
                let f: libloading::Symbol<unsafe extern "C" fn(i64)> = match lib.get(b"driver\0") {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("dlsym driver: {e}");
                        std::process::exit(4);
                    }
                };
                for v in values {
                    f(v);
                }
                0
            }
            // Calls the exported `main` `count` times, appending `text` to the
            // file named by `path` after every call. When that file is also
            // this process's stdin, the stream reaches end-of-file and then
            // grows again, which is how C's *sticky* end-of-file indicator
            // becomes observable: once `stdin` has seen EOF, glibc's `scanf`
            // fails without reading again (until `clearerr`, which the C code
            // never calls).
            //
            // Usage: main_growing <count> <path> <text>
            "main_growing" => {
                use std::io::Write;
                let count: usize = args
                    .get(3)
                    .expect("main_growing needs a count")
                    .parse()
                    .expect("count must be a number");
                let path = args.get(4).expect("main_growing needs a path").clone();
                let text = args.get(5).cloned().unwrap_or_default();
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    match lib.get(b"main\0") {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("dlsym main: {e}");
                            std::process::exit(4);
                        }
                    };
                let mut rets = Vec::with_capacity(count);
                for _ in 0..count {
                    rets.push(f().to_string());
                    let mut file = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .expect("append to the stdin file");
                    file.write_all(text.as_bytes()).expect("write");
                    file.flush().expect("flush");
                }
                fflush(std::ptr::null_mut());
                eprint!("{}", rets.join(" "));
                0
            }
            // The next four modes act as a *C host* embedding the library: they
            // use libc's own `stdin`/`stdout`, which the C library shares and a
            // reimplementation on top of `std::io` would not.
            //
            // The host's own `scanf` result is reported on stderr.
            "host_scanf_then_main" => {
                let (r, v) = host_scanf();
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("dlsym main");
                let rc = f();
                fflush(std::ptr::null_mut());
                eprint!("host scanf={r} v={v}");
                rc
            }
            "main_then_host_scanf" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("dlsym main");
                let rc = f();
                let (r, v) = host_scanf();
                fflush(std::ptr::null_mut());
                eprint!("host scanf={r} v={v}");
                rc
            }
            // Writes markers through libc's `stdout` before and after the call,
            // so the *order* of the bytes on the descriptor is observable.
            "host_printf_around_main" => {
                host_printf("HOST-BEFORE|");
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("dlsym main");
                let rc = f();
                host_printf("HOST-AFTER|");
                rc
            }
            // Leaves through `_exit`, which skips stdio cleanup: whatever the
            // library left in libc's `stdout` buffer is dropped.
            "main_then_raw_exit" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("dlsym main");
                let rc = f();
                _exit(rc);
            }
            other => {
                eprintln!("unknown mode: {other}");
                std::process::exit(2);
            }
        }
    };

    unsafe {
        fflush(std::ptr::null_mut());
    }
    std::process::exit(rc);
}
