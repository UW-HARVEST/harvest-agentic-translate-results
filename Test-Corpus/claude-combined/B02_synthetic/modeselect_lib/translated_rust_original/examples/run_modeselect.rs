// Helper binary used by the integration tests: loads a .so given on the
// command line, calls `modeselect(a, b, c, d)`, and exits. The test harness
// captures this process's stdout, which is the only writer to fd 1 — so the
// captured bytes match exactly what the underlying C/Rust library printed.
//
// Usage: run_modeselect <so-path> <a> <b> <c> <d>

use libloading::{Library, Symbol};
use std::os::raw::c_int;

type FnModeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: {} <so-path> a b c d", args[0]);
        std::process::exit(2);
    }
    let so = &args[1];
    let a: c_int = args[2].parse().expect("a");
    let b: c_int = args[3].parse().expect("b");
    let c: c_int = args[4].parse().expect("c");
    let d: c_int = args[5].parse().expect("d");

    unsafe {
        let lib = Library::new(so).expect("load lib");
        let f: Symbol<FnModeselect> = lib.get(b"modeselect").expect("get modeselect");
        let v = f(a, b, c, d);
        // Append the return value so callers can verify it without parsing
        // the formatted output.
        println!("__RET__{}", v);
    }
}
