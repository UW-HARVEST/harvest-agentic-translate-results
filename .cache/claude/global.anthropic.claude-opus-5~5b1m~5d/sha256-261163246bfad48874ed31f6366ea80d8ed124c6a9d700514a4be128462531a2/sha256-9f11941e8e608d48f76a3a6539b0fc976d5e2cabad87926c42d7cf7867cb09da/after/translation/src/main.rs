// Translation of c_src/src/mdmain.c
//
// The C build compiles mdcore.c and mdmain.c into the `driver` executable, so
// the binary contains its own copy of the core module (mirrored here with
// `#[path]` module declarations, since the library is a cdylib).

#[path = "mdconfig.rs"]
pub mod mdconfig;
#[path = "mdcore.rs"]
pub mod mdcore;

use std::ffi::c_int;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use mdconfig::{INIT, OP_NAME_C, REPEAT};

/* mdmain.c uses C stdio; call the same libc functions so buffering, flush
 * order and formatting are identical to the C driver's. */
extern "C" {
    fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// glibc `atoi(s)` == `(int)strtol(s, NULL, 10)`.
fn atoi(bytes: &[u8]) -> c_int {
    let mut idx = 0usize;
    while idx < bytes.len()
        && matches!(bytes[idx], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    {
        idx += 1;
    }
    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }
    let mut acc: i64 = 0;
    let mut overflow = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let d = (bytes[idx] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        idx += 1;
    }
    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };
    value as c_int
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args.len();

    if argc < 3 {
        // `fprintf(stderr, "usage: %s A B\n", argv[0])`.  stderr is unbuffered
        // in C stdio and in Rust, and argv[0] is written as raw bytes (it need
        // not be valid UTF-8).
        let prog: &[u8] = args.first().map(|s| s.as_bytes()).unwrap_or(b"");
        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        let _ = lock.write_all(b"usage: ");
        let _ = lock.write_all(prog);
        let _ = lock.write_all(b" A B\n");
        let _ = lock.flush();
        std::process::exit(2);
    }

    let a = atoi(args[1].as_bytes());
    let b = atoi(args[2].as_bytes());

    let r_call = (mdconfig::op_fn())(a, b);
    let mut acc: c_int = INIT;
    acc = mdconfig::run_loop(acc);

    let x1 = mdcore::helper_call(a, b);
    let x2 = mdcore::helper_ptr(a, b);
    let x3 = mdcore::use_generated(REPEAT);
    let g = (mdcore::G_OP)(a, b);

    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);

    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            OP_NAME_C.as_ptr() as *const std::ffi::c_char,
            r_call as c_int,
            acc as c_int,
            g as c_int,
        );
        printf(c"summary=%d\n".as_ptr(), summary as c_int);
    }
    // `return 0` from main: libc `exit()` flushes all C stdio streams.
    std::process::exit(0);
}
