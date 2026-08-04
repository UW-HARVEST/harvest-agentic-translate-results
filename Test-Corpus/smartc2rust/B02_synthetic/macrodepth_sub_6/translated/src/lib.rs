

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

// FFI declarations for symbols provided by mdcore.c which are not emitted by
// bindgen (build.rs blocklists all functions and types; only constant macros
// are included in bindings.rs).
unsafe extern "C" {
    static mut G_OP: Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;
    static mut G_OP_NAME: *const c_char;

    fn helper_call(a: c_int, b: c_int) -> c_int;
    fn helper_ptr(a: c_int, b: c_int) -> c_int;
    fn use_generated(n: c_int) -> c_int;
}

pub fn rust_get_G_OP() -> Option<unsafe extern "C" fn(c_int, c_int) -> c_int> {
    unsafe { G_OP }
}

pub fn rust_set_G_OP(val: Option<unsafe extern "C" fn(c_int, c_int) -> c_int>) {
    unsafe { G_OP = val; }
}

pub fn rust_get_G_OP_NAME() -> *const c_char {
    unsafe { G_OP_NAME }
}

pub fn rust_set_G_OP_NAME(val: *const c_char) {
    unsafe { G_OP_NAME = val; }
}

fn rust_parse_c_int_arg(s: &str) -> c_int {
    // Emulate atoi: parse leading optional sign and digits, ignore trailing garbage.
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }
    let mut sign: c_int = 1;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        if bytes[idx] == b'-' {
            sign = -1;
        }
        idx += 1;
    }
    let mut value: c_int = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((bytes[idx] - b'0') as c_int);
        idx += 1;
    }
    value.wrapping_mul(sign)
}

#[unsafe(no_mangle)]
pub extern "C" fn mdmain_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    // Convert raw argv into a safe Vec<String>.
    let args: Vec<String> = if argv.is_null() || argc <= 0 {
        Vec::new()
    } else {
        let mut v: Vec<String> = Vec::with_capacity(argc as usize);
        for i in 0..(argc as usize) {
            let p: *mut c_char = unsafe { *argv.add(i) };
            if p.is_null() {
                v.push(String::new());
            } else {
                let cstr = unsafe { CStr::from_ptr(p) };
                v.push(cstr.to_string_lossy().into_owned());
            }
        }
        v
    };

    if argc < 3 {
        let prog = args.get(0).map(|s| s.as_str()).unwrap_or("");
        eprintln!("usage: {} A B", prog);
        return 2;
    }

    let a: c_int = rust_parse_c_int_arg(&args[1]);
    let b: c_int = rust_parse_c_int_arg(&args[2]);

    // r_call = (OP_FN(sub))(a, b) = op_sub(a, b) = a - b
    let r_call: c_int = a - b;

    // acc = INIT_FOR(sub) = INIT_sub (from bindings)
    // RUN_LOOP(sub, acc, 5) => STEP_sub(acc, 0..4) => acc -= 0; acc -= 1; ...; acc -= 4;
    let mut acc: c_int = INIT_sub as c_int;
    for i in 0..5 as c_int {
        acc -= i;
    }

    // Call C helpers through FFI (defined in mdcore.c, declared in bindings.rs).
    let x1: c_int = unsafe { helper_call(a, b) };
    let x2: c_int = unsafe { helper_ptr(a, b) };
    let x3: c_int = unsafe { use_generated(5) };

    // g = G_OP(a, b)
    let g: c_int = {
        let gop = rust_get_G_OP().expect("G_OP is null");
        unsafe { gop(a, b) }
    };

    // op_name from G_OP_NAME (C string).
    let op_name: String = {
        let ptr = rust_get_G_OP_NAME();
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
        }
    };

    println!("op={} call={} acc={} g.call={}", op_name, r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);
    0
}

