// Helper binary: load a shared library and call betagamma once with the
// arguments given on the command line, then print the result.
//
// Usage: betagamma_runner <path/to/lib.so> <a> <b> <c> <d>

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: {} <so_path> <a> <b> <c> <d>", args[0]);
        return ExitCode::from(2);
    }
    let so = &args[1];
    let a: c_int = args[2].parse().expect("parse a");
    let b: c_int = args[3].parse().expect("parse b");
    let c: c_int = args[4].parse().expect("parse c");
    let d: c_int = args[5].parse().expect("parse d");

    unsafe {
        let lib = Library::new(so).expect("load library");
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            lib.get(b"betagamma").expect("find symbol");
        let v = f(a, b, c, d);
        println!("{}", v);
    }
    ExitCode::from(0)
}
