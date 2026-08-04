// Helper used by integration tests: dlopen a shared library, call a named
// exported symbol, and (optionally) feed stdin / read stdout.
//
// Usage:
//   dlcall <so-path> <symbol> [stdin-text]
//
// Supported symbols:
//   - good: void(void)
//   - bad: void(void)             -- expected to SIGSEGV
//   - printIntPtrLine_null: dereference NULL via printIntPtrLine -- SIGSEGV
//   - printIntPtrLine_42: pass &42 to printIntPtrLine -> prints "42\n"
//   - printIntPtrLine_neg7: pass &-7 -> prints "-7\n"
//   - printIntPtrLine_zero: pass &0 -> prints "0\n"
//   - printIntPtrLine_imax: pass &INT_MAX -> prints "2147483647\n"
//   - printIntPtrLine_imin: pass &INT_MIN -> prints "-2147483648\n"
//   - main: int(void) -- runs full program
//
// Exit code is the symbol's return value (or 0 for void); on a crash the
// process is killed by SIGSEGV which the parent test detects via the
// command's exit status.

use libloading::{Library, Symbol};
use std::os::raw::c_int;

type VoidFn = unsafe extern "C" fn();
type MainFn = unsafe extern "C" fn() -> c_int;
type PrintFn = unsafe extern "C" fn(*const c_int);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: dlcall <so-path> <symbol>");
        std::process::exit(2);
    }
    let so_path = &args[1];
    let sym = &args[2];

    // Load the library. RTLD_NOW behavior; libloading handles it.
    let lib = unsafe { Library::new(so_path) }.expect("dlopen failed");

    match sym.as_str() {
        "good" => unsafe {
            let f: Symbol<VoidFn> = lib.get(b"good").expect("good not found");
            f();
        },
        "bad" => unsafe {
            let f: Symbol<VoidFn> = lib.get(b"bad").expect("bad not found");
            f();
        },
        "main" => unsafe {
            let f: Symbol<MainFn> = lib.get(b"main").expect("main not found");
            let _rc = f();
            // Don't propagate scanf-driven exit code; tests assert stdout.
        },
        "printIntPtrLine_42" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            let v: c_int = 42;
            f(&v as *const c_int);
        },
        "printIntPtrLine_neg7" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            let v: c_int = -7;
            f(&v as *const c_int);
        },
        "printIntPtrLine_zero" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            let v: c_int = 0;
            f(&v as *const c_int);
        },
        "printIntPtrLine_imax" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            let v: c_int = i32::MAX;
            f(&v as *const c_int);
        },
        "printIntPtrLine_imin" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            let v: c_int = i32::MIN;
            f(&v as *const c_int);
        },
        "printIntPtrLine_null" => unsafe {
            let f: Symbol<PrintFn> = lib.get(b"printIntPtrLine").unwrap();
            f(std::ptr::null());
        },
        other => {
            eprintln!("unknown symbol selector: {}", other);
            std::process::exit(3);
        }
    }

    // Make sure stdout is flushed (the C lib uses libc stdout; libloading
    // doesn't drop libc state, but the C lib's stdout buffer may need a
    // final flush before we exit). The libc dlclose at exit will flush.
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Drop the library explicitly so destructors run before process exit.
    drop(lib);
}
