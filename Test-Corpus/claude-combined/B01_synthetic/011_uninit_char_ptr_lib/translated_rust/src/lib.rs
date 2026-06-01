// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

use std::ffi::c_char;
use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            // Match C: printf("%s\n", line);
            let fmt = b"%s\n\0".as_ptr() as *const c_char;
            printf(fmt, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // Reproduce the C undefined behavior: declare a local pointer and use it
    // without initialization. The C code does:
    //     char *data;
    //     printLine(data);
    #[allow(invalid_value)]
    let data: *const c_char = unsafe { MaybeUninit::uninit().assume_init() };
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    printLine(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
