//! Test helper: `dlopen` an arbitrary `.so` (C or Rust) and invoke one exported
//! symbol, letting whatever the library `printf`s go straight to this process's
//! stdout.  Used by `tests/differential.rs` to byte-compare the *printed* output
//! of the two libraries without touching the test harness's own fd 1.
//!
//!   socall <so-path> op_add|op_sub|op_mul|helper_call|helper_ptr <a> <b>
//!   socall <so-path> use_generated <n>
//!   socall <so-path> G_OP <a> <b>
//!   socall <so-path> G_OP_NAME

use std::ffi::{c_char, c_int, CStr};

type BinFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnFn = unsafe extern "C" fn(c_int) -> c_int;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: socall <so> <symbol> [args...]");
        std::process::exit(64);
    }
    let lib = unsafe { libloading::Library::new(&args[1]) }.expect("dlopen");
    let sym = args[2].as_str();
    let num = |i: usize| -> c_int { args[i].parse::<i64>().expect("int arg") as c_int };

    match sym {
        "op_add" | "op_sub" | "op_mul" | "helper_call" | "helper_ptr" => {
            let name = format!("{sym}\0");
            let f: libloading::Symbol<BinFn> =
                unsafe { lib.get(name.as_bytes()) }.expect("symbol");
            let r = unsafe { f(num(3), num(4)) };
            println!("ret={r}");
        }
        "use_generated" => {
            let f: libloading::Symbol<UnFn> =
                unsafe { lib.get(b"use_generated\0") }.expect("symbol");
            let r = unsafe { f(num(3)) };
            println!("ret={r}");
        }
        "G_OP" => {
            let g: libloading::Symbol<*const BinFn> =
                unsafe { lib.get(b"G_OP\0") }.expect("symbol");
            let f: BinFn = unsafe { **g };
            let r = unsafe { f(num(3), num(4)) };
            println!("ret={r}");
        }
        "G_OP_NAME" => {
            let g: libloading::Symbol<*const *const c_char> =
                unsafe { lib.get(b"G_OP_NAME\0") }.expect("symbol");
            let p = unsafe { **g };
            assert!(!p.is_null(), "G_OP_NAME is NULL");
            let s = unsafe { CStr::from_ptr(p) };
            println!("ret={}", s.to_string_lossy());
        }
        other => {
            eprintln!("unknown symbol {other}");
            std::process::exit(65);
        }
    }
}
