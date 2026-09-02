//! Out-of-process probe used by the Phase C undefined-behaviour tests.
//!
//! Usage: `probe <path-to-.so> <x>`
//!
//! `dlopen`s the given shared object, calls its `pow43` export with `x`, and
//! prints the raw result bits. Exits 0 on success. If the call performs an
//! out-of-bounds read into an unmapped page the process dies by signal, which
//! the parent test observes instead of taking the whole test binary down.
//!
//! A Rust bounds-check panic is reported distinctly (exit code 101 / "panicked"
//! on stderr) so the parent can tell "UB read, same as C" apart from
//! "Rust rejected an input C accepted".

use libloading::{Library, Symbol};

type Pow43Fn = unsafe extern "C" fn(std::ffi::c_int) -> f32;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: probe <so-path> <x>");
        std::process::exit(2);
    }
    let so = &args[1];
    let x: i32 = args[2].parse().expect("x must be an i32");

    unsafe {
        let lib = Library::new(so).expect("dlopen failed");
        let f: Symbol<Pow43Fn> = lib.get(b"pow43\0").expect("no `pow43` export");
        let v = f(x);
        println!("{:08x}", v.to_bits());
    }
    std::process::exit(0);
}
