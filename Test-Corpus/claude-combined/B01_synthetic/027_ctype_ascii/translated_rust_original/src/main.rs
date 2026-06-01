// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

use std::ffi::CString;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isblank(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

// LLVM has builtin knowledge of `isdigit`/`isalpha`/etc. as libcalls and may
// constant-fold them to return 0/1, which differs from glibc's actual return
// values (it returns the bitmask from __ctype_b). Routing through a function
// pointer placed in a static prevents the optimizer from recognizing the call
// as the well-known libc function and inlining a 0/1 result.
type CtypeFn = unsafe extern "C" fn(c_int) -> c_int;

#[inline(never)]
fn call_ctype(f: CtypeFn, c: c_int) -> c_int {
    // Read the function pointer through a volatile read so the optimizer
    // cannot see which libc function is being invoked.
    let p: *const CtypeFn = &f;
    let f2 = unsafe { std::ptr::read_volatile(p) };
    unsafe { f2(c) }
}

// LC_ALL is 6 on glibc Linux.
const LC_ALL: c_int = 6;

fn driver(c: c_int, out: &mut impl Write) {
    // setlocale(LC_ALL, "C");
    let locale = CString::new("C").unwrap();
    unsafe {
        setlocale(LC_ALL, locale.as_ptr());
    }

    writeln!(out, "alphanumeric: {}", call_ctype(isalnum, c)).unwrap();
    writeln!(out, "alphabetic: {}", call_ctype(isalpha, c)).unwrap();
    writeln!(out, "lowercase: {}", call_ctype(islower, c)).unwrap();
    writeln!(out, "uppercase: {}", call_ctype(isupper, c)).unwrap();
    writeln!(out, "digit: {}", call_ctype(isdigit, c)).unwrap();
    writeln!(out, "hexadecimal: {}", call_ctype(isxdigit, c)).unwrap();
    writeln!(out, "control: {}", call_ctype(iscntrl, c)).unwrap();
    writeln!(out, "graphical: {}", call_ctype(isgraph, c)).unwrap();
    writeln!(out, "space: {}", call_ctype(isspace, c)).unwrap();
    writeln!(out, "blank: {}", call_ctype(isblank, c)).unwrap();
    writeln!(out, "printing: {}", call_ctype(isprint, c)).unwrap();
    writeln!(out, "punctuation: {}", call_ctype(ispunct, c)).unwrap();

    // printf("to lower: %c\n", tolower(c));
    // %c prints the low byte of the integer.
    let low_byte = (call_ctype(tolower, c) & 0xff) as u8;
    out.write_all(b"to lower: ").unwrap();
    out.write_all(&[low_byte]).unwrap();
    out.write_all(b"\n").unwrap();

    let low_byte = (call_ctype(toupper, c) & 0xff) as u8;
    out.write_all(b"to upper: ").unwrap();
    out.write_all(&[low_byte]).unwrap();
    out.write_all(b"\n").unwrap();
}

fn main() {
    // Mirror C's `char c = getchar();`
    // getchar() returns int: either the byte (0..255) or EOF (-1).
    // Storing into `char` truncates to a single byte; on x86_64 Linux `char`
    // is signed, so EOF (-1) stays -1 and a byte like 0xC3 becomes -61.
    // The driver function takes a `char`, which is then implicitly promoted
    // back to int when passed to ctype functions.
    let mut buf = [0u8; 1];
    let n = io::stdin().read(&mut buf).unwrap_or(0);

    let c_int_val: c_int = if n == 0 {
        // EOF: getchar returns -1, stored as char (signed) = -1.
        -1
    } else {
        // Byte read. Stored into `char` (signed on x86_64), then promoted
        // to int with sign extension.
        buf[0] as i8 as c_int
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    driver(c_int_val, &mut handle);
}
