//! Helper process used by the differential tests.
//!
//! Usage: `driver_runner <path-to-shared-library> <path-to-input-file>`
//!
//! The input file holds one hexadecimal 64-bit pattern per line. For each one,
//! the corresponding `double` is passed to the library's exported `driver`
//! symbol, which is loaded with `libloading` — i.e. exactly the way any external
//! caller would reach it. Everything `driver` prints goes to this process's
//! stdout, so the parent can capture it without fighting over file descriptor 1.

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::Read;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn fesetround(mode: c_int) -> c_int;
}

/// `LC_ALL` on glibc/Linux.
const LC_ALL: c_int = 6;

fn rounding_mode(name: &str) -> Option<c_int> {
    // `fenv.h` values are target specific.
    #[cfg(target_arch = "x86_64")]
    let table = [
        ("tonearest", 0x0000),
        ("downward", 0x0400),
        ("upward", 0x0800),
        ("towardzero", 0x0c00),
    ];
    #[cfg(target_arch = "aarch64")]
    let table = [
        ("tonearest", 0x0000_0000),
        ("upward", 0x0040_0000),
        ("downward", 0x0080_0000),
        ("towardzero", 0x00c0_0000),
    ];
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let table: [(&str, c_int); 0] = [];

    table
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v as c_int)
}

type DriverFn = unsafe extern "C" fn(f64);

fn main() {
    let mut args = std::env::args_os().skip(1);
    let lib_path = args.next().expect("usage: driver_runner <lib.so> <inputs>");
    let input_path = args.next().expect("usage: driver_runner <lib.so> <inputs>");

    let mut text = String::new();
    std::fs::File::open(&input_path)
        .expect("open input file")
        .read_to_string(&mut text)
        .expect("read input file");

    // Optional ambient state, applied identically before either library runs:
    // `printf`'s `%a`/`%f` conversions depend on `LC_NUMERIC` and on the current
    // floating-point rounding direction, so the harness can exercise both.
    unsafe {
        if let Ok(loc) = std::env::var("DRIVER_LOCALE") {
            let c = CString::new(loc.clone()).expect("locale name");
            let got = setlocale(LC_ALL, c.as_ptr());
            if got.is_null() {
                eprintln!("driver_runner: setlocale({loc}) failed");
                std::process::exit(2);
            }
        }
        if let Ok(mode) = std::env::var("DRIVER_ROUNDING") {
            let Some(v) = rounding_mode(&mode) else {
                eprintln!("driver_runner: unknown/unsupported rounding mode {mode}");
                std::process::exit(3);
            };
            if fesetround(v) != 0 {
                eprintln!("driver_runner: fesetround({mode}) failed");
                std::process::exit(4);
            }
        }
    }

    unsafe {
        let lib = libloading::Library::new(&lib_path).expect("dlopen library");
        let sym: libloading::Symbol<DriverFn> = lib.get(b"driver\0").expect("`driver` symbol");
        let driver: DriverFn = *sym;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let bits = u64::from_str_radix(line, 16).expect("hex bit pattern");
            driver(f64::from_bits(bits));
        }

        // Make sure glibc's stdout buffer reaches the pipe before we exit.
        fflush(std::ptr::null_mut());
    }
}
