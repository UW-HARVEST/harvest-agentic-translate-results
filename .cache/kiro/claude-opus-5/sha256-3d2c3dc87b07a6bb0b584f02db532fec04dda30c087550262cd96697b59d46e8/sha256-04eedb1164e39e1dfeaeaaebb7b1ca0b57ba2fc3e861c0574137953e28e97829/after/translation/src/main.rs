//! Translation of `c_src/src/mdmain.c` — the `driver` executable.
//!
//! ```c
//! int main(int argc, char **argv) {
//!     if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }
//!     int a = atoi(argv[1]);
//!     int b = atoi(argv[2]);
//!     ...
//! }
//! ```
//!
//! The modules are pulled in with `#[path]` rather than through the `driver`
//! library because the library is a `cdylib` (it mirrors the C object file), so
//! it cannot be linked as a Rust dependency. Compiling the same sources into the
//! binary matches CMake, which feeds `mdcore.c` to both artifacts.

// The two modules are the full translation of `mdcore.c` / `mdmacros.h`; the
// binary only uses part of that surface, so the unused rest is expected.
#![allow(dead_code)]

#[path = "mdcore.rs"]
mod mdcore;
#[path = "mdmacros.rs"]
mod mdmacros;
use core::ffi::c_int;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use mdcore::{G_OP, OP_FN, helper_call, helper_ptr, use_generated};
use mdmacros::{INIT, OP_NAME, REPEAT, run_loop};

/// `atoi` as implemented by glibc: `(int) strtol(nptr, NULL, 10)`.
///
/// Leading C `isspace` characters are skipped, an optional sign may follow, and
/// digits are consumed until the first non-digit. Values that do not fit in a
/// `long` saturate to `LONG_MAX` / `LONG_MIN` before the narrowing cast to
/// `int`; anything unparseable yields `0`.
///
/// Operates on raw bytes because `argv` strings need not be valid UTF-8.
fn atoi(b: &[u8]) -> c_int {
    let mut i = 0;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let neg = match b.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    // Accumulate in i64 (== long on LP64) with saturation, exactly like strtol.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < b.len() && b[i].is_ascii_digit() {
        let d = i64::from(b[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }
    let v: i64 = if overflow {
        if neg { i64::MIN } else { i64::MAX }
    } else if neg {
        acc.wrapping_neg()
    } else {
        acc
    };
    v as c_int
}

fn main() {
    // `args_os` keeps the raw bytes: `argv` is not required to be valid UTF-8
    // and the C code happily passes such strings to `atoi` / `printf("%s")`.
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if argv.len() < 3 {
        let prog = argv.first().map(|s| s.as_bytes()).unwrap_or(b"");
        let mut err = std::io::stderr();
        let _ = err.write_all(b"usage: ");
        let _ = err.write_all(prog);
        let _ = err.write_all(b" A B\n");
        let _ = err.flush();
        std::process::exit(2);
    }
    let a = atoi(argv[1].as_bytes());
    let b = atoi(argv[2].as_bytes());

    let r_call = OP_FN(a, b);
    let acc = run_loop(INIT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = G_OP(a, b);

    println!(
        "op={} call={} acc={} g.call={}",
        OP_NAME.to_str().unwrap(),
        r_call,
        acc,
        g
    );
    println!(
        "summary={}",
        r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(g)
    );
    std::process::exit(0);
}
