// Differential test harness.
//
// Loads ONE shared library (either the C `libdriver.so` or the Rust
// `libdriver.so`) with `libloading` and replays a script of calls against its
// exported C symbols. Everything the library prints goes to this process's
// stdout untouched; the harness itself never writes to stdout, so the parent
// test can compare the two runs byte-for-byte.
//
// Usage:  harness <path-to-libdriver.so>   < script
//
// Script grammar (one op per line, blank lines ignored):
//   printIntLine <i32>
//   printLine NULL
//   printLine HEX <hex-bytes>      // bytes are NUL-terminated by the harness
//   bad
//   good
//   driver <i32>
//   driverWide <i64>              // calls `driver` through an i64-typed
//                                 // signature, so the argument register's
//                                 // upper half carries non-zero garbage while
//                                 // the `int` parameter slot may be zero

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Write};

unsafe extern "C" {
    // Flush every stdio stream before exit so the ordering of the library's
    // `printf`/`puts` output is fully materialised.
    unsafe fn fflush(stream: *mut c_void) -> c_int;
}

fn die(msg: &str) -> ! {
    let _ = writeln!(std::io::stderr(), "harness: {msg}");
    std::process::exit(2);
}

fn unhex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        die("odd-length hex payload");
    }
    let mut out = Vec::with_capacity(b.len() / 2);
    for pair in b.chunks(2) {
        let hi = (pair[0] as char).to_digit(16).unwrap_or_else(|| die("bad hex"));
        let lo = (pair[1] as char).to_digit(16).unwrap_or_else(|| die("bad hex"));
        out.push((hi * 16 + lo) as u8);
    }
    out
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let so_path = match args.next() {
        Some(p) => p,
        None => die("missing <path-to-libdriver.so> argument"),
    };

    let mut script = String::new();
    if std::io::stdin().read_to_string(&mut script).is_err() {
        die("failed to read script from stdin");
    }

    // SAFETY: loading a trusted, locally built shared object.
    let lib = unsafe { libloading::Library::new(&so_path) }
        .unwrap_or_else(|e| die(&format!("dlopen {so_path:?} failed: {e}")));

    // Every symbol is resolved up-front through the dynamic symbol table, so a
    // missing `#[no_mangle]` export in the Rust build is an immediate failure.
    unsafe {
        let print_line: libloading::Symbol<unsafe extern "C" fn(*const c_char)> = lib
            .get(b"printLine\0")
            .unwrap_or_else(|e| die(&format!("printLine: {e}")));
        let print_int_line: libloading::Symbol<unsafe extern "C" fn(c_int)> = lib
            .get(b"printIntLine\0")
            .unwrap_or_else(|e| die(&format!("printIntLine: {e}")));
        let bad: libloading::Symbol<unsafe extern "C" fn()> =
            lib.get(b"bad\0").unwrap_or_else(|e| die(&format!("bad: {e}")));
        let good: libloading::Symbol<unsafe extern "C" fn()> = lib
            .get(b"good\0")
            .unwrap_or_else(|e| die(&format!("good: {e}")));
        let driver: libloading::Symbol<unsafe extern "C" fn(c_int)> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| die(&format!("driver: {e}")));
        // Deliberately mis-typed view of the same symbol: lets a script pass a
        // 64-bit value whose low 32 bits are the `int` the callee sees.
        let driver_wide: libloading::Symbol<unsafe extern "C" fn(i64)> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| die(&format!("driver: {e}")));

        for line in script.lines() {
            let line = line.trim_end_matches(['\r']);
            if line.is_empty() {
                continue;
            }
            let mut it = line.splitn(3, ' ');
            let op = it.next().unwrap();
            match op {
                "printIntLine" => {
                    let v: i32 = it
                        .next()
                        .unwrap_or_else(|| die("printIntLine needs an argument"))
                        .parse()
                        .unwrap_or_else(|_| die("printIntLine argument is not an i32"));
                    print_int_line(v as c_int);
                }
                "printLine" => match it.next() {
                    Some("NULL") => print_line(std::ptr::null()),
                    Some("HEX") => {
                        let payload = it.next().unwrap_or("");
                        let mut bytes = unhex(payload);
                        if bytes.contains(&0) {
                            die("printLine HEX payload must not contain an interior NUL");
                        }
                        bytes.push(0);
                        print_line(bytes.as_ptr() as *const c_char);
                    }
                    _ => die("printLine needs NULL or HEX <hex>"),
                },
                "bad" => bad(),
                "good" => good(),
                "driver" => {
                    let v: i32 = it
                        .next()
                        .unwrap_or_else(|| die("driver needs an argument"))
                        .parse()
                        .unwrap_or_else(|_| die("driver argument is not an i32"));
                    driver(v as c_int);
                }
                "driverWide" => {
                    let v: i64 = it
                        .next()
                        .unwrap_or_else(|| die("driverWide needs an argument"))
                        .parse()
                        .unwrap_or_else(|_| die("driverWide argument is not an i64"));
                    driver_wide(v);
                }
                other => die(&format!("unknown op {other:?}")),
            }
        }

        fflush(std::ptr::null_mut());
    }
}
