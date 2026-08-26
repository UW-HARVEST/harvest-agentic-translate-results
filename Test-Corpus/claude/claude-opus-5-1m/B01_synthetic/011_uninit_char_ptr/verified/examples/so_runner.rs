//! Hermetic `.so` driver used by the differential test-suite.
//!
//! `so_runner <so-path> <symbol> [payload-hex|--null]`
//!
//! `dlopen()`s the given shared library through `libloading` and calls one of
//! its exported C symbols, inheriting this process' stdin/stdout untouched. It
//! is spawned as a *subprocess* by the tests whenever the called symbol touches
//! stdin (`main`), because libc's `stdin` FILE keeps buffered state that would
//! leak from one in-process call to the next and because the C `main` must see a
//! pristine process just like the executable does.
//!
//! Anything this program itself needs to say goes to stderr, so stdout carries
//! only the bytes the library produced.

use std::ffi::{c_char, c_int, c_void};

extern "C" {
    /// `fflush(NULL)` flushes every open output stream, which is what makes the
    /// C library's buffered `printf` output visible before we exit.
    fn fflush(stream: *mut c_void) -> c_int;
}

fn hex_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    assert!(b.len() % 2 == 0, "odd hex length");
    let val = |c: u8| -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("bad hex digit"),
        }
    };
    b.chunks(2).map(|p| (val(p[0]) << 4) | val(p[1])).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: so_runner <so-path> <symbol> [payload-hex|--null]");
        std::process::exit(64);
    }
    let so_path = &args[1];
    let symbol = &args[2];

    // SAFETY: the test-suite only ever passes the two libraries it just built.
    let lib = unsafe { libloading::Library::new(so_path) };
    let lib = match lib {
        Ok(l) => l,
        Err(e) => {
            eprintln!("dlopen({so_path}) failed: {e}");
            std::process::exit(65);
        }
    };

    let code = unsafe {
        match symbol.as_str() {
            "main" => {
                let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    lib.get(b"main\0").expect("no `main` symbol");
                // An optional repeat count exercises the stream state a C
                // `main` leaves behind in libc's `stdin` FILE buffer.
                let times: usize = args
                    .get(3)
                    .map(|s| s.parse().expect("repeat count"))
                    .unwrap_or(1);
                let mut last = 0;
                for _ in 0..times {
                    last = f();
                }
                last
            }
            "good" | "bad" => {
                let f: libloading::Symbol<unsafe extern "C" fn()> = lib
                    .get(format!("{symbol}\0").as_bytes())
                    .expect("no such symbol");
                f();
                0
            }
            "printLine" => {
                let f: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
                    lib.get(b"printLine\0").expect("no `printLine` symbol");
                match args.get(3).map(String::as_str) {
                    None | Some("--null") => f(std::ptr::null()),
                    Some(hex) => {
                        let mut buf = hex_decode(hex);
                        buf.push(0); // NUL terminator
                        f(buf.as_ptr() as *const c_char);
                    }
                }
                0
            }
            other => {
                eprintln!("unknown symbol `{other}`");
                std::process::exit(66);
            }
        }
    };

    // Flush the C library's stdio buffers, then Rust's, before exiting.
    unsafe { fflush(std::ptr::null_mut()) };
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(code);
}
