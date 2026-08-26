//! `dlopen` probe used by `tests/differential_ffi.rs`.
//!
//! Loads a shared object **by path** with `libloading`, resolves the exported
//! `printLine` symbol and calls it — so the C `.so` and the Rust `.so` are
//! driven through exactly the same external-caller path. The library is never
//! `dlclose`d and the process exits normally, so both sides flush `stdout` at
//! process exit (glibc's own flush for the C build, the `atexit` hook for the
//! Rust build). Nothing else is ever written to stdout.
//!
//! Usage:
//!   ffi_probe <so-path> <op>...
//!
//! where each `<op>` is one of
//!   null                  -> printLine(NULL)
//!   hex:<hex-bytes>       -> printLine("<bytes>")   (may be empty)
//!   rep:<hex-byte>:<n>    -> printLine("<byte> repeated n times")

use std::ffi::c_char;

type PrintLine = unsafe extern "C" fn(*const c_char);

fn parse_hex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "hex payload must have even length: {s:?}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("valid hex"))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: ffi_probe <so-path> <op>...");
        std::process::exit(2);
    }

    let print_line: PrintLine = unsafe {
        let lib = libloading::Library::new(&args[1])
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", args[1]));
        let sym: libloading::Symbol<PrintLine> = lib
            .get(b"printLine\0")
            .unwrap_or_else(|e| panic!("dlsym(printLine) failed: {e}"));
        let f = *sym;
        // Keep the library mapped for the lifetime of the process: no dlclose,
        // so exit-time flushing is the only flush either side performs.
        std::mem::forget(lib);
        f
    };

    for op in &args[2..] {
        if op == "null" {
            unsafe { print_line(std::ptr::null()) };
            continue;
        }
        let payload: Vec<u8> = if let Some(rest) = op.strip_prefix("hex:") {
            parse_hex(rest)
        } else if let Some(rest) = op.strip_prefix("rep:") {
            let (byte, count) = rest.split_once(':').expect("rep:<hexbyte>:<n>");
            let b = u8::from_str_radix(byte, 16).expect("valid hex byte");
            let n: usize = count.parse().expect("valid count");
            vec![b; n]
        } else {
            panic!("unknown op {op:?}");
        };

        assert!(
            !payload.contains(&0),
            "payload must not contain an interior NUL: {payload:?}"
        );

        let mut buf = payload;
        buf.push(0); // NUL terminator
        unsafe { print_line(buf.as_ptr() as *const c_char) };
    }
}
