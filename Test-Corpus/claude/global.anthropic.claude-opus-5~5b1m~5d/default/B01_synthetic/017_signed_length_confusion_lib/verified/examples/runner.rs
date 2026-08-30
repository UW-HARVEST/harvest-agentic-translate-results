// Out-of-process differential runner.
//
// Usage: runner <path-to-.so> <op> <arg>
//
//   runner LIB driver <int>          -> loads LIB, calls driver(<int>)
//   runner LIB printLine <hex-bytes> -> loads LIB, calls printLine(bytes || NUL)
//   runner LIB printLineNull -       -> loads LIB, calls printLine(NULL)
//
// Used for inputs whose C behaviour is undefined and terminates the process
// (e.g. `driver(-1)`), so the crash can be observed and compared without
// killing the test harness itself.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static mut stdout: *mut c_void;
}

const IONBF: c_int = 2; // _IONBF on glibc

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: runner <lib> <op> <arg>");
        std::process::exit(2);
    }
    let lib_path = &args[1];
    let op = &args[2];
    let arg = &args[3];

    let lib = unsafe { libloading::Library::new(lib_path) }.expect("failed to load library");

    // `<op>!nobuf` switches libc's stdout to unbuffered mode first.
    let (op, unbuffered) = match op.strip_suffix("!nobuf") {
        Some(base) => (base.to_string(), true),
        None => (op.clone(), false),
    };
    if unbuffered {
        unsafe {
            let s = stdout;
            setvbuf(s, std::ptr::null_mut(), IONBF, 0);
        }
    }

    match op.as_str() {
        "driver" => {
            let data: i32 = arg.parse().expect("bad int");
            let f: libloading::Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { lib.get(b"driver\0") }.expect("no driver symbol");
            unsafe { f(data) };
        }
        "printLine" => {
            let mut buf: Vec<u8> = (0..arg.len() / 2)
                .map(|i| u8::from_str_radix(&arg[2 * i..2 * i + 2], 16).expect("bad hex"))
                .collect();
            buf.push(0);
            let f: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { lib.get(b"printLine\0") }.expect("no printLine symbol");
            unsafe { f(buf.as_ptr() as *const c_char) };
        }
        "printLineNull" => {
            let f: libloading::Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { lib.get(b"printLine\0") }.expect("no printLine symbol");
            unsafe { f(std::ptr::null()) };
        }
        other => {
            eprintln!("unknown op {other}");
            std::process::exit(2);
        }
    }

    // Flush the C runtime's streams (the libraries print through libc `printf`).
    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(0);
}
