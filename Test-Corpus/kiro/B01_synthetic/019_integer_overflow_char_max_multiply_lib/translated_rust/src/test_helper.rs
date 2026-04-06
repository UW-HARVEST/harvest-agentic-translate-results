use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, CString};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: test_helper <lib_path> <func_name> [args...]");
        std::process::exit(1);
    }
    let lib_path = &args[1];
    let func_name = &args[2];

    let lib = unsafe { Library::new(lib_path).expect("failed to load library") };

    match func_name.as_str() {
        "printHexCharLine" => {
            let val: i8 = args[3].parse().unwrap();
            let f: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { lib.get(b"printHexCharLine").unwrap() };
            unsafe { f(val) };
        }
        "printLine" => {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { lib.get(b"printLine").unwrap() };
            if args[3] == "__NULL__" {
                unsafe { f(std::ptr::null()) };
            } else {
                let s = CString::new(args[3].as_str()).unwrap();
                unsafe { f(s.as_ptr()) };
            }
        }
        "bad" => {
            let f: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"bad").unwrap() };
            unsafe { f() };
        }
        "good" => {
            let f: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"good").unwrap() };
            unsafe { f() };
        }
        "driver" => {
            let val: c_int = args[3].parse().unwrap();
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { lib.get(b"driver").unwrap() };
            unsafe { f(val) };
        }
        _ => {
            eprintln!("Unknown function: {}", func_name);
            std::process::exit(1);
        }
    }
}
