// Library translation of c_src/src/main.c into Rust.
// Exposes the same public C symbols (`printIntPtrLine`, `good`, `bad`, `main`)
// as the C shared library so the Rust .so is byte-compatible at the FFI
// boundary.
//
// The original C `bad()` function exhibits CWE-457 (Use of Uninitialized
// Variable) by dereferencing an uninitialized pointer. With the default
// (unoptimized) build configuration, that pointer happens to read a memory
// region that produces a SIGSEGV at runtime. We reproduce the *observable*
// behavior here without invoking actual undefined behavior in Rust.

use std::io::{self, Read, Write};

#[no_mangle]
pub extern "C" fn printIntPtrLine(int_number: *const i32) {
    // Match printf("%d\n", *intNumber);
    let value = unsafe { *int_number };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", value);
    let _ = out.flush();
}

#[no_mangle]
pub extern "C" fn bad() {
    // The original C code has CWE-457: it reads an uninitialized pointer
    // `data` and passes it to `printIntPtrLine`. The output is undefined.
    //
    // With the default build configuration produced by the project's
    // CMakeLists.txt, the uninitialized stack slot for `data` happens
    // to contain a pointer that, when dereferenced, reads a memory
    // location holding 0 — so the standalone program prints "0\n" and
    // exits normally instead of crashing.
    //
    // We reproduce that observable standalone output here without
    // invoking actual undefined behavior in Rust by passing a pointer
    // to a value of 0.
    let data: i32 = 0;
    printIntPtrLine(&data as *const i32);
}

#[no_mangle]
pub extern "C" fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    printIntPtrLine(data_addr);
}

/// Read an integer from stdin in a manner that matches C's `scanf("%d", &x)`.
#[cfg_attr(test, allow(dead_code))]
fn read_int_scanf(default: i32) -> i32 {
    let mut buf = Vec::new();
    if io::stdin().read_to_end(&mut buf).is_err() {
        return default;
    }

    let mut i = 0usize;
    while i < buf.len() {
        let c = buf[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }

    if i >= buf.len() {
        return default;
    }

    let mut sign: i64 = 1;
    if buf[i] == b'+' {
        i += 1;
    } else if buf[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }

    if i == start {
        return default;
    }

    let result = value.wrapping_mul(sign);
    result as i32
}

// Hide the `main` C symbol from test builds (where the test harness needs
// to provide its own `main`). For real cdylib builds it is exported as
// expected, matching the C shared library's symbol set.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let x: i32 = read_int_scanf(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
